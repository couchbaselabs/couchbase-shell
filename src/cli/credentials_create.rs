use crate::cli::util::{cluster_from_conn_str, find_org_id, find_project_id};
use crate::cli::{client_error_to_shell_error, no_active_cluster_error};
use crate::client::cloud_json::CredentialsCreateRequest;
use crate::read_input;
use crate::state::State;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature};
use std::ops::Add;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone)]
pub struct CredentialsCreate {
    state: Arc<Mutex<State>>,
}

impl crate::cli::CredentialsCreate {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for crate::cli::CredentialsCreate {
    fn name(&self) -> &str {
        "credentials create"
    }

    fn signature(&self) -> Signature {
        Signature::build("credentials create").category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Creates credentials on a Capella cluster"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        credentials_create(self.state.clone(), engine_state, stack, call, input)
    }
}

fn credentials_create(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    _stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let (name, password) = {
        let guard = state.lock().unwrap();
        let active_cluster = match guard.active_cluster() {
            Some(c) => c,
            None => {
                return Err(no_active_cluster_error(span));
            }
        };
        let org = if let Some(cluster_org) = active_cluster.capella_org() {
            guard.get_capella_org(cluster_org)
        } else {
            guard.active_capella_org()
        }?;

        let client = org.client();
        let deadline = Instant::now().add(org.timeout());

        let org_id = find_org_id(ctrl_c.clone(), &client, deadline, span)?;

        let project_id = find_project_id(
            ctrl_c.clone(),
            guard.active_project().unwrap(),
            &client,
            deadline,
            span,
            org_id.clone(),
        )?;

        let json_cluster = cluster_from_conn_str(
            active_cluster.display_name().unwrap_or("".to_string()),
            ctrl_c.clone(),
            active_cluster.hostnames().clone(),
            &client,
            deadline,
            span,
            org_id.clone(),
            project_id.clone(),
        )?;

        if json_cluster.state() != "healthy" {
            return Err(ShellError::GenericError {
                error: "Cannot create credentials until cluster state is healthy".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: vec![],
            });
        }

        println!("Please enter username:");
        let name = match read_input() {
            Some(user) => user,
            None => {
                return Err(ShellError::GenericError {
                    error: "Username required".to_string(),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: vec![],
                })
            }
        };

        let password = match rpassword::prompt_password("Password: ") {
            Ok(p) => p,
            Err(_) => {
                return Err(ShellError::GenericError {
                    error: "Password required".to_string(),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: vec![],
                });
            }
        };

        let payload = CredentialsCreateRequest::new(name.clone(), password.clone());

        client
            .create_credentials(
                org_id,
                project_id,
                json_cluster.id(),
                serde_json::to_string(&payload).unwrap(),
                deadline,
                ctrl_c,
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;

        (name, password)
    };

    let guard = &mut state.lock().unwrap();
    let active_cluster = guard.active_mut_cluster().unwrap();
    active_cluster.set_username(name);
    active_cluster.set_password(password);

    Ok(PipelineData::empty())
}
