//! The `collections drop` commanddrop a collection from the server.

use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::client::ManagementRequest::DropCollection;
use crate::state::State;
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::collections::get_bucket_or_active;
use crate::cli::error::{
    client_error_to_shell_error, no_active_scope_error, unexpected_status_code_error,
};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
use nu_protocol::Value::Nothing;

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
            .category(Category::Custom("couchbase".to_string()))
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

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();
    let collection: String = call.req(engine_state, stack, 0)?;

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let bucket = get_bucket_or_active(active_cluster, engine_state, stack, call)?;

        let scope_name = match call.get_flag(engine_state, stack, "scope")? {
            Some(name) => name,
            None => match active_cluster.active_scope() {
                Some(s) => s,
                None => {
                    return Err(no_active_scope_error(span));
                }
            },
        };

        debug!(
            "Running collections drop for {:?} on bucket {:?}, scope {:?}",
            &collection, &bucket, &scope_name
        );

        let response = active_cluster
            .cluster()
            .http_client()
            .management_request(
                DropCollection {
                    scope: scope_name,
                    bucket,
                    name: collection.clone(),
                },
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        match response.status() {
            200 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
                ));
            }
        }
    }

    Ok(PipelineData::Value(Nothing {internal_span: span}, None))
}
