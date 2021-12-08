use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

pub struct UseCmd {
    state: Arc<Mutex<State>>,
}

impl UseCmd {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for UseCmd {
    fn name(&self) -> &str {
        "use"
    }

    fn signature(&self) -> Signature {
        Signature::build("use")
            .switch("cloud", "show default execution environment of cloud", None)
            .switch(
                "timeouts",
                "show default execution environment for timeouts",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Modify the default execution environment of commands"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        use_cmd(args, self.state.clone())
    }
}

fn use_cmd(args: CommandArgs, state: Arc<Mutex<State>>) -> Result<OutputStream, ShellError> {
    let show_cloud = args.has_flag("cloud");
    let show_timeouts = args.has_flag("timeouts");

    let guard = state.lock().unwrap();
    let active = guard.active_cluster();
    let mut using_now = TaggedDictBuilder::new(Tag::default());
    if show_cloud {
        let project = match guard.active_cloud() {
            Ok(c) => c.active_project().unwrap_or_else(|| "".to_string()),
            Err(_e) => "".to_string(),
        };

        using_now.insert_value(
            "cloud_organization",
            guard
                .active_cloud_org_name()
                .unwrap_or_else(|| String::from("")),
        );
        using_now.insert_value(
            "cloud",
            guard
                .active_cloud_name()
                .unwrap_or_else(|| String::from("")),
        );
        using_now.insert_value("project", project);
    } else {
        using_now.insert_value("username", active.username());
        using_now.insert_value("cluster", guard.active());
        using_now.insert_value(
            "bucket",
            active
                .active_bucket()
                .unwrap_or_else(|| String::from("<not set>")),
        );
        using_now.insert_value(
            "scope",
            active.active_scope().unwrap_or_else(|| String::from("")),
        );
        using_now.insert_value(
            "collection",
            active
                .active_collection()
                .unwrap_or_else(|| String::from("")),
        );
        if let Some(co) = active.cloud_org() {
            using_now.insert_value("cloud_organization", co);
        }
    }
    if show_timeouts {
        let timeouts = active.timeouts();
        using_now.insert_value(
            "data_timeout (ms)",
            UntaggedValue::int(timeouts.data_timeout().as_millis() as i64),
        );
        using_now.insert_value(
            "management_timeout (ms)",
            UntaggedValue::int(timeouts.management_timeout().as_millis() as i64),
        );
        using_now.insert_value(
            "analytics_timeout (ms)",
            UntaggedValue::int(timeouts.analytics_timeout().as_millis() as i64),
        );
        using_now.insert_value(
            "query_timeout (ms)",
            UntaggedValue::int(timeouts.query_timeout().as_millis() as i64),
        );
        using_now.insert_value(
            "search_timeout (ms)",
            UntaggedValue::int(timeouts.search_timeout().as_millis() as i64),
        );
    }
    let clusters = vec![using_now.into_value()];

    Ok(clusters.into())
}
