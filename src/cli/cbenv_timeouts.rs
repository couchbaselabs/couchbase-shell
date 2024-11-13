use crate::cli::error::no_active_cluster_error;
use crate::cli::util::NuValueMap;
use crate::state::State;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone)]
pub struct UseTimeouts {
    state: Arc<Mutex<State>>,
}

impl UseTimeouts {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UseTimeouts {
    fn name(&self) -> &str {
        "cb-env timeouts"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env timeouts")
            .named(
                "analytics-timeout",
                SyntaxShape::Int,
                "the new timeout for analytics queries (in ms)",
                None,
            )
            .named(
                "query-timeout",
                SyntaxShape::Int,
                "the new timeout for queries (in ms)",
                None,
            )
            .named(
                "search-timeout",
                SyntaxShape::Int,
                "the new timeout for search queries (in ms)",
                None,
            )
            .named(
                "data-timeout",
                SyntaxShape::Int,
                "the new timeout for data operations (in ms)",
                None,
            )
            .named(
                "management-timeout",
                SyntaxShape::Int,
                "the new timeout for management operations (in ms)",
                None,
            )
            .named(
                "transaction-timeout",
                SyntaxShape::Int,
                "the new timeout for transactions operations (in ms)",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Sets the active timeouts for operations"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let guard = self.state.lock().unwrap();
        let active = match guard.active_cluster() {
            Some(c) => c,
            None => {
                return Err(no_active_cluster_error(call.head));
            }
        };

        let analytics: Option<i64> = call.get_flag(engine_state, stack, "analytics-timeout")?;
        let search: Option<i64> = call.get_flag(engine_state, stack, "search-timeout")?;
        let query: Option<i64> = call.get_flag(engine_state, stack, "query-timeout")?;
        let data: Option<i64> = call.get_flag(engine_state, stack, "data-timeout")?;
        let management: Option<i64> = call.get_flag(engine_state, stack, "management-timeout")?;
        let transaction: Option<i64> = call.get_flag(engine_state, stack, "transaction-timeout")?;

        let mut timeouts = active.timeouts();

        if let Some(t) = analytics {
            timeouts.set_analytics_timeout(Duration::from_millis(t as u64));
        };
        if let Some(t) = search {
            timeouts.set_search_timeout(Duration::from_millis(t as u64));
        };
        if let Some(t) = query {
            timeouts.set_query_timeout(Duration::from_millis(t as u64));
        };
        if let Some(t) = data {
            timeouts.set_data_timeout(Duration::from_millis(t as u64));
        };
        if let Some(t) = management {
            timeouts.set_management_timeout(Duration::from_millis(t as u64));
        };
        if let Some(t) = transaction {
            timeouts.set_transaction_timeout(Duration::from_millis(t as u64));
        };

        active.set_timeouts(timeouts);

        let new_timeouts = active.timeouts();
        let mut using_now = NuValueMap::default();

        using_now.add_i64(
            "data_timeout (ms)",
            new_timeouts.data_timeout().as_millis() as i64,
            call.head,
        );
        using_now.add_i64(
            "management_timeout (ms)",
            new_timeouts.management_timeout().as_millis() as i64,
            call.head,
        );
        using_now.add_i64(
            "analytics_timeout (ms)",
            new_timeouts.analytics_timeout().as_millis() as i64,
            call.head,
        );
        using_now.add_i64(
            "query_timeout (ms)",
            new_timeouts.query_timeout().as_millis() as i64,
            call.head,
        );
        using_now.add_i64(
            "search_timeout (ms)",
            new_timeouts.search_timeout().as_millis() as i64,
            call.head,
        );
        using_now.add_i64(
            "transaction_timeout (ms)",
            new_timeouts.transaction_timeout().as_millis() as i64,
            call.head,
        );

        Ok(using_now.into_pipeline_data(call.head))
    }
}
