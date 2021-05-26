//! The `buckets get` command fetches buckets from the server.

use crate::state::State;

use crate::cli::buckets_builder::{BucketSettings, JSONBucketSettings};
use crate::cli::util::cluster_identifiers_from;
use crate::client::ManagementRequest;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::time::Instant;

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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        buckets_get(self.state.clone(), args)
    }
}

fn buckets_get(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let bucket = match args
        .call_info
        .args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
    {
        Some(v) => v,
        None => "".into(),
    };

    debug!("Running buckets get for bucket {:?}", &bucket);

    if bucket.is_empty() {
        buckets_get_all(state, cluster_identifiers, ctrl_c)
    } else {
        buckets_get_one(state, cluster_identifiers, bucket, ctrl_c)
    }
}

fn buckets_get_one(
    state: Arc<State>,
    cluster_identifiers: Vec<String>,
    name: String,
    ctrl_c: Arc<AtomicBool>,
) -> Result<OutputStream, ShellError> {
    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let cluster = match state.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let response = cluster.cluster().management_request(
            ManagementRequest::GetBucket { name: name.clone() },
            Instant::now().add(cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        let content: JSONBucketSettings = serde_json::from_str(response.content())?;
        results.push(bucket_to_tagged_dict(
            BucketSettings::try_from(content)?,
            identifier,
        ));
    }

    Ok(OutputStream::from(results))
}

fn buckets_get_all(
    state: Arc<State>,
    cluster_identifiers: Vec<String>,
    ctrl_c: Arc<AtomicBool>,
) -> Result<OutputStream, ShellError> {
    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let cluster = match state.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let response = cluster.cluster().management_request(
            ManagementRequest::GetBuckets,
            Instant::now().add(cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        let content: Vec<JSONBucketSettings> = serde_json::from_str(response.content())?;

        for bucket in content.into_iter() {
            results.push(bucket_to_tagged_dict(
                BucketSettings::try_from(bucket)?,
                identifier.clone(),
            ));
        }
    }

    Ok(OutputStream::from(results))
}

fn bucket_to_tagged_dict(bucket: BucketSettings, cluster_name: String) -> Value {
    let mut collected = TaggedDictBuilder::new(Tag::default());
    collected.insert_value("cluster", cluster_name);
    collected.insert_value("name", bucket.name());
    collected.insert_value("type", bucket.bucket_type().to_string());
    collected.insert_value("replicas", UntaggedValue::int(bucket.num_replicas()));
    collected.insert_value(
        "min_durability_level",
        bucket.minimum_durability_level().to_string(),
    );
    collected.insert_value("ram_quota", UntaggedValue::filesize(bucket.ram_quota_mb()));
    collected.insert_value("flush_enabled", bucket.flush_enabled());
    collected.into_value()
}
