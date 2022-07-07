use crate::cli::cloud_json::JSONCloudDeleteAllowListRequest;
use crate::cli::util::{cluster_identifiers_from, get_active_cluster, validate_is_cloud};
use crate::client::CapellaRequest;
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{
    cant_run_against_hosted_capella_error, deserialize_error, unexpected_status_code_error,
};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct AllowListsDrop {
    state: Arc<Mutex<State>>,
}

impl AllowListsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for AllowListsDrop {
    fn name(&self) -> &str {
        "allowlists drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("allowlists drop")
            .required(
                "address",
                SyntaxShape::String,
                "the address to disallow access",
            )
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Removes an address to disallow Capella cluster access"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        addresses_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn addresses_drop(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let address: String = call.req(engine_state, stack, 0)?;
    let span = call.head;

    debug!("Running allowlists drop for {}", &address);

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;
        validate_is_cloud(active_cluster, "allowlists drop", span.clone())?;

        let cloud = guard
            .capella_org_for_cluster(active_cluster.capella_org().unwrap())?
            .client();
        let cluster = cloud.find_cluster(
            identifier.clone(),
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        if cluster.environment() == CapellaEnvironment::Hosted {
            return Err(cant_run_against_hosted_capella_error(
                "allowlists drop",
                span,
            ));
        }

        let entry = JSONCloudDeleteAllowListRequest::new(address.clone());

        let response = cloud.capella_request(
            CapellaRequest::DeleteAllowListEntry {
                cluster_id: cluster.id(),
                payload: serde_json::to_string(&entry)
                    .map_err(|e| deserialize_error(e.to_string(), span))?,
            },
            Instant::now().add(active_cluster.timeouts().query_timeout()),
            ctrl_c.clone(),
        )?;

        match response.status() {
            204 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    span,
                ));
            }
        }
    }

    Ok(PipelineData::new(call.head))
}
