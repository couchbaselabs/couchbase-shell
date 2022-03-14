//! The `collections drop` commanddrop a collection from the server.

use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, generic_labeled_error, validate_is_not_cloud,
};
use crate::client::ManagementRequest::DropCollection;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct CollectionsDrop {
    state: Arc<Mutex<State>>,
}

impl CollectionsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for CollectionsDrop {
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
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Deletes collections through the HTTP API"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        collections_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn collections_drop(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let guard = state.lock().unwrap();
    let collection: String = call.req(engine_state, stack, 0)?;

    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier));
            }
        };
        validate_is_not_cloud(
            active_cluster,
            "collections drop cannot be run against cloud clusters",
        )?;

        let bucket = match call.get_flag(engine_state, stack, "bucket")? {
            Some(v) => v,
            None => match active_cluster.active_bucket() {
                Some(s) => s,
                None => {
                    return Err(ShellError::MissingParameter(
                        "Could not auto-select a bucket - please use --bucket instead".to_string(),
                        span,
                    ));
                }
            },
        };

        let scope_name = match call.get_flag(engine_state, stack, "scope")? {
            Some(name) => name,
            None => match active_cluster.active_scope() {
                Some(s) => s,
                None => {
                    return Err(ShellError::MissingParameter(
                        "Could not auto-select a scope - please use --scope instead".to_string(),
                        span,
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
                return Err(generic_labeled_error(
                    "Failed to drop collection",
                    format!("Failed to drop collection {}", response.content()),
                ));
            }
        }
    }

    Ok(PipelineData::new(span))
}
