use crate::config::{CapellaOrganizationConfig, ClusterConfig, ShellConfig, DEFAULT_KV_BATCH_SIZE};
use crate::state::State;
use std::fs;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::cli::error::generic_error;
use crate::cli::util::get_username_and_password;
use crate::{
    ClusterTimeouts, RemoteCluster, RemoteClusterResources, RemoteClusterType, RustTlsConfig,
};
use nu_engine::CallExt;
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::Value::Nothing;
use nu_protocol::{Category, PipelineData, ShellError, Signature, Span, SyntaxShape};

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
    let save = call.get_flag(engine_state, stack, "save")?.unwrap_or(false);
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
    guard.add_cluster(identifier, cluster)?;

    if save {
        update_config_file(&mut guard, call.head)?;
    }

    Ok(PipelineData::Value(
        Nothing {
            internal_span: call.head,
        },
        None,
    ))
}

pub fn update_config_file(guard: &mut MutexGuard<State>, span: Span) -> Result<(), ShellError> {
    let path = match guard.config_path() {
        Some(p) => p,
        None => {
            return Err(generic_error(
                "A config path must be discoverable to save config",
                None,
                span,
            ));
        }
    };
    let mut cluster_configs = Vec::new();
    for (identifier, cluster) in guard.clusters() {
        cluster_configs.push(ClusterConfig::from((identifier.clone(), cluster)))
    }
    let mut capella_configs = Vec::new();
    for (identifier, c) in guard.capella_orgs() {
        capella_configs.push(CapellaOrganizationConfig::new(
            identifier.clone(),
            c.secret_key(),
            c.access_key(),
            Some(c.timeout()),
            c.default_project(),
            None,
        ))
    }

    let config = ShellConfig::new_from_clusters(cluster_configs, capella_configs);

    fs::write(
        path,
        config
            .to_str()
            .map_err(|e| generic_error(format!("Failed to write config file {}", e), None, span))?,
    )
    .map_err(|e| generic_error(format!("Failed to write config file {}", e), None, span))?;

    Ok(())
}
