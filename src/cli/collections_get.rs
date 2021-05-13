use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde_derive::Deserialize;
use std::sync::Arc;
use std::time::Duration;

pub struct CollectionsGet {
    state: Arc<State>,
}

impl CollectionsGet {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for CollectionsGet {
    fn name(&self) -> &str {
        "collections get"
    }

    fn signature(&self) -> Signature {
        Signature::build("collections get")
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named("scope", SyntaxShape::String, "the name of the scope", None)
    }

    fn usage(&self) -> &str {
        "Fetches collections through the HTTP API"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        collections_get(self.state.clone(), args)
    }
}

fn collections_get(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;

    let bucket = match args
        .call_info
        .args
        .get("bucket")
        .map(|bucket| bucket.as_string().ok())
        .flatten()
    {
        Some(v) => v,
        None => match state.active_cluster().active_bucket() {
            Some(s) => s,
            None => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Could not auto-select a bucket - please use --bucket instead"
                )));
            }
        },
    };

    let scope = args
        .call_info
        .args
        .get("scope")
        .map(|c| c.as_string().ok())
        .flatten();

    debug!(
        "Running collections get for bucket {:?}, scope {:?}",
        &bucket, &scope
    );

    let active_cluster = state.active_cluster();
    let response = active_cluster
        .cluster()
        .management_request(ManagementRequest::GetCollections { bucket })?;

    let manifest: Manifest = match response.status() {
        200 => match serde_json::from_str(response.content()) {
            Ok(m) => m,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Failed to decode response body {}",
                    e,
                )));
            }
        },
        _ => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Request failed {}",
                response.content(),
            )));
        }
    };

    let mut results: Vec<Value> = vec![];
    for scope_res in manifest.scopes {
        if let Some(scope_name) = &scope {
            if scope_name != &scope_res.name {
                continue;
            }
        }
        let collections = scope_res.collections;
        if collections.len() == 0 {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("scope", scope_res.name.clone());
            collected.insert_value("collection", "");
            collected.insert_value("max_expiry", UntaggedValue::duration(0));
            results.push(collected.into_value());
            continue;
        }

        for collection in collections {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("scope", scope_res.name.clone());
            collected.insert_value("collection", collection.name);
            collected.insert_value(
                "max_expiry",
                UntaggedValue::duration(Duration::from_secs(collection.max_expiry).as_nanos()),
            );
            results.push(collected.into_value());
        }
    }
    Ok(OutputStream::from(results))
}

#[derive(Debug, Deserialize)]
pub struct ManifestCollection {
    pub uid: String,
    pub name: String,
    #[serde(rename = "maxTTL")]
    pub max_expiry: u64,
}

#[derive(Debug, Deserialize)]
pub struct ManifestScope {
    pub uid: String,
    pub name: String,
    pub collections: Vec<ManifestCollection>,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub uid: String,
    pub scopes: Vec<ManifestScope>,
}
