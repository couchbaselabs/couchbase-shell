use crate::config::{CloudConfig, ClusterConfig, ClusterTlsConfig, ShellConfig};
use crate::state::{ClusterTimeouts, RemoteCluster, State};
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::fs;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct ClustersRegister {
    state: Arc<Mutex<State>>,
}

impl ClustersRegister {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

impl nu_engine::WholeStreamCommand for ClustersRegister {
    fn name(&self) -> &str {
        "clusters register"
    }

    fn signature(&self) -> Signature {
        Signature::build("clusters register")
            .required_named(
                "identifier",
                SyntaxShape::String,
                "the identifier to use for this cluster",
                None,
            )
            .required_named(
                "hostnames",
                SyntaxShape::String,
                "the comma separated list of hosts to use for this cluster",
                None,
            )
            .required_named(
                "username",
                SyntaxShape::String,
                "the username use for this cluster",
                None,
            )
            .required_named(
                "password",
                SyntaxShape::String,
                "the password to use for this cluster",
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
            .named(
                "tls-validate-hosts",
                SyntaxShape::String,
                "whether or not to validate hosts with tls, defaults to false",
                None,
            )
            .switch(
                "save",
                "whether or not to add the cluster to the .cbsh config file, defaults to false",
                None,
            )
            .named(
                "cloud",
                SyntaxShape::String,
                "the name of the cloud control pane to use this cluster",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Registers a cluster for use with the shell"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        clusters_register(args, self.state.clone())
    }
}

fn clusters_register(
    args: CommandArgs,
    state: Arc<Mutex<State>>,
) -> Result<OutputStream, ShellError> {
    let identifier: String = args.req_named("identifier")?;

    let hostnames = args
        .req_named::<String>("hostnames")?
        .split(',')
        .map(|v| v.to_owned())
        .collect();
    let username = args.req_named("username")?;
    let password = args.req_named("password")?;
    let bucket = args.get_flag("default-bucket")?;
    let scope = args.get_flag("default-scope")?;
    let collection = args.get_flag("default-collection")?;
    let tls_enabled = args.get_flag("tls-enabled")?.unwrap_or(true);
    let tls_accept_all_certs = args.get_flag("tls-accept-all-certs")?.unwrap_or(true);
    let tls_accept_all_hosts = args.get_flag("tls-validate-hosts")?.unwrap_or(true);
    let cert_path = args.get_flag("tls-cert-path")?;
    let save = args.get_flag("save")?.unwrap_or(false);
    let cloud = args.get_flag("cloud")?;

    let cluster = RemoteCluster::new(
        hostnames,
        username,
        password,
        bucket,
        scope,
        collection,
        ClusterTlsConfig::new(
            tls_enabled,
            cert_path,
            tls_accept_all_certs,
            tls_accept_all_hosts,
        ),
        ClusterTimeouts::default(),
        cloud,
    );

    let mut guard = state.lock().unwrap();
    guard.add_cluster(identifier, cluster)?;

    if save {
        update_config_file(&mut guard)?;
    }

    Ok(OutputStream::empty())
}

pub fn update_config_file(guard: &mut MutexGuard<State>) -> Result<(), ShellError> {
    let path = match guard.config_path() {
        Some(p) => p,
        None => {
            return Err(ShellError::unexpected(
                "A config path must be discoverable to save config",
            ));
        }
    };
    let mut cluster_configs = Vec::new();
    for (identifier, cluster) in guard.clusters() {
        cluster_configs.push(ClusterConfig::from((identifier.clone(), cluster)))
    }
    let mut cloud_configs = Vec::new();
    for (identifier, cloud) in guard.clouds() {
        cloud_configs.push(CloudConfig::new(
            identifier.clone(),
            cloud.secret_key(),
            cloud.access_key(),
        ))
    }

    let config = ShellConfig::new_from_clusters(cluster_configs, cloud_configs);

    fs::write(
        path,
        config
            .to_str()
            .map_err(|e| ShellError::unexpected(e.to_string()))?,
    )
    .map_err(|e| ShellError::unexpected(e.to_string()))?;

    Ok(())
}
