use crate::cli::buckets_builder::{
    BucketSettingsBuilder, BucketType, DurabilityLevel, JSONCloudBucketSettings,
};
use crate::client::{CloudRequest, HttpResponse, ManagementRequest};
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, Instant};

pub struct BucketsCreate {
    state: Arc<Mutex<State>>,
}

impl BucketsCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for BucketsCreate {
    fn name(&self) -> &str {
        "buckets create"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets create")
            .required_named("name", SyntaxShape::String, "the name of the bucket", None)
            .required_named(
                "ram",
                SyntaxShape::Int,
                "the amount of ram to allocate (mb)",
                None,
            )
            .named("type", SyntaxShape::String, "the type of bucket", None)
            .named(
                "replicas",
                SyntaxShape::Int,
                "the number of replicas for the bucket",
                None,
            )
            .switch("flush", "whether to enable flush", None)
            .named(
                "durability",
                SyntaxShape::String,
                "the minimum durability level",
                None,
            )
            .named(
                "expiry",
                SyntaxShape::Int,
                "the maximum expiry for documents created in this bucket (seconds)",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Creates a bucket"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_create(self.state.clone(), args)
    }
}

fn buckets_create(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let name: String = args.req_named("name")?;
    let ram = args.req_named("ram")?;

    let bucket_type: Option<String> = args.get_flag("type")?;
    let replicas: Option<i32> = args.get_flag("replicas")?;
    let flush = args.get_flag("flush")?.unwrap_or(false);
    let durability: Option<String> = args.get_flag("durability")?;
    let expiry = args.get_flag("expiry")?;

    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();

    if active_cluster.cloud().is_some()
        && (bucket_type.is_some() || flush || durability.is_some() || expiry.is_some())
    {
        return Err(ShellError::unexpected(
            "Cloud flag cannot be used with type, flush, durability, or expiry",
        ));
    }

    debug!("Running buckets create for bucket {}", &name);

    let mut builder = BucketSettingsBuilder::new(name).ram_quota_mb(ram);
    if let Some(t) = bucket_type {
        builder = builder.bucket_type(match BucketType::try_from(t.as_str()) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Failed to parse bucket type {}",
                    e
                )));
            }
        });
    }
    if let Some(r) = replicas {
        builder = builder.num_replicas(match u32::try_from(r) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Failed to parse durability level {}",
                    e
                )));
            }
        });
    }
    if flush {
        builder = builder.flush_enabled(flush);
    }
    if let Some(d) = durability {
        builder = builder.minimum_durability_level(match DurabilityLevel::try_from(d.as_str()) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Failed to parse durability level {}",
                    e
                )));
            }
        });
    }
    if let Some(e) = expiry {
        builder = builder.max_expiry(Duration::from_secs(e));
    }

    let settings = builder.build();

    let response: HttpResponse;
    if let Some(c) = active_cluster.cloud() {
        let identifier = guard.active();
        let cloud = guard.cloud_for_cluster(c)?.cloud();
        let cluster_id = cloud.find_cluster_id(
            identifier,
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;
        let json_settings = JSONCloudBucketSettings::try_from(settings)?;
        response = cloud.cloud_request(
            CloudRequest::CreateBucket {
                cluster_id,
                payload: serde_json::to_string(&json_settings)?,
            },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;
    } else {
        let cluster = active_cluster.cluster();

        let form = settings.as_form(false)?;
        let payload = serde_urlencoded::to_string(&form).unwrap();

        response = cluster.http_client().management_request(
            ManagementRequest::CreateBucket { payload },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c,
        )?;
    }

    match response.status() {
        200 => Ok(OutputStream::empty()),
        201 => Ok(OutputStream::empty()),
        202 => Ok(OutputStream::empty()),
        _ => Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        )),
    }
}
