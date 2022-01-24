use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct UseTimeouts {
    state: Arc<Mutex<State>>,
}

impl UseTimeouts {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseTimeouts {
    fn name(&self) -> &str {
        "use timeouts"
    }

    fn signature(&self) -> Signature {
        Signature::build("use timeouts")
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
    }

    fn usage(&self) -> &str {
        "Sets the active timeouts for operations"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let guard = self.state.lock().unwrap();
        let active = match guard.active_cluster() {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("An active cluster must be set"));
            }
        };

        let analytics = args.get_flag("analytics-timeout")?;
        let search = args.get_flag("search-timeout")?;
        let query = args.get_flag("query-timeout")?;
        let data = args.get_flag("data-timeout")?;
        let management = args.get_flag("management-timeout")?;

        let mut timeouts = active.timeouts();

        if let Some(t) = analytics {
            timeouts.set_analytics_timeout(Duration::from_millis(t));
        };
        if let Some(t) = search {
            timeouts.set_search_timeout(Duration::from_millis(t));
        };
        if let Some(t) = query {
            timeouts.set_query_timeout(Duration::from_millis(t));
        };
        if let Some(t) = data {
            timeouts.set_data_timeout(Duration::from_millis(t));
        };
        if let Some(t) = management {
            timeouts.set_management_timeout(Duration::from_millis(t));
        };

        active.set_timeouts(timeouts);

        let new_timeouts = active.timeouts();
        let mut using_now = TaggedDictBuilder::new(Tag::default());
        using_now.insert_value(
            "data_timeout (ms)",
            UntaggedValue::int(new_timeouts.data_timeout().as_millis() as i64),
        );
        using_now.insert_value(
            "management_timeout (ms)",
            UntaggedValue::int(new_timeouts.management_timeout().as_millis() as i64),
        );
        using_now.insert_value(
            "analytics_timeout (ms)",
            UntaggedValue::int(new_timeouts.analytics_timeout().as_millis() as i64),
        );
        using_now.insert_value(
            "query_timeout (ms)",
            UntaggedValue::int(new_timeouts.query_timeout().as_millis() as i64),
        );
        using_now.insert_value(
            "search_timeout (ms)",
            UntaggedValue::int(new_timeouts.search_timeout().as_millis() as i64),
        );
        let clusters = vec![using_now.into_value()];
        Ok(clusters.into())
    }
}
