use crate::state::RemoteCluster;
use log::debug;
use log::error;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use toml::ser::Error;

pub(crate) const DEFAULT_DATA_TIMEOUT: Duration = Duration::from_millis(5000);
pub(crate) const DEFAULT_QUERY_TIMEOUT: Duration = Duration::from_millis(75000);
pub(crate) const DEFAULT_ANALYTICS_TIMEOUT: Duration = Duration::from_millis(75000);
pub(crate) const DEFAULT_SEARCH_TIMEOUT: Duration = Duration::from_millis(75000);
pub(crate) const DEFAULT_MANAGEMENT_TIMEOUT: Duration = Duration::from_millis(75000);

/// Holds the complete config in an aggregated manner.
#[derive(Debug, Deserialize, Serialize)]
pub struct ShellConfig {
    version: usize,
    /// Note: clusters is kept for backwards compatibility and
    /// convenience, docs should only mention cluster
    #[serde(alias = "cluster")]
    #[serde(alias = "clusters")]
    clusters: Vec<ClusterConfig>,

    #[serde(alias = "cloud-control-pane", default)]
    control_pane: Option<CloudControlPaneConfig>,

    #[serde(alias = "cloud", default)]
    clouds: Vec<CloudConfig>,

    /// Stores the path from which it got loaded, if present
    path: Option<PathBuf>,
}

impl ShellConfig {
    /// Tries to load the configuration from different paths.
    ///
    /// It first tries the `.cbsh/config` in the current directory, and if not found there
    /// it then tries the home directory (so `~/.cbsh/config`).
    pub fn new() -> Self {
        let mut config = try_config_from_path(std::env::current_dir().unwrap())
            .or_else(|| try_config_from_path(dirs::home_dir().unwrap()))
            .unwrap_or_default();

        let standalone_credentials = try_credentials_from_path(std::env::current_dir().unwrap())
            .or_else(|| try_credentials_from_path(dirs::home_dir().unwrap()));

        if let Some(standalone) = standalone_credentials {
            for value in config.clusters_mut() {
                let identifier = value.identifier().to_owned();
                let mut config_credentials = value.credentials_mut();

                for cred in &standalone.clusters {
                    if cred.identifier() == identifier {
                        if config_credentials.username.is_none() && cred.username.is_some() {
                            config_credentials.username = cred.username.clone()
                        }
                        if config_credentials.password.is_none() && cred.password.is_some() {
                            config_credentials.password = cred.password.clone()
                        }
                    }
                }
            }

            if let Some(value) = config.control_pane_mut() {
                let config_credentials = value.credentials_mut();

                if let Some(cred) = &standalone.control_pane {
                    if config_credentials.secret_key.is_empty() && !cred.secret_key.is_empty() {
                        config_credentials.secret_key = cred.secret_key.clone()
                    }
                    if config_credentials.access_key.is_empty() && !cred.access_key.is_empty() {
                        config_credentials.access_key = cred.access_key.clone()
                    }
                }
            }
        }

        config
    }

    pub fn new_from_clusters(
        clusters: Vec<ClusterConfig>,
        clouds: Vec<CloudConfig>,
        control_pane: Option<CloudControlPaneConfig>,
    ) -> Self {
        Self {
            clusters,
            path: None,
            version: 1,
            clouds,
            control_pane,
        }
    }

    pub fn location(&self) -> &Option<PathBuf> {
        &self.path
    }

    /// Builds the config from a raw input string.
    pub fn from_str(input: &str) -> Self {
        // Note: ideally this propagates up into a central error handling facility,
        // but for now just logging it nicely and bailing out is probably goint to be fine.
        match toml::from_str(input) {
            Ok(i) => i,
            Err(e) => {
                error!("Failed to parse config file: {}", e);
                std::process::exit(-1);
            }
        }
    }

    pub fn to_str(&self) -> Result<String, Error> {
        toml::to_string(self)
    }

    /// Returns the individual configurations for all the clusters configured.
    pub fn clusters(&self) -> &Vec<ClusterConfig> {
        &self.clusters
    }

    pub fn clusters_mut(&mut self) -> &mut Vec<ClusterConfig> {
        &mut self.clusters
    }

    pub fn control_pane(&self) -> Option<&CloudControlPaneConfig> {
        self.control_pane.as_ref()
    }

    pub fn control_pane_mut(&mut self) -> &mut Option<CloudControlPaneConfig> {
        &mut self.control_pane
    }

    pub fn clouds(&self) -> &Vec<CloudConfig> {
        &self.clouds
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            clusters: vec![],
            version: 1,
            path: None,
            clouds: vec![],
            control_pane: None,
        }
    }
}

fn try_config_from_path(mut path: PathBuf) -> Option<ShellConfig> {
    path.push(".cbsh");
    path.push("config");

    let read = fs::read_to_string(&path);
    match read {
        Ok(r) => {
            let mut conf = ShellConfig::from_str(&r);
            conf.path = Some(path);
            Some(conf)
        }
        Err(e) => {
            debug!("Could not locate {:?} because of {:?}", path, e);
            None
        }
    }
}

fn try_credentials_from_path(mut path: PathBuf) -> Option<StandaloneCredentialsConfig> {
    path.push(".cbsh");
    path.push("credentials");

    let read = fs::read_to_string(&path);
    match read {
        Ok(r) => Some(StandaloneCredentialsConfig::from_str(&r)),
        Err(e) => {
            debug!("Could not locate {:?} becaue of {:?}", path, e);
            None
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CloudControlPaneConfig {
    #[serde(flatten)]
    credentials: CloudControlPaneCredentials,
    #[serde(default)]
    #[serde(
        rename(deserialize = "management-timeout", serialize = "management-timeout"),
        with = "humantime_serde"
    )]
    management_timeout: Option<Duration>,
}

impl CloudControlPaneConfig {
    pub fn new(
        secret_key: String,
        access_key: String,
        management_timeout: Option<Duration>,
    ) -> Self {
        Self {
            credentials: CloudControlPaneCredentials {
                access_key,
                secret_key,
            },
            management_timeout,
        }
    }
    pub fn secret_key(&self) -> String {
        self.credentials.secret_key.clone()
    }
    pub fn access_key(&self) -> String {
        self.credentials.access_key.clone()
    }
    pub fn management_timeout(&self) -> Option<&Duration> {
        self.management_timeout.as_ref()
    }

    pub fn credentials_mut(&mut self) -> &mut CloudControlPaneCredentials {
        &mut self.credentials
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClusterConfig {
    identifier: String,
    hostnames: Vec<String>,
    #[serde(rename(deserialize = "default-bucket", serialize = "default-bucket"))]
    default_bucket: Option<String>,
    #[serde(rename(deserialize = "default-scope", serialize = "default-scope"))]
    default_scope: Option<String>,
    #[serde(rename(deserialize = "default-collection", serialize = "default-collection"))]
    default_collection: Option<String>,

    #[serde(flatten)]
    credentials: ClusterCredentials,
    #[serde(flatten)]
    timeouts: ClusterConfigTimeouts,
    #[serde(flatten)]
    tls: ClusterTlsConfig,

    #[serde(default)]
    #[serde(rename(deserialize = "cloud", serialize = "cloud"))]
    cloud: bool,
}

impl ClusterConfig {
    pub fn identifier(&self) -> &str {
        self.identifier.as_ref()
    }

    pub fn hostnames(&self) -> &Vec<String> {
        &self.hostnames
    }
    pub fn username(&self) -> String {
        if let Some(u) = &self.credentials.username {
            return u.clone();
        }
        error!(
            "No username found in config or credentials file for identifier \"{}\"!",
            self.identifier
        );
        std::process::exit(-1);
    }
    pub fn password(&self) -> String {
        if let Some(p) = &self.credentials.password {
            return p.clone();
        }
        error!(
            "No password found in config or credentials file for identifier \"{}\"!",
            self.identifier
        );
        std::process::exit(-1);
    }
    pub fn default_bucket(&self) -> Option<String> {
        self.default_bucket.as_ref().cloned()
    }
    pub fn default_scope(&self) -> Option<String> {
        self.default_scope.as_ref().cloned()
    }
    pub fn default_collection(&self) -> Option<String> {
        self.default_collection.as_ref().cloned()
    }
    pub fn credentials_mut(&mut self) -> &mut ClusterCredentials {
        &mut self.credentials
    }
    pub fn timeouts(&self) -> &ClusterConfigTimeouts {
        &self.timeouts
    }
    pub fn tls(&self) -> &ClusterTlsConfig {
        &self.tls
    }
    pub fn cloud(&self) -> bool {
        self.cloud
    }
}

impl From<(String, &RemoteCluster)> for ClusterConfig {
    fn from(cluster: (String, &RemoteCluster)) -> Self {
        let cloud = cluster.1.cloud();

        Self {
            identifier: cluster.0,
            hostnames: cluster.1.hostnames().clone(),
            default_collection: cluster.1.active_collection(),
            default_scope: cluster.1.active_scope(),
            default_bucket: cluster.1.active_bucket(),
            timeouts: ClusterConfigTimeouts {
                data_timeout: Some(cluster.1.timeouts().data_timeout()),
                query_timeout: Some(cluster.1.timeouts().query_timeout()),
                analytics_timeout: Some(cluster.1.timeouts().analytics_timeout()),
                search_timeout: Some(cluster.1.timeouts().search_timeout()),
                management_timeout: Some(cluster.1.timeouts().management_timeout()),
            },
            tls: cluster.1.tls_config().clone(),
            credentials: ClusterCredentials {
                username: Some(cluster.1.username().to_string()),
                password: Some(cluster.1.password().to_string()),
            },
            cloud,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CloudControlPaneCredentials {
    #[serde(default)]
    #[serde(rename(deserialize = "access-key", serialize = "access-key"))]
    access_key: String,
    #[serde(default)]
    #[serde(rename(deserialize = "secret-key", serialize = "secret-key"))]
    secret_key: String,
}

impl CloudControlPaneCredentials {}

#[derive(Debug, Deserialize, Serialize)]
pub struct CloudConfig {
    identifier: String,
    #[serde(rename(deserialize = "default-project", serialize = "default-project"))]
    default_project: Option<String>,
}

impl CloudConfig {
    pub fn new(identifier: String, default_project: Option<String>) -> Self {
        Self {
            identifier,
            default_project,
        }
    }
    pub fn identifier(&self) -> String {
        self.identifier.clone()
    }
    pub fn default_project(&self) -> Option<String> {
        self.default_project.as_ref().cloned()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClusterCredentials {
    username: Option<String>,
    password: Option<String>,
}

impl ClusterCredentials {}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ClusterConfigTimeouts {
    #[serde(default)]
    #[serde(
        rename(deserialize = "data-timeout", serialize = "data-timeout"),
        with = "humantime_serde"
    )]
    data_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(
        rename(deserialize = "connect-timeout", serialize = "connect-timeout"),
        with = "humantime_serde"
    )]
    query_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(
        rename(deserialize = "search-timeout", serialize = "search-timeout"),
        with = "humantime_serde"
    )]
    search_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(
        rename(deserialize = "analytics-timeout", serialize = "analytics-timeout"),
        with = "humantime_serde"
    )]
    analytics_timeout: Option<Duration>,
    #[serde(default)]
    #[serde(
        rename(deserialize = "management-timeout", serialize = "management-timeout"),
        with = "humantime_serde"
    )]
    management_timeout: Option<Duration>,
}

impl Default for ClusterConfigTimeouts {
    fn default() -> Self {
        ClusterConfigTimeouts {
            data_timeout: Some(DEFAULT_DATA_TIMEOUT),
            query_timeout: Some(DEFAULT_QUERY_TIMEOUT),
            analytics_timeout: Some(DEFAULT_ANALYTICS_TIMEOUT),
            search_timeout: Some(DEFAULT_SEARCH_TIMEOUT),
            management_timeout: Some(DEFAULT_MANAGEMENT_TIMEOUT),
        }
    }
}

impl ClusterConfigTimeouts {
    pub fn data_timeout(&self) -> Option<&Duration> {
        self.data_timeout.as_ref()
    }

    pub fn query_timeout(&self) -> Option<&Duration> {
        self.query_timeout.as_ref()
    }

    pub fn search_timeout(&self) -> Option<&Duration> {
        self.search_timeout.as_ref()
    }

    pub fn analytics_timeout(&self) -> Option<&Duration> {
        self.analytics_timeout.as_ref()
    }

    pub fn management_timeout(&self) -> Option<&Duration> {
        self.management_timeout.as_ref()
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ClusterTlsConfig {
    #[serde(rename(deserialize = "tls-enabled", serialize = "tls-enabled"))]
    #[serde(default = "default_as_true")]
    enabled: bool,
    #[serde(rename(deserialize = "tls-cert-path", serialize = "tls-cert-path"))]
    cert_path: Option<String>,
    #[serde(rename(
        deserialize = "tls-validate-hostnames",
        serialize = "tls-validate-hostnames"
    ))]
    #[serde(default = "default_as_false")]
    validate_hostnames: bool,
    #[serde(rename(
        deserialize = "tls-accept-all-certs",
        serialize = "tls-accept-all-certs"
    ))]
    #[serde(default = "default_as_true")]
    accept_all_certs: bool,
}

impl ClusterTlsConfig {
    pub fn new(
        enabled: bool,
        cert_path: Option<String>,
        validate_hostnames: bool,
        accept_all_certs: bool,
    ) -> Self {
        Self {
            enabled,
            cert_path,
            validate_hostnames,
            accept_all_certs,
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn cert_path(&self) -> &Option<String> {
        &self.cert_path
    }

    pub fn validate_hostnames(&self) -> bool {
        self.validate_hostnames
    }

    pub fn accept_all_certs(&self) -> bool {
        self.accept_all_certs
    }
}

impl Default for ClusterTlsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cert_path: None,
            validate_hostnames: false,
            accept_all_certs: true,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StandaloneCredentialsConfig {
    version: usize,
    /// Note: clusters is kept for backwards compatibility and
    /// convenience, docs should only mention cluster
    #[serde(alias = "cluster")]
    #[serde(alias = "clusters", default)]
    clusters: Vec<StandaloneClusterCredentials>,

    #[serde(alias = "cloud-control-pane", default)]
    control_pane: Option<CloudControlPaneCredentials>,
}

impl StandaloneCredentialsConfig {
    /// Builds the config from a raw input string.
    pub fn from_str(input: &str) -> Self {
        // Note: ideally this propagates up into a central error handling facility,
        // but for now just logging it nicely and bailing out is probably goint to be fine.
        match toml::from_str(input) {
            Ok(i) => i,
            Err(e) => {
                error!("Failed to parse credentials config file: {}", e);
                std::process::exit(-1);
            }
        }
    }
}

impl Default for StandaloneCredentialsConfig {
    fn default() -> Self {
        Self {
            clusters: vec![],
            version: 1,
            control_pane: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StandaloneClusterCredentials {
    identifier: String,
    username: Option<String>,
    password: Option<String>,
}

impl StandaloneClusterCredentials {
    fn identifier(&self) -> &str {
        self.identifier.as_ref()
    }
}

fn default_as_false() -> bool {
    false
}
fn default_as_true() -> bool {
    true
}
