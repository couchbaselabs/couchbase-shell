use crate::cli::cloud_json::JSONCloudsResponse;

use crate::client::CloudRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, TaggedDictBuilder};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct CloudsStatus {
    state: Arc<Mutex<State>>,
}

impl CloudsStatus {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for CloudsStatus {
    fn name(&self) -> &str {
        "clouds status"
    }

    fn signature(&self) -> Signature {
        Signature::build("clouds status").switch(
            "all",
            "Show status for all clouds, not only locally managed clouds",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Shows the current status for the managed clouds, optionally showing for all clouds"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clouds_status(self.state.clone(), args)
    }
}

fn clouds_status(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let all = args.get_flag("all")?.unwrap_or(false);

    debug!("Running clouds status");

    let guard = state.lock().unwrap();
    let control = guard.active_cloud_org()?;
    let client = control.client();
    let response = client.cloud_request(
        CloudRequest::GetClouds {},
        Instant::now().add(control.timeout()),
        ctrl_c,
    )?;
    if response.status() != 200 {
        return Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        ));
    };

    let content: JSONCloudsResponse = serde_json::from_str(response.content())?;

    let mut results = vec![];
    if all {
        let unknown_clouds = content
            .items()
            .into_iter()
            .filter(|c| !guard.clouds().contains_key(c.name()));

        for cloud in unknown_clouds {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("identifier", cloud.name());
            collected.insert_value("status", cloud.status());
            collected.insert_value("region", cloud.region());
            collected.insert_value("provider", cloud.provider());
            collected.insert_value("cloud_id", cloud.id());
            collected.insert_value("managed", false);
            collected.insert_value("not_found", false);
            results.push(collected.into_value())
        }
    }

    for cloud in guard.clouds() {
        let mut remote_cloud = None;
        for content_cloud in content.items() {
            if content_cloud.name() == cloud.0 {
                remote_cloud = Some(content_cloud);
            }
        }
        if let Some(c) = remote_cloud {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("identifier", cloud.0.clone());
            collected.insert_value("status", c.status());
            collected.insert_value("region", c.region());
            collected.insert_value("provider", c.provider());
            collected.insert_value("cloud_id", c.id());
            collected.insert_value("managed", true);
            collected.insert_value("not_found", false);
            results.push(collected.into_value())
        } else {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("identifier", cloud.0.clone());
            collected.insert_value("status", "");
            collected.insert_value("region", "");
            collected.insert_value("provider", "");
            collected.insert_value("cloud_id", "");
            collected.insert_value("managed", true);
            collected.insert_value("not_found", true);
            results.push(collected.into_value())
        }
    }

    Ok(OutputStream::from(results))
}
