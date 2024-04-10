use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::state::State;
use log::debug;
use std::sync::{Arc, Mutex};

use crate::cli::query::{handle_query_response, query_context_from_args, send_query};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct QueryAdvise {
    state: Arc<Mutex<State>>,
}

impl QueryAdvise {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for QueryAdvise {
    fn name(&self) -> &str {
        "query advise"
    }

    fn signature(&self) -> Signature {
        Signature::build("query advise")
            .required("statement", SyntaxShape::String, "the query statement")
            .switch("with-meta", "Includes related metadata in the result", None)
            .switch("disable-context", "disable automatically detecting the query context based on the active bucket and scope", None)
            .named(
                "databases",
                SyntaxShape::String,
                "the databases to query against",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Calls the query adviser and lists recommended indexes"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let statement: String = call.req(engine_state, stack, 0)?;
    let statement = format!("ADVISE {}", statement);

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let maybe_scope = query_context_from_args(active_cluster, engine_state, stack, call)?;

        debug!("Running n1ql advise query {}", &statement);

        let response = send_query(
            active_cluster,
            statement.clone(),
            None,
            maybe_scope,
            ctrl_c.clone(),
            None,
            span,
            None,
        )?;
        drop(guard);

        results.extend(handle_query_response(
            call.has_flag(engine_state, stack, "with-meta")?,
            identifier.clone(),
            response,
            span,
        )?);
    }

    Ok(Value::List {
        vals: results,
        internal_span: call.head,
    }
    .into_pipeline_data())
}
