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
    }

    fn usage(&self) -> &str {
        "Modify the default execution environment of commands"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        use_cmd(args, self.state.clone())
    }
}

fn use_cmd(args: CommandArgs, state: Arc<Mutex<State>>) -> Result<OutputStream, ShellError> {
    let show_capella = args.has_flag("capella");
    let show_timeouts = args.has_flag("timeouts");

    let guard = state.lock().unwrap();
    let mut using_now = TaggedDictBuilder::new(Tag::default());
    if show_capella {
        let org = guard.active_capella_org()?;

        using_now.insert_value(
            "capella-organization",
            guard
                .active_capella_org_name()
                .unwrap_or_else(|| String::from("")),
        );
        using_now.insert_value(
            "cloud",
            org.active_cloud().unwrap_or_else(|| String::from("")),
        );
        using_now.insert_value(
            "project",
            org.active_project().unwrap_or_else(|| String::from("")),
        );
        if show_timeouts {
            using_now.insert_value(
                "management-timeout (ms)",
                UntaggedValue::int(org.timeout().as_millis() as i64),
            );
        }
    } else {
        match guard.active_cluster() {
            Some(active) => {
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
                if let Some(co) = active.capella_org() {
                    using_now.insert_value("capella-organization", co);
                }

                if show_timeouts {
                    let timeouts = active.timeouts();
                    using_now.insert_value(
                        "data-timeout (ms)",
                        UntaggedValue::int(timeouts.data_timeout().as_millis() as i64),
                    );
                    using_now.insert_value(
                        "management-timeout (ms)",
                        UntaggedValue::int(timeouts.management_timeout().as_millis() as i64),
                    );
                    using_now.insert_value(
                        "analytics-timeout (ms)",
                        UntaggedValue::int(timeouts.analytics_timeout().as_millis() as i64),
                    );
                    using_now.insert_value(
                        "query-timeout (ms)",
                        UntaggedValue::int(timeouts.query_timeout().as_millis() as i64),
                    );
                    using_now.insert_value(
                        "search-timeout (ms)",
                        UntaggedValue::int(timeouts.search_timeout().as_millis() as i64),
                    );
                }
            }
            None => {
                using_now.insert_value("username", String::from("<not set>"));
                using_now.insert_value("cluster", String::from("<not set>"));
                using_now.insert_value("bucket", String::from("<not set>"));
                using_now.insert_value("scope", String::from("<not set>"));
                using_now.insert_value("collection", String::from("<not set>"));
            }
        }
    }
    let clusters = vec![using_now.into_value()];

    Ok(clusters.into())
}
