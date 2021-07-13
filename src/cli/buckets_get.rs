//! The `buckets get` command fetches buckets from the server.

use crate::state::State;

use crate::cli::buckets_builder::{BucketSettings, JSONBucketSettings, JSONCloudBucketSettings};
use crate::cli::util::cluster_identifiers_from;
use crate::client::{CloudRequest, ManagementRequest};
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct BucketsGet {
    state: Arc<Mutex<State>>,
}

impl BucketsGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for BucketsGet {
    fn name(&self) -> &str {
        "buckets get"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets get")
            .required("bucket", SyntaxShape::String, "the name of the bucket")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Fetches buckets through the HTTP API"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_get(self.state.clone(), args)
    }
}

fn buckets_get(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let bucket: String = args.req(0)?;

    debug!("Running buckets get for bucket {:?}", &bucket);

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let guard = state.lock().unwrap();
        let cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        if let Some(plane) = cluster.cloud_org() {
            let cloud = guard.cloud_org_for_cluster(plane)?.client();
            let cluster_id = cloud.find_cluster_id(
                identifier.clone(),
                Instant::now().add(cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?;
            let response = cloud.cloud_request(
                CloudRequest::GetBuckets { cluster_id },
                Instant::now().add(cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?;
            if response.status() != 200 {
                return Err(ShellError::unexpected(response.content()));
            }

            let content: Vec<JSONCloudBucketSettings> = serde_json::from_str(response.content())?;
            let mut bucket_settings: Option<JSONCloudBucketSettings> = None;

            for b in content.into_iter() {
                if b.name() == bucket.clone() {
                    bucket_settings = Some(b);
                    break;
                }
            }

            if let Some(b) = bucket_settings {
                results.push(bucket_to_tagged_dict(
                    BucketSettings::try_from(b)?,
                    identifier,
                    true,
                ));
            } else {
                return Err(ShellError::unexpected("bucket not found"));
            }
        } else {
            let response = cluster.cluster().http_client().management_request(
                ManagementRequest::GetBucket {
                    name: bucket.clone(),
                },
                Instant::now().add(cluster.timeouts().query_timeout()),
                ctrl_c.clone(),
            )?;

            let content: JSONBucketSettings = serde_json::from_str(response.content())?;
            results.push(bucket_to_tagged_dict(
                BucketSettings::try_from(content)?,
                identifier,
                false,
            ));
        }
    }

    Ok(OutputStream::from(results))
}

pub(crate) fn bucket_to_tagged_dict(
    bucket: BucketSettings,
    cluster_name: String,
    is_cloud: bool,
) -> Value {
    let mut collected = TaggedDictBuilder::new(Tag::default());
    collected.insert_value("cluster", cluster_name);
    collected.insert_value("name", bucket.name());
    collected.insert_value("type", bucket.bucket_type().to_string());
    collected.insert_value("replicas", UntaggedValue::int(bucket.num_replicas()));
    collected.insert_value(
        "min_durability_level",
        bucket.minimum_durability_level().to_string(),
    );
    collected.insert_value(
        "ram_quota",
        UntaggedValue::filesize(bucket.ram_quota_mb() * 1024 * 1024),
    );
    collected.insert_value("flush_enabled", bucket.flush_enabled());
    collected.insert_value("status", bucket.status().unwrap_or(&"".to_string()).clone());
    collected.insert_value("cloud", is_cloud);
    collected.into_value()
}
