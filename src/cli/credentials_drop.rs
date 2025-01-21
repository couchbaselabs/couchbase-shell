use crate::cli::util::{
    cluster_from_conn_str, cluster_identifiers_from, find_org_id, find_project_id,
    get_active_cluster,
};
use crate::cli::{client_error_to_shell_error, generic_error};
use crate::state::State;
use nu_engine::CallExt;
use nu_protocol::engine::{Call, Command, EngineState, Stack};
use nu_protocol::{Category, PipelineData, ShellError, Signature, SyntaxShape};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct CredentialsDrop {
    state: Arc<Mutex<State>>,
}

impl CredentialsDrop {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for CredentialsDrop {
    fn name(&self) -> &str {
        "credentials drop"
    }

    fn signature(&self) -> Signature {
        Signature::build("credentials drop")
            .category(Category::Custom("couchbase".to_string()))
            .required(
                "credentials ID",
                SyntaxShape::String,
                "the id of the credentials to delete",
            )
            .named(
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
        credentials_drop(self.state.clone(), engine_state, stack, call, input)
    }
}

fn credentials_drop(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let span = call.head;
    let signals = engine_state.signals().clone();

    let cluster_identifiers = cluster_identifiers_from(engine_state, stack, &state, call, true)?;
    if cluster_identifiers.len() != 1 {
        return Err(generic_error(
            "multiple clusters specified",
            "Credentials drop can only be used against one cluster at a time".to_string(),
            span,
        ));
    }

    let credential_id = match call.opt(engine_state, stack, 0)? {
        Some(c_id) => Ok(c_id),
        None => Err(generic_error(
            "missing credential ID",
            "The ID of the credential to be deleted is required, use `credentials` command to see credential IDs".to_string(),
            span,
        ))
    }?;

    let guard = state.lock().unwrap();
    let cluster = get_active_cluster(cluster_identifiers[0].clone(), &guard, span)?;

    let org = guard.named_or_active_org(cluster.capella_org())?;

    let client = org.client();

    let org_id = find_org_id(signals.clone(), &client, span)?;

    let project_id = find_project_id(
        signals.clone(),
        guard.active_project().unwrap(),
        &client,
        span,
        org_id.clone(),
    )?;

    let json_cluster = cluster_from_conn_str(
        cluster_identifiers[0].clone(),
        signals.clone(),
        cluster.hostnames().clone(),
        &client,
        span,
        org_id.clone(),
        project_id.clone(),
    )?;

    client
        .drop_credentials(
            org_id,
            project_id,
            json_cluster.id(),
            credential_id,
            signals.clone(),
        )
        .map_err(|e| client_error_to_shell_error(e, span))?;

    Ok(PipelineData::empty())
}
