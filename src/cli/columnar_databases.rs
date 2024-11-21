use crate::cli::analytics_common::{read_analytics_response, send_analytics_query};
use crate::cli::generic_error;
use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::state::State;
use log::debug;
use nu_engine::command_prelude::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoPipelineData, PipelineData, ShellError, Signature, SyntaxShape, Value,
};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

#[derive(Clone)]
pub struct ColumnarDatabases {
    state: Arc<Mutex<State>>,
}

impl ColumnarDatabases {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for ColumnarDatabases {
    fn name(&self) -> &str {
        "columnar databases"
    }

    fn signature(&self) -> Signature {
        Signature::build("columnar databases")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Lists all databases on a Columnar analytics cluster"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        columnar_databases(self.state.clone(), engine_state, stack, call, input)
    }
}

fn columnar_databases(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let signals = engine_state.signals().clone();
    let statement = "SELECT `Database`.* FROM `Metadata`.`Database`";
    let span = call.head;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;

    let guard = state.lock().unwrap();
    debug!("Running analytics query {}", &statement);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let resp = send_analytics_query(
            active_cluster,
            None,
            statement,
            signals.clone(),
            span,
            Arc::new(Runtime::new().unwrap()),
        )?;

        results.extend(
            read_analytics_response(identifier.clone(), resp, span, false, false).map_err(|e| {
                if e.to_string().contains("No nodes found for service")
                    || format!("{:?}", e).contains("Cannot find analytics collection Database")
                {
                    cluster_not_columnar(identifier)
                } else {
                    e
                }
            })?,
        );

        // Handle collection Databases not found here
    }

    Ok(Value::List {
        vals: results,
        internal_span: span,
    }
    .into_pipeline_data())
}

fn cluster_not_columnar(identifier: String) -> ShellError {
    generic_error(
        format!("{} not a Columnar cluster", identifier),
        "columnar commnands are only supported against Columnar clusters".to_string(),
        None,
    )
}
