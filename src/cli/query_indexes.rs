use crate::cli::util::convert_json_value_to_nu_value;
use crate::client::{Client, ManagementRequest, QueryRequest};
use crate::state::{RemoteCluster, State};
use log::debug;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde::Deserialize;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct QueryIndexes {
    state: Arc<Mutex<State>>,
}

impl QueryIndexes {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for QueryIndexes {
    fn name(&self) -> &str {
        "query indexes"
    }

    fn signature(&self) -> Signature {
        Signature::build("query indexes")
            .switch(
                "definitions",
                "Whether to fetch the index definitions (changes the output format)",
                None,
            )
            .named(
                "cluster",
                SyntaxShape::String,
                "the cluster to query against",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Lists all query indexes"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        indexes(self.state.clone(), args)
    }
}

fn indexes(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;

    let guard = state.lock().unwrap();
    let active_cluster = match args.call_info.args.get("cluster") {
        Some(c) => {
            let identifier = match c.as_string() {
                Ok(s) => s,
                Err(e) => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Could not convert cluster name to string: {}",
                        e
                    )));
                }
            };
            match guard.clusters().get(identifier.as_str()) {
                Some(c) => c,
                None => {
                    return Err(ShellError::untagged_runtime_error(
                        "Could not get cluster from available clusters".to_string(),
                    ));
                }
            }
        }
        None => guard.active_cluster(),
    };

    let fetch_defs = match args.call_info.args.get("definitions") {
        Some(n) => n.as_bool()?,
        None => false,
    };

    if fetch_defs {
        return index_definitions(active_cluster, ctrl_c);
    }

    let statement = "select keyspace_id as `bucket`, name, state, `using` as `type`, ifmissing(condition, null) as condition, ifmissing(is_primary, false) as `primary`, index_key from system:indexes";

    debug!("Running n1ql query {}", &statement);

    let mut client = match Client::try_lookup_srv(active_cluster.hostnames()[0].clone()) {
        Ok(seeds) => active_cluster.cluster().http_client_with_seeds(seeds),
        Err(_) => active_cluster.cluster().http_client(),
    };

    let response = client.query_request(
        QueryRequest::Execute {
            statement: statement.into(),
            scope: None,
        },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    let content: serde_json::Value = serde_json::from_str(response.content())?;
    let converted = convert_json_value_to_nu_value(&content, Tag::default())?;
    Ok(OutputStream::one(converted))
}

#[derive(Debug, Deserialize)]
struct IndexDefinition {
    bucket: String,
    definition: String,
    collection: Option<String>,
    scope: Option<String>,
    #[serde(rename = "indexName")]
    index_name: String,
    status: String,
    #[serde(rename = "storageMode")]
    storage_mode: String,
    #[serde(rename = "numReplica")]
    replicas: u8,
}

#[derive(Debug, Deserialize)]
struct IndexStatus {
    indexes: Vec<IndexDefinition>,
}

fn index_definitions(
    cluster: &RemoteCluster,
    ctrl_c: Arc<AtomicBool>,
) -> Result<OutputStream, ShellError> {
    debug!("Running fetch n1ql indexes");

    let response = cluster.cluster().http_client().management_request(
        ManagementRequest::IndexStatus,
        Instant::now().add(cluster.timeouts().query_timeout()),
        ctrl_c,
    )?;

    let defs: IndexStatus = serde_json::from_str(response.content())?;
    let n = defs
        .indexes
        .into_iter()
        .map(|d| {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("bucket", d.bucket);
            collected.insert_value("scope", d.scope.unwrap_or_else(|| "".into()));
            collected.insert_value("collection", d.collection.unwrap_or_else(|| "".into()));
            collected.insert_value("name", d.index_name);
            collected.insert_value("status", d.status);
            collected.insert_value("storage_mode", d.storage_mode);
            collected.insert_value("replicas", UntaggedValue::int(d.replicas));
            collected.insert_value("definition", d.definition);

            collected.into_value()
        })
        .collect::<Vec<_>>();

    Ok(n.into())
}
