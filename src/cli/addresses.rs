use crate::cli::cloud_json::JSONCloudGetAllowListResponse;
use crate::cli::util::{cluster_identifiers_from, validate_is_cloud};
use crate::client::CloudRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_cli::TaggedDictBuilder;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Addresses {
    state: Arc<Mutex<State>>,
}

impl Addresses {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Addresses {
    fn name(&self) -> &str {
        "addresses"
    }

    fn signature(&self) -> Signature {
        Signature::build("addresses").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "List all allowed addresses for cloud cluster access"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        addresses(self.state.clone(), args)
    }
}

fn addresses(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    debug!("Running addresses");

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
            "addresses can only be used with clusters registered to a cloud control pane",
        )?;

        let cloud = guard
            .cloud_for_cluster(active_cluster.cloud().unwrap())?
            .cloud();
        let cluster_id = cloud.find_cluster_id(
            identifier.clone(),
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        let response = cloud.cloud_request(
            CloudRequest::GetAllowList { cluster_id },
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
