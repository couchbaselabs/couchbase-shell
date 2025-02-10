use crate::cli::util::NuValueMap;
use crate::state::State;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct UseCmd {
    state: Arc<Mutex<State>>,
}

impl UseCmd {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UseCmd {
    fn name(&self) -> &str {
        "cb-env"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env")
            .switch(
                "capella",
                "show default execution environment of capella",
                None,
            )
            .switch(
                "timeouts",
                "show default execution environment for timeouts",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Modify the default execution environment of commands"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        use_cmd(self.state.clone(), engine_state, stack, call, input)
    }
}

fn use_cmd(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let show_capella = call.has_flag(engine_state, stack, "capella")?;
    let show_timeouts = call.has_flag(engine_state, stack, "timeouts")?;

    let span = call.head;

    let guard = state.lock().unwrap();
    let mut results = NuValueMap::default();
    if show_capella {
        let org = guard.active_capella_org()?;

        results.add_string(
            "capella-organization",
            guard
                .active_capella_org_name()
                .unwrap_or_else(|| String::from("")),
            span,
        );

        results.add_string(
            "capella-project",
            guard.active_project().unwrap_or_else(|_| String::from("")),
            span,
        );

        if show_timeouts {
            results.add_i64(
                "management-timeout (ms)",
                org.timeout().as_millis() as i64,
                span,
            );
        }
    } else {
        match guard.active_cluster() {
            Some(active) => {
                let display_name = if let Some(dn) = active.display_name() {
                    dn
                } else {
                    active.username().to_string()
                };
                results.add_string("username", active.username(), span);
                results.add_string("display_name", display_name, span);
                results.add_string("cluster", guard.active(), span);
                results.add_string(
                    "bucket",
                    active.active_bucket().unwrap_or_else(|| String::from("")),
                    span,
                );
                results.add_string(
                    "scope",
                    active
                        .active_scope()
                        .unwrap_or_else(|| String::from("_default")),
                    span,
                );
                results.add_string(
                    "collection",
                    active
                        .active_collection()
                        .unwrap_or_else(|| String::from("_default")),
                    span,
                );
                results.add_bool("is capella", active.is_capella(), span);
                if let Some(co) = active.capella_org() {
                    results.add_string("capella-organization", co, span);
                }

                if show_timeouts {
                    let timeouts = active.timeouts();
                    results.add_i64(
                        "data-timeout (ms)",
                        timeouts.data_timeout().as_millis() as i64,
                        span,
                    );
                    results.add_i64(
                        "management-timeout (ms)",
                        timeouts.management_timeout().as_millis() as i64,
                        span,
                    );
                    results.add_i64(
                        "analytics-timeout (ms)",
                        timeouts.analytics_timeout().as_millis() as i64,
                        span,
                    );
                    results.add_i64(
                        "query-timeout (ms)",
                        timeouts.query_timeout().as_millis() as i64,
                        span,
                    );
                    results.add_i64(
                        "search-timeout (ms)",
                        timeouts.search_timeout().as_millis() as i64,
                        span,
                    );
                }
            }
            None => {
                results.add_string("username", String::from(""), span);
                results.add_string("display_name", String::from(""), span);
                results.add_string("cluster", String::from(""), span);
                results.add_string("bucket", String::from(""), span);
                results.add_string("scope", String::from(""), span);
                results.add_string("collection", String::from(""), span);
                results.add_string("cluster_type", String::from(""), span);
            }
        }
    }

    if let Some(llm_id) = guard.active_llm_id() {
        results.add_string("llm", llm_id, span);
    }

    Ok(results.into_pipeline_data(call.head))
}
