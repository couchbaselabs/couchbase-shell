use crate::cli::cloud_json::JSONCloudAppendAllowListRequest;
use crate::cli::util::{
    cluster_identifiers_from, cluster_not_found_error, json_parse_fail_error,
    unexpected_status_code_error, validate_is_cloud,
};
use crate::client::CapellaRequest;
use crate::state::{CapellaEnvironment, State};
use log::debug;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct AllowListsAdd {
    state: Arc<Mutex<State>>,
}

impl AllowListsAdd {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for AllowListsAdd {
    fn name(&self) -> &str {
        "allowlists add"
    }

    fn signature(&self) -> Signature {
        Signature::build("allowlists add")
            .required("address", SyntaxShape::String, "the address to add to allow access")
            .named(
                "duration",
                SyntaxShape::String,
                "the duration (hours) to allow access from this address, if not set then address is added for permanent access",
                None,
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
        "Adds an address to allow for Capella cluster access"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        addresses_add(self.state.clone(), engine_state, stack, call, input)
    }
}

fn addresses_add(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let address: String = call.req(engine_state, stack, 0)?;
    let duration = call.get_flag(engine_state, stack, "duration")?;

    debug!("Running allowlists add for {}", &address);

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(cluster_not_found_error(identifier, call.span()));
            }
        };
        validate_is_cloud(
            active_cluster,
            "allowlists can only be used with clusters registered to a Capella organisation",
        )?;

        let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
        let cloud = guard
            .capella_org_for_cluster(active_cluster.capella_org().unwrap())?
            .client();
        let cluster = cloud.find_cluster(identifier.clone(), deadline, ctrl_c.clone())?;

        if cluster.environment() == CapellaEnvironment::Hosted {
            return Err(ShellError::UnsupportedInput(
                "allowlists add cannot be run against hosted Capella clusters".into(),
                call.span(),
            ));
        }

        let rule_type = if duration.is_some() {
            "temporary"
        } else {
            "permanent"
        };

        let entry = JSONCloudAppendAllowListRequest::new(
            address.clone(),
            rule_type.to_string(),
            duration.clone(),
        );
        let response = cloud.capella_request(
            CapellaRequest::CreateAllowListEntry {
                cluster_id: cluster.id(),
                payload: serde_json::to_string(&entry)
                    .map_err(|e| json_parse_fail_error(e, Some(call.span())))?,
            },
            deadline,
            ctrl_c.clone(),
        )?;

        match response.status() {
            202 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    Some(call.span()),
                ));
            }
        };
    }
    Ok(PipelineData::new(call.head))
}
