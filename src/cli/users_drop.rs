use crate::cli::util::{cluster_identifiers_from, get_active_cluster};
use crate::client::{CapellaRequest, ManagementRequest};
use crate::state::{CapellaEnvironment, State};
use log::debug;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

use crate::cli::error::{cant_run_against_hosted_capella_error, unexpected_status_code_error};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};

#[derive(Clone)]
pub struct UsersDrop {
    state: Arc<Mutex<State>>,
}

impl UsersDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for UsersDrop {
    fn name(&self) -> &str {
        "users drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("users drop")
            .required("username", SyntaxShape::String, "the username of the user")
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
            .category(Category::Custom("couchbase".into()))
    }

    fn usage(&self) -> &str {
        "Deletes a user"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        users_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn users_drop(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();
    let username: String = call.req(engine_state, stack, 0)?;

    debug!("Running users drop {}", username);

    let cluster_identifiers = cluster_identifiers_from(&engine_state, stack, &state, &call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let active_cluster = get_active_cluster(identifier.clone(), &guard, span.clone())?;

        let response = if let Some(plane) = active_cluster.capella_org() {
            let cloud = guard.capella_org_for_cluster(plane)?.client();
            let deadline = Instant::now().add(active_cluster.timeouts().management_timeout());
            let cluster =
                cloud.find_cluster(identifier.clone(), deadline.clone(), ctrl_c.clone())?;

            if cluster.environment() == CapellaEnvironment::Hosted {
                return Err(cant_run_against_hosted_capella_error("users drop", span));
            }

            cloud.capella_request(
                CapellaRequest::DeleteUser {
                    cluster_id: cluster.id(),
                    username: username.clone(),
                },
                deadline,
                ctrl_c.clone(),
            )?
        } else {
            active_cluster.cluster().http_client().management_request(
                ManagementRequest::DropUser {
                    username: username.clone(),
                },
                Instant::now().add(active_cluster.timeouts().management_timeout()),
                ctrl_c.clone(),
            )?
        };

        match response.status() {
            200 => {}
            204 => {}
            _ => {
                return Err(unexpected_status_code_error(
                    response.status(),
                    response.content(),
                    call.span(),
                ));
            }
        }
    }

    Ok(PipelineData::new(span))
}
