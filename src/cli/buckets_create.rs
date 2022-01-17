use crate::cli::buckets_builder::{
    BucketSettingsBuilder, BucketType, DurabilityLevel, JSONCloudBucketSettings,
};
use crate::cli::util::cluster_identifiers_from;
use crate::client::{CapellaRequest, HttpResponse, ManagementRequest};
use crate::state::{CapellaEnvironment, State};
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
            .required("name", SyntaxShape::String, "the name of the bucket")
            .required(
                "ram",
                SyntaxShape::Int,
                "the amount of ram to allocate (mb)",
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
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
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
    let name: String = args.req(0)?;
    let ram = args.req(1)?;

    let bucket_type: Option<String> = args.get_flag("type")?;
    let replicas: Option<i32> = args.get_flag("replicas")?;
    let flush = args.get_flag("flush")?.unwrap_or(false);
    let durability: Option<String> = args.get_flag("durability")?;
    let expiry = args.get_flag("expiry")?;

    debug!("Running buckets create for bucket {}", &name);

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    let mut builder = BucketSettingsBuilder::new(name).ram_quota_mb(ram);
    if let Some(ref t) = bucket_type {
        builder = builder.bucket_type(match BucketType::try_from(t.as_str()) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(ShellError::unexpected(format!(
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
                return Err(ShellError::unexpected(format!(
                    "Failed to parse durability level {}",
                    e
                )));
            }
        });
    }
    if flush {
        builder = builder.flush_enabled(flush);
    }
    if let Some(ref d) = durability {
        builder = builder.minimum_durability_level(match DurabilityLevel::try_from(d.as_str()) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(ShellError::unexpected(format!(
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

    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::unexpected("Cluster not found"));
            }
        };

        if active_cluster.capella_org().is_some()
            && (bucket_type.clone().is_some()
                || flush
                || durability.clone().is_some()
                || expiry.is_some())
        {
            return Err(ShellError::unexpected(
                "Capella flag cannot be used with type, flush, durability, or expiry",
            ));
        }

        let response: HttpResponse;
        if let Some(plane) = active_cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster =
                cloud.find_cluster(identifier.clone(), deadline.clone(), ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(ShellError::unexpected(
                    "buckets create cannot be run against hosted Capella clusters",
                ));
            }

            let json_settings = JSONCloudBucketSettings::try_from(&settings)?;
            response = cloud.capella_request(
                CapellaRequest::CreateBucket {
                    cluster_id: cluster.id(),
                    payload: serde_json::to_string(&json_settings)?,
                },
                deadline,
                ctrl_c.clone(),
            )?;
        } else {
            let cluster = active_cluster.cluster();

            let form = settings.as_form(false)?;
            let payload = serde_urlencoded::to_string(&form).unwrap();

            response = cluster.http_client().management_request(
                ManagementRequest::CreateBucket { payload },
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?;
        }

        match response.status() {
            200 => {}
            201 => {}
            202 => {}
            _ => {
                return Err(ShellError::unexpected(response.content()));
            }
        }
    }

    Ok(OutputStream::empty())
}
