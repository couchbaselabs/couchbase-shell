//! The `collections drop` commanddrop a collection from the server.

use crate::cli::util::{cluster_identifiers_from, validate_is_not_cloud};
use crate::client::ManagementRequest::DropCollection;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};

use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct CollectionsDrop {
    state: Arc<Mutex<State>>,
}

impl CollectionsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for CollectionsDrop {
    fn name(&self) -> &str {
        "collections drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("collections drop")
            .required("name", SyntaxShape::String, "the name of the collection")
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Deletes collections through the HTTP API"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        collections_drop(self.state.clone(), args)
    }
}

fn collections_drop(
    state: Arc<Mutex<State>>,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();
    let collection: String = args.req(0)?;

    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };
        validate_is_not_cloud(
            active_cluster,
            "collections drop cannot be run against cloud clusters",
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

        let scope_name = match args.get_flag("scope")? {
            Some(name) => name,
            None => match active_cluster.active_scope() {
                Some(s) => s,
                None => {
                    return Err(ShellError::unexpected(
                        "Could not auto-select a scope - please use --scope instead".to_string(),
                    ));
                }
            },
        };

        debug!(
            "Running collections drop for {:?} on bucket {:?}, scope {:?}",
            &collection, &bucket, &scope_name
        );

        let response = active_cluster.cluster().http_client().management_request(
            DropCollection {
                scope: scope_name,
                bucket,
                name: collection.clone(),
            },
            Instant::now().add(active_cluster.timeouts().management_timeout()),
            ctrl_c.clone(),
        )?;

        match response.status() {
            200 => {}
            _ => {
                return Err(ShellError::unexpected(response.content()));
            }
        }
    }
    Ok(OutputStream::empty())
}
