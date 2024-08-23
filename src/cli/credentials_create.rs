use crate::cli::util::{
    cluster_from_conn_str, cluster_identifiers_from, find_org_id, find_project_id,
    get_active_cluster, get_username_and_password,
};
use crate::cli::{client_error_to_shell_error, generic_error};
use crate::client::cloud_json::CredentialsCreateRequest;
use crate::state::State;
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
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
        Signature::build("credentials create")
            .category(Category::Custom("couchbase".to_string()))
            .switch("read", "enable read access", None)
            .switch("write", "enable write access", None)
            .named(
                "username",
                SyntaxShape::String,
                "the username to use for the registered cluster",
                None,
            )
            .named(
                "password",
                SyntaxShape::String,
                "the password to use for the registered cluster",
                None,
            )
            .switch(
                "registered",
                "create credentials with the username/password the active cluster was registered with",
                None,
            ) .named(
            "clusters",
            SyntaxShape::String,
            "the clusters which should be contacted",
            None,
        )
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
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let ctrl_c = engine_state.ctrlc.as_ref().unwrap().clone();

    let read = call.has_flag(engine_state, stack, "read")?;
    let write = call.has_flag(engine_state, stack, "write")?;
    let use_registered = call.has_flag(engine_state, stack, "registered")?;

    if !read && !write {
        return Err(ShellError::GenericError {
            error: "Credentials must have at least read or write access".to_string(),
            msg: "".to_string(),
            span: None,
            help: Some("Use the --read and --write flags to add permissions.".into()),
            inner: vec![],
        });
    }

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    let guard = state.lock().unwrap();

    for identifier in cluster_identifiers {
        let cluster = get_active_cluster(identifier.clone(), &guard, span)?;

        let org = guard.named_or_active_org(cluster.capella_org())?;

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
            identifier,
            ctrl_c.clone(),
            cluster.hostnames().clone(),
            &client,
            deadline,
            span,
            org_id.clone(),
            project_id.clone(),
        )?;

        if json_cluster.state() != "healthy" {
            return Err(generic_error(
                "Cluster not healthy",
                "Cannot create credentials until cluster is healthy. Check the status of the cluster with 'clusters get'".to_string(),
                span
            ));
        }

        let (name, password) = if use_registered {
            (
                cluster.username().to_string(),
                cluster.password().to_string(),
            )
        } else {
            let username_flag = call.get_flag(engine_state, stack, "username")?;
            let password_flag = call.get_flag(engine_state, stack, "password")?;
            get_username_and_password(username_flag, password_flag)?
        };

        let payload = CredentialsCreateRequest::new(name.clone(), password.clone(), read, write);

        client
            .create_credentials(
                org_id,
                project_id,
                json_cluster.id(),
                serde_json::to_string(&payload).unwrap(),
                deadline,
                ctrl_c.clone(),
            )
            .map_err(|e| client_error_to_shell_error(e, span))?;
    }

    Ok(PipelineData::empty())
}
