use crate::cli::collections::Manifest;
use crate::cli::util::{cluster_identifiers_from, validate_is_not_cloud};
use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Scopes {
    state: Arc<Mutex<State>>,
}

impl Scopes {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Scopes {
    fn name(&self) -> &str {
        "scopes"
    }

    fn signature(&self) -> Signature {
        Signature::build("scopes")
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Fetches scopes through the HTTP API"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        scopes_get(self.state.clone(), args)
    }
}

fn scopes_get(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let guard = state.lock().unwrap();

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };
        validate_is_not_cloud(
            active_cluster,
            "scopes get cannot be run against cloud clusters",
        )?;

        let bucket = match args.get_flag("bucket")? {
            Some(v) => v,
            None => match active_cluster.active_bucket() {
                Some(s) => s,
                None => {
                    return Err(ShellError::unexpected(
                        "Could not auto-select a bucket - please use --bucket instead".to_string(),
                    ));
                }
            },
        };

        debug!("Running scopes get for bucket {:?}", &bucket);

        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetScopes { bucket },
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        let manifest: Manifest = match response.status() {
            200 => match serde_json::from_str(response.content()) {
                Ok(m) => m,
                Err(e) => {
                    return Err(ShellError::unexpected(format!(
                        "Failed to decode response body {}",
                        e,
                    )));
                }
            },
            _ => {
                return Err(ShellError::unexpected(response.content()));
            }
        };

        for scope in manifest.scopes {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("scope", scope.name);
            collected.insert_value("cluster", identifier.clone());
            results.push(collected.into_value());
        }
    }

    Ok(OutputStream::from(results))
}
