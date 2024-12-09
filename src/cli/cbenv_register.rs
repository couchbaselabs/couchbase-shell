use crate::cli::error::generic_error;
use crate::cli::util::{get_username_and_password, read_config_file, update_config_file};
use crate::config::{ClusterConfig, DEFAULT_KV_BATCH_SIZE};
use crate::state::State;
use crate::{
    ClusterTimeouts, RemoteCluster, RemoteClusterResources, RemoteClusterType, RustTlsConfig,
};
use nu_engine::command_prelude::Call;
use nu_engine::CallExt;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Value::Nothing;
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, SyntaxShape};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub struct CbEnvRegister {
    state: Arc<Mutex<State>>,
}

impl CbEnvRegister {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl Command for CbEnvRegister {
    fn name(&self) -> &str {
        "cb-env register"
    }

    fn signature(&self) -> Signature {
        Signature::build("cb-env register")
            .required(
                "identifier",
                SyntaxShape::String,
                "the identifier to use for this cluster",
            )
            .required(
                "connstr",
                SyntaxShape::String,
                "the connection string to use for this cluster",
            )
            .named(
                "username",
                SyntaxShape::String,
                "the username to use for the registered cluster",
                None,
            )
            .named(
                "password",
                SyntaxShape::String,
                "the password to use with the registered cluster",
                None,
            )
            .named(
                "display_name",
                SyntaxShape::String,
                "the display name to use for the user when this cluster is active",
                None,
            )
            .named(
                "default-bucket",
                SyntaxShape::String,
                "the default bucket to use with this cluster",
                None,
            )
            .named(
                "default-scope",
                SyntaxShape::String,
                "the default scope to use with this cluster",
                None,
            )
            .named(
                "default-collection",
                SyntaxShape::String,
                "the default collection to use with this cluster",
                None,
            )
            .named(
                "tls-enabled",
                SyntaxShape::String,
                "whether or not to enable tls, defaults to true",
                None,
            )
            .named(
                "tls-cert-path",
                SyntaxShape::String,
                "the path to the certificate to use with tls",
                None,
            )
            .named(
                "tls-accept-all-certs",
                SyntaxShape::String,
                "whether or not to accept all certs with tls, defaults to true",
                None,
            )
            .switch(
                "save",
                "whether or not to add the cluster to the .cbsh config file, defaults to false",
                None,
            )
            .named(
                "capella-organization",
                SyntaxShape::String,
                "capella organization that this cluster belongs to",
                None,
            )
            .named(
                "project",
                SyntaxShape::String,
                "project that this cluster belongs to",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Registers a cluster for use with the shell"
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        clusters_register(self.state.clone(), engine_state, stack, call, input)
    }
}

fn clusters_register(
    state: Arc<Mutex<State>>,
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let identifier: String = call.req(engine_state, stack, 0)?;

    let conn_string: String = call.req(engine_state, stack, 1)?;
    let bucket = call.get_flag(engine_state, stack, "default-bucket")?;
    let scope = call.get_flag(engine_state, stack, "default-scope")?;
    let collection = call.get_flag(engine_state, stack, "default-collection")?;
    let tls_enabled = call
        .get_flag(engine_state, stack, "tls-enabled")?
        .unwrap_or(true);
    let tls_accept_all_certs = call
        .get_flag(engine_state, stack, "tls-accept-all-certs")?
        .unwrap_or(true);
    let cert_path = call.get_flag(engine_state, stack, "tls-cert-path")?;
    let save = call.has_flag(engine_state, stack, "save")?;
    let capella = call.get_flag(engine_state, stack, "capella-organization")?;
    let project = call.get_flag(engine_state, stack, "project")?;
    let display_name = call.get_flag(engine_state, stack, "display-name")?;

    let hostnames = conn_string
        .split(',')
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    let tls_config = if tls_enabled {
        Some(
            RustTlsConfig::new(tls_accept_all_certs, cert_path)
                .map_err(|e| generic_error(e.message(), e.expanded_message(), None))?,
        )
    } else {
        None
    };

    let username_flag = call.get_flag(engine_state, stack, "username")?;
    let password_flag = call.get_flag(engine_state, stack, "password")?;

    let (username, password) = get_username_and_password(username_flag, password_flag)?;

    let cluster = RemoteCluster::new(
        RemoteClusterResources {
            hostnames: hostnames.clone(),
            username,
            password,
            active_bucket: bucket,
            active_scope: scope,
            active_collection: collection,
            display_name,
        },
        tls_config,
        ClusterTimeouts::default(),
        capella,
        project,
        DEFAULT_KV_BATCH_SIZE,
        RemoteClusterType::from(hostnames),
    );

    let mut guard = state.lock().unwrap();
    guard.add_cluster(identifier.clone(), cluster)?;

    if save {
        save_new_cluster_config(&mut guard, call.head, identifier)?;
    }

    Ok(PipelineData::Value(
        Nothing {
            internal_span: call.head,
        },
        None,
    ))
}

fn save_new_cluster_config(
    guard: &mut MutexGuard<State>,
    span: Span,
    identifier: String,
) -> Result<(), ShellError> {
    let mut config = read_config_file(guard, span)?;
    let clusters = config.clusters_mut();

    if clusters.iter().any(|c| c.identifier() == identifier) {
        return Err(generic_error(
            format!(
                "failed to update config file: cluster with identifier {} already exists",
                identifier
            ),
            None,
            span,
        ));
    }

    let new_cluster = guard.clusters().get(&identifier).unwrap();

    clusters.push(ClusterConfig::from((identifier.clone(), new_cluster)));

    update_config_file(guard, span, config)
}
