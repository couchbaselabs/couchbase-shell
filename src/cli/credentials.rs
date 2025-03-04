use crate::cli::client_error_to_shell_error;
use crate::cli::util::{
    cluster_from_conn_str, cluster_identifiers_from, find_org_id, find_project_id,
    get_active_cluster, NuValueMap,
};
use crate::state::State;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{
    Category, IntoInterruptiblePipelineData, PipelineData, Record, ShellError, Signature,
    SyntaxShape, Value,
};
use nu_utils::SharedCow;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Credentials {
    state: Arc<Mutex<State>>,
}

impl Credentials {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for Credentials {
    fn name(&self) -> &str {
        "credentials"
    }

    fn signature(&self) -> Signature {
        Signature::build("credentials")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Lists existing credentials on a Capella cluster"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        credentials(self.state.clone(), engine_state, stack, call, input)
    }
}

fn credentials(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let org = guard.named_or_active_org(cluster.capella_org())?;

        let client = org.client();

        let org_id = find_org_id(signals.clone(), &client, span)?;

        let project_id = find_project_id(
            signals.clone(),
            guard.active_project().unwrap(),
            &client,
            span,
            org_id.clone(),
        )?;

        let json_cluster = cluster_from_conn_str(
            identifier.clone(),
            signals.clone(),
            cluster.hostnames().clone(),
            &client,
            span,
            org_id.clone(),
            project_id.clone(),
        )?;

        let credentials = client
            .list_credentials(org_id, project_id, json_cluster.id(), signals.clone())
            .map_err(|e| client_error_to_shell_error(e, span))?;

        for creds in credentials.data() {
            let mut collected = NuValueMap::default();
            collected.add_string("id", creds.id(), span);
            collected.add_string("name", creds.name(), span);
            collected.add_string("cluster", identifier.clone(), span);

            let mut access_records = vec![];
            for acc in creds.access() {
                let cols = vec![
                    "bucket".to_string(),
                    "scopes".to_string(),
                    "privileges".to_string(),
                ];
                let mut vals = vec![];

                vals.push(Value::String {
                    val: acc.bucket(),
                    internal_span: span,
                });

                let mut scope_values = vec![];
                for scope in acc.scopes() {
                    scope_values.push(Value::String {
                        val: scope,
                        internal_span: span,
                    })
                }

                vals.push(Value::List {
                    vals: scope_values,
                    internal_span: span,
                });

                let mut privilege_values = vec![];
                for privilege in acc.privileges() {
                    privilege_values.push(Value::String {
                        val: privilege,
                        internal_span: span,
                    })
                }

                vals.push(Value::List {
                    vals: privilege_values,
                    internal_span: span,
                });

                let access = Record::from_raw_cols_vals(cols, vals, span, span).unwrap();
                access_records.push(Value::Record {
                    val: SharedCow::new(access),
                    internal_span: span,
                });
            }

            collected.add_vec("access", access_records, span);
            results.push(collected.into_value(span))
        }
    }

    Ok(results.into_pipeline_data(span, signals))
}
