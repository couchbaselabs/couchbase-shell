use crate::cli::util::cluster_identifiers_from;
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
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

pub struct CollectionsGet {
    state: Arc<Mutex<State>>,
}

impl CollectionsGet {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
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
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters to query against",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Fetches collections through the HTTP API"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        collections_get(self.state.clone(), args)
    }
}

fn collections_get(
    state: Arc<Mutex<State>>,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    let scope: Option<String> = args.get_flag("scope")?;

    let mut results: Vec<Value> = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };

        let bucket = match args.get_flag("bucket")? {
            Some(v) => v,
            None => match active_cluster.active_bucket() {
                Some(s) => s,
                None => {
                    return Err(ShellError::untagged_runtime_error(
                        "Could not auto-select a bucket - please use --bucket instead".to_string(),
                    ));
                }
            },
        };

        debug!(
            "Running collections get for bucket {:?}, scope {:?}",
            &bucket, &scope
        );

        let response = active_cluster.cluster().http_client().management_request(
            ManagementRequest::GetCollections { bucket },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

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
                let mut collected = TaggedDictBuilder::new(Tag::default());
                collected.insert_value("error", response.content().to_string());
                collected.insert_value("cluster", identifier.clone());
                results.push(collected.into_value());
                continue;
            }
        };

        for scope_res in manifest.scopes {
            if let Some(scope_name) = &scope {
                if scope_name != &scope_res.name {
                    continue;
                }
            }
            let collections = scope_res.collections;
            if collections.is_empty() {
                let mut collected = TaggedDictBuilder::new(Tag::default());
                collected.insert_value("scope", scope_res.name.clone());
                collected.insert_value("collection", "");
                collected.insert_value("max_expiry", UntaggedValue::duration(0));
                collected.insert_value("cluster", identifier.clone());
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
                collected.insert_value("cluster", identifier.clone());
                results.push(collected.into_value());
            }
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
