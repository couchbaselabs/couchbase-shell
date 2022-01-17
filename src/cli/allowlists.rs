use crate::cli::cloud_json::JSONCloudGetAllowListResponse;
use crate::cli::util::{cluster_identifiers_from, validate_is_cloud};
use crate::client::CapellaRequest;
use crate::state::{CapellaEnvironment, State};
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct AllowLists {
    state: Arc<Mutex<State>>,
}

impl AllowLists {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for AllowLists {
    fn name(&self) -> &str {
        "allowlists"
    }

    fn signature(&self) -> Signature {
        Signature::build("allowlists").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Displays allow list for Capella cluster access"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        addresses(self.state.clone(), args)
    }
}

fn addresses(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    debug!("Running allowlists");

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();
    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };
        validate_is_cloud(
            active_cluster,
            "allowlists can only be used with clusters registered to a Capella organisation",
        )?;

        let cloud = guard
            .capella_org_for_cluster(active_cluster.capella_org().unwrap())?
            .client();
        let cluster = cloud.find_cluster(
            identifier.clone(),
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        if cluster.environment() == CapellaEnvironment::Hosted {
            return Err(ShellError::unexpected(
                "allowlists cannot be run against hosted Capella clusters",
            ));
        }

        let response = cloud.capella_request(
            CapellaRequest::GetAllowList {
                cluster_id: cluster.id(),
            },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;
        if response.status() != 200 {
            return Err(ShellError::untagged_runtime_error(
                response.content().to_string(),
            ));
        };

        let content: Vec<JSONCloudGetAllowListResponse> = serde_json::from_str(response.content())?;

        let mut entries = content
            .into_iter()
            .map(|entry| {
                let mut collected = TaggedDictBuilder::new(Tag::default());
                collected.insert_value("address", entry.address());
                collected.insert_value("type", entry.rule_type());
                collected.insert_value("state", entry.state());
                collected.insert_value(
                    "duration",
                    entry.duration().unwrap_or_else(|| "-".to_string()),
                );
                collected.insert_value("created", entry.created_at());
                collected.insert_value("updated", entry.updated_at());
                collected.into_value()
            })
            .collect();

        results.append(&mut entries);
    }
    Ok(OutputStream::from(results))
}
