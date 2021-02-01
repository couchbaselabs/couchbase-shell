use crate::state::State;
use async_trait::async_trait;
use couchbase::{BucketSettingsBuilder, BucketType, CreateBucketOptions, DurabilityLevel};
use log::debug;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::time::Duration;

pub struct BucketsCreate {
    state: Arc<State>,
}

impl BucketsCreate {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for BucketsCreate {
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
        "Creates a bucket"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_create(self.state.clone(), args).await
    }
}

async fn buckets_create(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let name = match args.get("name") {
        Some(v) => match v.as_string() {
            Ok(name) => name,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("name is required")),
    };
    let ram = match args.get("ram") {
        Some(v) => match v.as_u64() {
            Ok(ram) => ram,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("ram is required")),
    };
    let bucket_type = match args.get("type") {
        Some(v) => match v.as_string() {
            Ok(t) => Some(t),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let replicas = match args.get("replicas") {
        Some(v) => match v.as_u64() {
            Ok(pwd) => Some(pwd),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let flush = match args.get("flush") {
        Some(v) => match v.as_string() {
            Ok(f) => {
                let flush_str = match f.strip_prefix("$") {
                    Some(f2) => f2,
                    None => f.as_str(),
                };

                match flush_str.parse::<bool>() {
                    Ok(b) => Some(b),
                    Err(e) => {
                        return Err(ShellError::untagged_runtime_error(format!(
                            "Failed to parse flush {}",
                            e
                        )));
                    }
                }
            }
            Err(_) => match v.as_bool() {
                Ok(f) => Some(f),
                Err(e) => return Err(e),
            },
        },
        None => None,
    };
    let durability = match args.get("durability") {
        Some(v) => match v.as_string() {
            Ok(pwd) => Some(pwd),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let expiry = match args.get("expiry") {
        Some(v) => match v.as_u64() {
            Ok(pwd) => Some(pwd),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let cluster = match args.get("cluster") {
        Some(v) => match v.as_string() {
            Ok(pwd) => match state.clusters().get(&pwd) {
                Some(c) => c.cluster(),
                None => {
                    return Err(ShellError::untagged_runtime_error("Cluster not found"));
                }
            },
            Err(e) => return Err(e),
        },
        None => state.active_cluster().cluster(),
    };

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
    if let Some(f) = flush {
        builder = builder.flush_enabled(f);
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

    let mgr = cluster.buckets();
    let result = mgr
        .create_bucket(builder.build(), CreateBucketOptions::default())
        .await;

    match result {
        Ok(_) => Ok(OutputStream::empty()),
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
