use crate::cli::client_error_to_shell_error;
use crate::cli::util::{
    cluster_from_conn_str, cluster_identifiers_from, find_org_id, find_project_id,
    get_active_cluster,
};
use crate::state::State;
use log::{debug, info};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape, Value};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AllowIP {
    state: Arc<Mutex<State>>,
}

impl crate::cli::AllowIP {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for crate::cli::AllowIP {
    fn name(&self) -> &str {
        "allow ip"
    }

    fn signature(&self) -> Signature {
        Signature::build("allow ip")
            .category(Category::Custom("couchbase".to_string()))
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .optional(
                "address",
                SyntaxShape::String,
                "ip address to allow access to the cluster",
            )
    }

    fn usage(&self) -> &str {
        "Adds IP address to allowlist on a Capella cluster"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        allow_ip(self.state.clone(), engine_state, stack, call, input)
    }
}

fn allow_ip(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    debug!("Running allow_ip");

    let ip_address = match input.into_value(span)? {
        Value::String { val, .. } => format_ip_address(val),
        Value::Nothing { .. } => {
            if let Some(address) = call.opt(engine_state, stack, 0)? {
                format_ip_address(address)
            } else {
                return Err(ShellError::GenericError {
                    error: "No IP address provided".to_string(),
                    msg: "".to_string(),
                    span: None,
                    help: Some("Provide IP as positional parameter or piped input".into()),
                    inner: vec![],
                });
            }
        }
        _ => {
            return Err(ShellError::GenericError {
                error: "IP address must be a string".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: vec![],
            })
        }
    };

    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let org = guard.named_or_active_org(cluster.capella_org())?;
        let client = org.client();

        let org_id = find_org_id(ctrl_c.clone(), &client, span)?;

        let project_id = find_project_id(
            ctrl_c.clone(),
            guard.named_or_active_project(cluster.project())?,
            &client,
            span,
            org_id.clone(),
        )?;

        let json_cluster = cluster_from_conn_str(
            identifier,
            ctrl_c.clone(),
            cluster.hostnames().clone(),
            &client,
            span,
            org_id.clone(),
            project_id.clone(),
        )?;

        client
            .allow_ip_address(
                org_id,
                project_id,
                json_cluster.id(),
                ip_address.clone(),
                ctrl_c.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;
    }

    Ok(PipelineData::empty())
}

fn format_ip_address(ip_address: String) -> String {
    if !ip_address.contains('/') {
        info!("IP address supplied without a subnet mask, defaulting to '/32'");
        format!("{}/32", ip_address)
    } else {
        ip_address
    }
}
