use crate::cli::util::{cluster_identifiers_from, get_active_cluster, namespace_from_args};
use crate::cli::{
    client_error_to_shell_error, deserialize_error, generic_error, unexpected_status_code_error,
};
use crate::client::ManagementRequest;
use crate::remote_cluster::RemoteCluster;
use crate::state::State;
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signals, Signature, Span, SyntaxShape};
use serde_json::{json, Value};
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct VectorCreateIndex {
    state: Arc<Mutex<State>>,
}

impl VectorCreateIndex {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for VectorCreateIndex {
    fn name(&self) -> &str {
        "vector create-index"
    }

    fn signature(&self) -> Signature {
        Signature::build("vector create-index")
            .required("name", SyntaxShape::String, "the index name")
            .required(
                "field",
                SyntaxShape::String,
                "name of the vector field to build the index on",
            )
            .required(
                "dimension",
                SyntaxShape::Int,
                "the dimension of the vectors the index will be built on",
            )
            .named(
                "similarity-metric",
                SyntaxShape::String,
                "metric used to calculate vector similarity - defaults to l2_norm",
                None,
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .named(
                "bucket",
                SyntaxShape::String,
                "the name of the bucket",
                None,
            )
            .named("scope", SyntaxShape::String, "the name of the scope", None)
            .named(
                "collection",
                SyntaxShape::String,
                "the name of the collection",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn description(&self) -> &str {
        "Creates a vector index"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        run(self.state.clone(), engine_state, stack, call, input)
    }
}

fn run(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let name: String = call.req(engine_state, stack, 0)?;
    let field: String = call.req(engine_state, stack, 1)?;
    let dimension: u16 = call.req(engine_state, stack, 2)?;

    let bucket_flag: Option<String> = call.get_flag(engine_state, stack, "bucket")?;
    let scope_flag: Option<String> = call.get_flag(engine_state, stack, "scope")?;
    let collection_flag: Option<String> = call.get_flag(engine_state, stack, "collection")?;

    let sim_metric = call
        .get_flag(engine_state, stack, "similarity-metric")?
        .unwrap_or("l2_norm".to_string());
    SimilarityMetric::try_from(sim_metric.as_str())?;

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        let (bucket, scope, collection) = index_creation_namespace(
            bucket_flag.clone(),
            scope_flag.clone(),
            collection_flag.clone(),
            cluster,
            span,
        )?;

        let uuid = get_bucket_uuid(cluster, bucket.clone(), signals.clone(), span)?;
        let json = create_index_json(
            &name,
            (bucket.clone(), scope.clone(), collection.clone()),
            dimension,
            &field,
            &sim_metric,
            &uuid,
        );

        let response = cluster
            .cluster()
            .http_client()
            .management_request(
                ManagementRequest::VectorCreateIndex {
                    bucket: bucket.clone(),
                    scope: scope.clone(),
                    name: name.clone(),
                    payload: json.to_string(),
                },
                Instant::now().add(cluster.timeouts().search_timeout()),
                signals.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        if response.status() != 200 {
            return Err(unexpected_status_code_error(
                response.status(),
                response.content()?,
                span,
            ));
        }
    }

    Ok(PipelineData::empty())
}

pub enum SimilarityMetric {
    L2Norm,
    DotProduct,
}

impl TryFrom<&str> for SimilarityMetric {
    type Error = ShellError;

    fn try_from(alias: &str) -> Result<Self, Self::Error> {
        match alias {
            "l2_norm" => Ok(SimilarityMetric::L2Norm),
            "dot_product" => Ok(SimilarityMetric::DotProduct),
            _ => Err(generic_error(
                "Invalid similarity metric",
                "The supported similarity metrics are 'l2_norm' and 'dot_product'".to_string(),
                None,
            )),
        }
    }
}

fn get_bucket_uuid(
    cluster: &RemoteCluster,
    bucket: String,
    signals: Signals,
    span: Span,
) -> Result<String, ShellError> {
    let response = cluster
        .cluster()
        .http_client()
        .management_request(
            ManagementRequest::GetBucket { name: bucket },
            Instant::now().add(cluster.timeouts().management_timeout()),
            signals.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    let content: serde_json::Map<String, Value> = serde_json::from_str(&response.content()?)
        .map_err(|e| deserialize_error(e.to_string(), span))?;
    match content.get("uuid") {
        Some(id) => Ok(id.as_str().unwrap().to_string()),
        None => Err(generic_error(
            "Could not retrieve bucket uuid from config",
            None,
            None,
        )),
    }
}

fn index_creation_namespace(
    bucket_flag: Option<String>,
    scope_flag: Option<String>,
    collection_flag: Option<String>,
    cluster: &RemoteCluster,
    span: Span,
) -> Result<(String, String, String), ShellError> {
    let (bucket, mut scope, mut collection) =
        namespace_from_args(bucket_flag, scope_flag, collection_flag, cluster, span)?;
    if scope.is_empty() {
        scope = "_default".into()
    }
    if collection.is_empty() {
        collection = "_default".into()
    }
    Ok((bucket, scope, collection))
}

fn create_index_json(
    index_name: &str,
    namespace: (String, String, String),
    dimension: u16,
    field: &str,
    sim_metric: &str,
    uuid: &str,
) -> serde_json::Value {
    let (bucket, scope, collection) = namespace;
    json!({
     "name": index_name,
     "type": "fulltext-index",
     "params": {
      "mapping": {
       "types": {
        format!("{}.{}", scope, collection): {
         "enabled": true,
         "dynamic": true,
         "properties": {
          field: {
           "enabled": true,
           "dynamic": false,
           "fields": [
            {
             "name": field,
             "type": "vector",
             "store": false,
             "index": true,
             "include_term_vectors": false,
             "include_in_all": false,
             "docvalues": false,
             "similarity": sim_metric,
             "vector_index_optimized_for": "recall",
             "dims": dimension
            }
           ]
          }
         }
        }
       },
       "default_mapping": {
        "enabled": false,
        "dynamic": true
       },
       "default_type": "_default",
       "default_analyzer": "standard",
       "default_datetime_parser": "dateTimeOptional",
       "default_field": "_all",
       "store_dynamic": false,
       "index_dynamic": true,
       "docvalues_dynamic": false
      },
      "store": {
       "indexType": "scorch",
       "kvStoreName": ""
      },
      "doc_config": {
       "mode": "scope.collection.type_field",
       "type_field": "type",
       "docid_prefix_delim": "",
       "docid_regexp": ""
      }
     },
     "sourceType": "couchbase",
     "sourceName": bucket,
     "sourceUUID": uuid,
     "sourceParams": {},
     "planParams": {
      "maxPartitionsPerPIndex": 1024,
      "numReplicas": 0,
      "indexPartitions": 1
     },
     "uuid": ""
    })
}
