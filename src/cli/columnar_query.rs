use crate::cli::analytics::do_analytics_query;
use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::state::State;
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ColumnarQuery {
    state: Arc<Mutex<State>>,
}

impl ColumnarQuery {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ColumnarQuery {
    fn name(&self) -> &str {
        "columnar query"
    }

    fn signature(&self) -> Signature {
        Signature::build("columnar query")
            .required("statement", SyntaxShape::String, "the query statement")
            .named(
                "database",
                SyntaxShape::String,
                "the database to query against",
                None,
            )
            .named(
                "scope",
                SyntaxShape::String,
                "the scope to query against",
                None,
            )
            .switch("with-meta", "Includes related metadata in the result", None)
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Performs a query against a Columnar analytics cluster"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        columnar_query(self.state.clone(), engine_state, stack, call, input)
    }
}

fn columnar_query(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let statement: String = call.req(engine_state, stack, 0)?;
    let span = call.head;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();

    let scope: Option<String> = call.get_flag(engine_state, stack, "scope")?;
    let with_meta = call.has_flag(engine_state, stack, "with-meta")?;

    debug!("Running Columnar analytics query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let database = call
            .get_flag(engine_state, stack, "database")?
            .or_else(|| active_cluster.active_bucket());
        let maybe_scope = database.and_then(|d| scope.clone().map(|s| (d, s)));

        results.extend(do_analytics_query(
            identifier.clone(),
            active_cluster,
            maybe_scope,
            &statement,
            ctrl_c.clone(),
            span,
            with_meta,
            true,
        )?);
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}
