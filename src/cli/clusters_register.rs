use crate::cli::util::parse_optional_as_bool;
use crate::config::ClusterTlsConfig;
use crate::state::{ClusterTimeouts, RemoteCluster, State};
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_stream::OutputStream;
use std::sync::{Arc, Mutex};

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
                "tls-accept-all-hosts",
                SyntaxShape::String,
                "whether or not to accept all hosts with tls, defaults to false",
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
    let args = args.evaluate_once()?;

    let identifier = match args.call_info.args.get("identifier") {
        Some(v) => match v.as_string() {
            Ok(name) => name,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("identifier is required")),
    };
    let hostnames = match args.call_info.args.get("hostnames") {
        Some(v) => match v.as_string() {
            Ok(name) => name.split(',').map(|v| v.to_owned()).collect(),
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("hostnames is required")),
    };
    let username = match args.call_info.args.get("username") {
        Some(v) => match v.as_string() {
            Ok(name) => name,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("username is required")),
    };
    let password = match args.call_info.args.get("password") {
        Some(v) => match v.as_string() {
            Ok(name) => name,
            Err(e) => return Err(e),
        },
        None => return Err(ShellError::unexpected("password is required")),
    };
    let bucket = match args.call_info.args.get("default-bucket") {
        Some(v) => match v.as_string() {
            Ok(name) => Some(name),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let scope = match args.call_info.args.get("default-scope") {
        Some(v) => match v.as_string() {
            Ok(name) => Some(name),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let collection = match args.call_info.args.get("default-collection") {
        Some(v) => match v.as_string() {
            Ok(name) => Some(name),
            Err(e) => return Err(e),
        },
        None => None,
    };
    let tls_enabled = parse_optional_as_bool(&args.call_info.args, "tls-enabled", true)?;
    let tls_accept_all_certs =
        parse_optional_as_bool(&args.call_info.args, "tls-accept-all-certs", true)?;
    let tls_accept_all_hosts =
        parse_optional_as_bool(&args.call_info.args, "tls-accept-all-hosts", true)?;
    let cert_path = match args.call_info.args.get("tls-cert-path") {
        Some(v) => match v.as_string() {
            Ok(name) => Some(name),
            Err(e) => return Err(e),
        },
        None => None,
    };

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
    );

    let mut guard = state.lock().unwrap();
    guard.add_cluster(identifier, cluster)?;

    Ok(OutputStream::empty())
}
