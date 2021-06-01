use crate::cli::buckets_builder::{BucketSettings, DurabilityLevel, JSONBucketSettings};
use crate::client::ManagementRequest;
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

pub struct BucketsUpdate {
    state: Arc<Mutex<State>>,
}

impl BucketsUpdate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for BucketsUpdate {
    fn name(&self) -> &str {
        "buckets update"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets update")
            .required_named("name", SyntaxShape::String, "the name of the bucket", None)
            .named(
                "ram",
                SyntaxShape::Int,
                "the amount of ram to allocate (mb)",
                None,
            )
            .named(
                "replicas",
                SyntaxShape::Int,
                "the number of replicas for the bucket",
                None,
            )
            .named(
                "flush",
                SyntaxShape::String,
                "whether to enable flush",
                None,
            )
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
            .named(
                "cluster",
                SyntaxShape::String,
                "the cluster to create the bucket against",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Updates a bucket"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_update(self.state.clone(), args)
    }
}

fn buckets_update(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let name = match args.call_info.args.get("name") {
        Some(v) => match v.as_string() {
            Ok(name) => name,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("name is required")),
    };
    let guard = state.lock().unwrap();
    let active_cluster = guard.active_cluster();
    debug!("Running buckets update for bucket {}", &name);

    let response = active_cluster.cluster().management_request(
        ManagementRequest::GetBucket { name: name.clone() },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c.clone(),
    )?;

    let content: JSONBucketSettings = serde_json::from_str(response.content())?;
    let mut settings = BucketSettings::try_from(content)?;

    if let Some(v) = args.call_info.args.get("ram") {
        match v.as_u64() {
            Ok(ram) => {
                settings.set_ram_quota_mb(ram);
            }
            Err(e) => return Err(e),
        }
    };
    if let Some(v) = args.call_info.args.get("replicas") {
        match v.as_u64() {
            Ok(replicas) => settings.set_num_replicas(match u32::try_from(replicas) {
                Ok(bt) => bt,
                Err(e) => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Failed to parse durability level {}",
                        e
                    )));
                }
            }),
            Err(e) => return Err(e),
        }
    };
    if let Some(v) = args.call_info.args.get("flush") {
        match v.as_string() {
            Ok(f) => {
                let flush_str = match f.strip_prefix("$") {
                    Some(f2) => f2,
                    None => f.as_str(),
                };

                match flush_str.parse::<bool>() {
                    Ok(b) => settings.set_flush_enabled(b),
                    Err(e) => {
                        return Err(ShellError::untagged_runtime_error(format!(
                            "Failed to parse flush {}",
                            e
                        )));
                    }
                }
            }
            Err(_) => match v.as_bool() {
                Ok(f) => settings.set_flush_enabled(f),
                Err(e) => return Err(e),
            },
        }
    };
    if let Some(v) = args.call_info.args.get("durability") {
        match v.as_string() {
            Ok(d) => settings.set_minimum_durability_level(DurabilityLevel::try_from(d.as_str())?),
            Err(e) => return Err(e),
        }
    };
    if let Some(v) = args.call_info.args.get("expiry") {
        match v.as_u64() {
            Ok(ex) => settings.set_max_expiry(Duration::from_secs(ex)),
            Err(e) => return Err(e),
        }
    };

    let form = settings.as_form(true)?;
    let payload = serde_urlencoded::to_string(&form).unwrap();

    let response = active_cluster.cluster().management_request(
        ManagementRequest::UpdateBucket { name, payload },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    match response.status() {
        200 => Ok(OutputStream::empty()),
        202 => Ok(OutputStream::empty()),
        _ => Err(ShellError::untagged_runtime_error(
            response.content().to_string(),
        )),
    }
}
