use crate::state::State;
use async_trait::async_trait;
use couchbase::{DurabilityLevel, GetBucketOptions, UpdateBucketOptions};
use log::debug;
use nu_cli::{CommandArgs, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::time::Duration;

pub struct BucketsUpdate {
    state: Arc<State>,
}

impl BucketsUpdate {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for BucketsUpdate {
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_update(self.state.clone(), args).await
    }
}

async fn buckets_update(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let name = match args.get("name") {
        Some(v) => match v.as_string() {
            Ok(name) => name,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("name is required")),
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

    debug!("Running buckets update for bucket {}", &name);

    let mgr = cluster.buckets();
    let mut settings = match mgr
        .get_bucket(name.clone(), GetBucketOptions::default())
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Failed to get bucket {}",
                e
            )));
        }
    };

    match args.get("ram") {
        Some(v) => match v.as_u64() {
            Ok(ram) => {
                settings.set_ram_quota_mb(ram);
            }
            Err(e) => return Err(e),
        },
        None => {}
    };
    match args.get("replicas") {
        Some(v) => match v.as_u64() {
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
        },
        None => {}
    };
    match args.get("flush") {
        Some(v) => match v.as_string() {
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
        },
        None => {}
    };
    let mut has_dura_changed = false;
    match args.get("durability") {
        Some(v) => match v.as_string() {
            Ok(d) => {
                has_dura_changed = true;
                settings.set_minimum_durability_level(match DurabilityLevel::try_from(d.as_str()) {
                    Ok(bt) => bt,
                    Err(e) => {
                        return Err(ShellError::untagged_runtime_error(format!(
                            "Failed to parse durability level {}",
                            e
                        )));
                    }
                })
            }
            Err(e) => return Err(e),
        },
        None => {}
    };
    match args.get("expiry") {
        Some(v) => match v.as_u64() {
            Ok(ex) => settings.set_max_expiry(Duration::from_secs(ex)),
            Err(e) => return Err(e),
        },
        None => {}
    };

    let result = mgr
        .update_bucket(settings, UpdateBucketOptions::default())
        .await;

    match result {
        Ok(_) => {
            if has_dura_changed {
                println!("Bucket durability level has been changed, you must perform a cluster rebalance before this change will take effect.")
            }
            return Ok(OutputStream::empty());
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
