use crate::cli::buckets_builder::{BucketSettings, DurabilityLevel, JSONBucketSettings};
use crate::cli::error::{
    bucket_not_found_error, client_error_to_shell_error, deserialize_error, generic_error,
    serialize_error, unexpected_status_code_error,
};
use crate::cli::util::{cluster_identifiers_from, get_active_cluster, validate_is_not_cloud};
use crate::client::ManagementRequest;
use crate::state::State;
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, SyntaxShape};
use std::convert::TryFrom;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::{Duration, Instant};

#[derive(Clone)]
pub struct BucketsUpdate {
    state: Arc<Mutex<State>>,
}

impl BucketsUpdate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for BucketsUpdate {
    fn name(&self) -> &str {
        "buckets update"
    }

    fn signature(&self) -> Signature {
        Signature::build("buckets update")
            .required("name", SyntaxShape::String, "the name of the bucket")
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
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Updates a bucket"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        buckets_update(self.state.clone(), engine_state, stack, call, input)
    }
}

fn buckets_update(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let name: String = call.req(engine_state, stack, 0)?;
    let ram: Option<i64> = call.get_flag(engine_state, stack, "ram")?;
    let replicas: Option<i64> = call.get_flag(engine_state, stack, "replicas")?;
    let flush = call
        .get_flag(engine_state, stack, "flush")?
        .unwrap_or(false);
    let durability = call.get_flag(engine_state, stack, "durability")?;
    let expiry: Option<i64> = call.get_flag(engine_state, stack, "expiry")?;

    debug!("Running buckets update for bucket {}", &name);

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span)?;
        validate_is_not_cloud(active_cluster, "buckets", span)?;

        let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
        let get_response = active_cluster
            .cluster()
            .http_client()
            .management_request(
                ManagementRequest::GetBucket { name: name.clone() },
                deadline,
                ctrl_c.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;
        if get_response.status() != 200 {
            debug!("Failed to get buckets from server");
            return Err(unexpected_status_code_error(
                get_response.status(),
                get_response.content(),
                span,
            ));
        }

        let content: JSONBucketSettings = serde_json::from_str(get_response.content())
            .map_err(|e| deserialize_error(e.to_string(), span))?;
        let mut settings = BucketSettings::try_from(content)
            .map_err(|e| generic_error(format!("Invalid setting {}", e), None, span))?;

        update_bucket_settings(
            &mut settings,
            ram.map(|v| v as u64),
            replicas.map(|v| v as u64),
            flush,
            durability.clone(),
            expiry.map(|v| v as u64),
            span,
        )?;

        let form = settings
            .as_form(true)
            .map_err(|e| generic_error(format!("Invalid setting {}", e), None, span))?;
        let payload =
            serde_urlencoded::to_string(&form).map_err(|e| serialize_error(e.to_string(), span))?;

        let response = active_cluster
            .cluster()
            .http_client()
            .management_request(
                ManagementRequest::UpdateBucket {
                    name: name.clone(),
                    payload,
                },
                deadline,
                ctrl_c.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        match response.status() {
            200 => {}
            201 => {}
            202 => {}
            404 => {
                if response
                    .content()
                    .to_string()
                    .to_lowercase()
                    .contains("resource not found")
                {
                    return Err(bucket_not_found_error(name, span));
                }
            }
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
                ));
            }
        }
    }

    Ok(PipelineData::new(span))
}

fn update_bucket_settings(
    settings: &mut BucketSettings,
    ram: Option<u64>,
    replicas: Option<u64>,
    flush: bool,
    durability: Option<String>,
    expiry: Option<u64>,
    span: Span,
) -> Result<(), ShellError> {
    if let Some(r) = ram {
        settings.set_ram_quota_mb(r);
    }
    if let Some(r) = replicas {
        settings.set_num_replicas(match u32::try_from(r) {
            Ok(bt) => bt,
            Err(e) => {
                return Err(generic_error(
                    format!("Failed to parse num replicas {}", e),
                    None,
                    span,
                ));
            }
        });
    }
    if flush {
        settings.set_flush_enabled(flush);
    }
    if let Some(d) = durability {
        settings.set_minimum_durability_level(match DurabilityLevel::try_from(d.as_str()) {
            Ok(bt) => bt,
            Err(_e) => {

                return Err(generic_error(format!("Failed to parse durability level {}", d),
                                         "Allowed values for durability level are one, majority, majorityAndPersistActive, persistToMajority".to_string(), span));
            }
        });
    }
    if let Some(e) = expiry {
        settings.set_max_expiry(Duration::from_secs(e));
    }

    Ok(())
}
