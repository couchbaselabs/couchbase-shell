use crate::cli::convert_cb_error;
use crate::cli::util::cluster_identifiers_from;
use crate::state::State;
use async_trait::async_trait;
use couchbase::{GenericManagementRequest, Request};
use futures::channel::oneshot;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use serde::Deserialize;
use std::sync::Arc;

pub struct ClustersHealth {
    state: Arc<State>,
}

impl ClustersHealth {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for ClustersHealth {
    fn name(&self) -> &str {
        "clusters health"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets config").named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
    }

    fn usage(&self) -> &str {
        "Performs health checks on the target cluster(s)"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        health(args, self.state.clone()).await
    }
}

async fn health(args: CommandArgs, state: Arc<State>) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;

    let mut converted = vec![];
    for identifier in cluster_identifiers {
        converted.push(check_autofailover(state.clone(), &identifier).await?);

        let bucket_names = grab_bucket_names(state.clone(), &identifier).await?;
        for bucket_name in bucket_names {
            converted.push(check_resident_ratio(state.clone(), &bucket_name, &identifier).await?);
        }
    }

    Ok(converted.into())
}

async fn grab_bucket_names(state: Arc<State>, identifier: &str) -> Result<Vec<String>, ShellError> {
    let core = match state.clusters().get(identifier) {
        Some(c) => c.cluster().core(),
        None => {
            return Err(ShellError::untagged_runtime_error("Cluster not found"));
        }
    };

    let (sender, receiver) = oneshot::channel();
    let request =
        GenericManagementRequest::new(sender, "/pools/default/buckets".into(), "get".into(), None);
    core.send(Request::GenericManagementRequest(request));

    let input = match receiver.await {
        Ok(i) => i,
        Err(e) => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Error streaming result {}",
                e
            )))
        }
    };
    let result = convert_cb_error(input)?;

    if !result.payload().is_some() {
        return Err(ShellError::untagged_runtime_error(
            "Empty response from cluster even though got 200 ok",
        ));
    }

    let payload = match result.payload() {
        Some(p) => p,
        None => {
            return Err(ShellError::untagged_runtime_error(
                "Empty response from cluster even though got 200 ok",
            ));
        }
    };
    let resp: Vec<BucketInfo> = serde_json::from_slice(payload)?;
    Ok(resp.into_iter().map(|b| b.name).collect::<Vec<_>>())
}

#[derive(Debug, Deserialize)]
struct BucketInfo {
    name: String,
}

async fn check_autofailover(state: Arc<State>, identifier: &str) -> Result<Value, ShellError> {
    let mut collected = TaggedDictBuilder::new(Tag::default());

    let core = match state.clusters().get(identifier) {
        Some(c) => c.cluster().core(),
        None => {
            return Err(ShellError::untagged_runtime_error("Cluster not found"));
        }
    };

    let (sender, receiver) = oneshot::channel();
    let request = GenericManagementRequest::new(
        sender,
        format!("/settings/autoFailover"),
        "get".into(),
        None,
    );
    core.send(Request::GenericManagementRequest(request));

    let input = match receiver.await {
        Ok(i) => i,
        Err(e) => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Error streaming result {}",
                e
            )))
        }
    };
    let result = convert_cb_error(input)?;

    let payload = match result.payload() {
        Some(p) => p,
        None => {
            return Err(ShellError::untagged_runtime_error(
                "Empty response from cluster even though got 200 ok",
            ));
        }
    };
    let resp: AutoFailoverSettings = serde_json::from_slice(payload)?;

    collected.insert_value("cluster", identifier.clone());
    collected.insert_value("check", "Autofailover Enabled".clone());
    collected.insert_value("bucket", "-".clone());
    collected.insert_value("expected", UntaggedValue::boolean(true));
    collected.insert_value("actual", UntaggedValue::boolean(resp.enabled));

    let remedy = if resp.enabled {
        "Not needed"
    } else {
        "Enable Autofailover"
    };
    collected.insert_value("remedy", remedy.clone());

    Ok(collected.into_value())
}

#[derive(Debug, Deserialize)]
struct AutoFailoverSettings {
    enabled: bool,
}

async fn check_resident_ratio(
    state: Arc<State>,
    bucket_name: &str,
    identifier: &str,
) -> Result<Value, ShellError> {
    let mut collected = TaggedDictBuilder::new(Tag::default());

    let core = match state.clusters().get(identifier) {
        Some(c) => c.cluster().core(),
        None => {
            return Err(ShellError::untagged_runtime_error("Cluster not found"));
        }
    };

    let (sender, receiver) = oneshot::channel();
    let request = GenericManagementRequest::new(
        sender,
        format!("/pools/default/buckets/{}/stats", bucket_name),
        "get".into(),
        None,
    );
    core.send(Request::GenericManagementRequest(request));

    let input = match receiver.await {
        Ok(i) => i,
        Err(e) => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Error streaming result {}",
                e
            )))
        }
    };
    let result = convert_cb_error(input)?;

    let payload = match result.payload() {
        Some(p) => p,
        None => {
            return Err(ShellError::untagged_runtime_error(
                "Empty response from cluster even though got 200 ok",
            ));
        }
    };
    let resp: BucketStats = serde_json::from_slice(payload)?;
    let ratio = match resp.op.samples.active_resident_ratios.last() {
        Some(r) => *r,
        None => {
            println!("Failed to get resident ratios");
            0
        }
    };

    collected.insert_value("cluster", identifier.clone());
    collected.insert_value("check", "Resident Ratio Too Low".clone());
    collected.insert_value("bucket", bucket_name.clone());
    collected.insert_value("expected", ">= 10%");
    collected.insert_value("actual", format!("{}%", &ratio));

    let remedy = if ratio >= 10 {
        "Not needed"
    } else {
        "Should be more than 10%"
    };
    collected.insert_value("remedy", remedy.clone());

    Ok(collected.into_value())
}

#[derive(Debug, Deserialize)]
struct BucketStats {
    op: BucketStatsOp,
}

#[derive(Debug, Deserialize)]
struct BucketStatsOp {
    samples: BucketStatsSamples,
}

#[derive(Debug, Deserialize)]
struct BucketStatsSamples {
    #[serde(rename = "vb_active_resident_items_ratio")]
    active_resident_ratios: Vec<u32>,
}
