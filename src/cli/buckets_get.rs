//! The `buckets get` command fetches buckets from the server.

use crate::state::State;

use crate::cli::util::cluster_identifiers_from;
use crate::client::ManagementRequest;
use async_trait::async_trait;
use log::debug;
use nu_cli::OutputStream;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use serde::Deserialize;
use std::sync::Arc;

pub struct BucketsGet {
    state: Arc<State>,
}

impl BucketsGet {
    pub fn new(state: Arc<State>) -> Self {
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
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
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
        "Fetches buckets through the HTTP API"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_get(self.state.clone(), args).await
    }
}

async fn buckets_get(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let bucket = match args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
    {
        Some(v) => v,
        None => "".into(),
    };

    debug!("Running buckets get for bucket {:?}", &bucket);

    if bucket == "" {
        buckets_get_all(state, cluster_identifiers).await
    } else {
        buckets_get_one(state, cluster_identifiers, bucket).await
    }
}

async fn buckets_get_one(
    state: Arc<State>,
    cluster_identifiers: Vec<String>,
    name: String,
) -> Result<OutputStream, ShellError> {
    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let cluster = match state.clusters().get(&identifier) {
            Some(c) => c.cluster(),
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let response = cluster
            .management_request(ManagementRequest::GetBucket { name: name.clone() })
            .await;

        let content = serde_json::from_str(response.content()).unwrap();
        results.push(bucket_to_tagged_dict(content, identifier));
    }

    Ok(OutputStream::from(results))
}

async fn buckets_get_all(
    state: Arc<State>,
    cluster_identifiers: Vec<String>,
) -> Result<OutputStream, ShellError> {
    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let cluster = match state.clusters().get(&identifier) {
            Some(c) => c.cluster(),
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let response = cluster
            .management_request(ManagementRequest::GetBuckets)
            .await;

        let content: Vec<BucketConfig> = serde_json::from_str(response.content()).unwrap();

        for bucket in content.into_iter() {
            results.push(bucket_to_tagged_dict(bucket, identifier.clone()));
        }
    }

    Ok(OutputStream::from(results))
}

fn bucket_to_tagged_dict(bucket: BucketConfig, cluster_name: String) -> Value {
    let bucket_type = match bucket.bucket_type.as_str() {
        "membase" => "couchbase",
        "memcached" => "memcached",
        "ephemeral" => "ephemeral",
        _ => "unknown",
    };

    let mut collected = TaggedDictBuilder::new(Tag::default());
    collected.insert_value("cluster", cluster_name);
    collected.insert_value("name", bucket.name);
    collected.insert_value("type", bucket_type);
    collected.insert_value("replicas", UntaggedValue::int(bucket.num_replicas));
    collected.insert_value(
        "min_durability_level",
        bucket.durability_level.unwrap_or("none".to_string()),
    );
    /*collected.insert_value(
        "ram_quota",
        UntaggedValue::filesize(bucket.ram_quota_mb() * 1000 * 1000),
    );
    collected.insert_value("flush_enabled", bucket.flush_enabled());
    */
    collected.into_value()
}
#[derive(Deserialize, Debug)]
struct BucketConfig {
    name: String,
    #[serde(rename = "bucketType")]
    bucket_type: String,
    #[serde(rename = "replicaNumber")]
    num_replicas: u32,
    #[serde(rename = "durabilityMinLevel")]
    durability_level: Option<String>,
}
