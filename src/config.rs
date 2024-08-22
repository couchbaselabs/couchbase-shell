use crate::remote_cluster::{RemoteCluster, RemoteClusterType};
use crate::state::Provider;
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
pub(crate) const DEFAULT_TRANSACTION_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const DEFAULT_KV_BATCH_SIZE: u32 = 500;

/// Holds the complete config in an aggregated manner.
#[derive(Debug, Deserialize, Serialize)]
pub struct ShellConfig {
    version: usize,

    /// Stores the path from which it got loaded, if present
    #[serde(skip)]
    path: Option<PathBuf>,

    /// Note: clusters and database are kept for backwards compatibility and
    /// convenience, docs should only mention cluster
    #[serde(alias = "cluster", default)]
    #[serde(alias = "database")]
    #[serde(alias = "clusters")]
    #[serde(rename(serialize = "cluster"))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    clusters: Vec<ClusterConfig>,

    #[serde(alias = "capella-organization", default)]
    #[serde(alias = "capella-organisation")]
    #[serde(rename(serialize = "capella-organization"))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    capella_orgs: Vec<CapellaOrganizationConfig>,

    #[serde(alias = "llm", default)]
    llms: Vec<LLMConfig>,
}

impl ShellConfig {
    /// Tries to load the configuration from different paths.
    ///
    /// It first tries the `.cbsh/config` in the current directory, and if not found there
    /// it then tries the home directory (so `~/.cbsh/config`).
    pub fn new(config_path: Option<PathBuf>) -> Option<Self> {
        let (mut config, standalone_credentials) = if let Some(p) = config_path {
            let config = try_config_from_path(p.clone())
                .expect("Config file could not be loaded from specific path");

            let standalone_credentials = try_credentials_from_path(p);

            (config, standalone_credentials)
        } else {
            let config = match try_config_from_dot_path(std::env::current_dir().unwrap())
                .or_else(|| try_config_from_dot_path(dirs::home_dir().unwrap()))
            {
                Some(c) => c,
                None => return None,
            };

            let standalone_credentials =
                try_credentials_from_dot_path(std::env::current_dir().unwrap())
                    .or_else(|| try_credentials_from_dot_path(dirs::home_dir().unwrap()));
            (config, standalone_credentials)
        };

        if let Some(standalone) = standalone_credentials {
            for value in config.clusters_mut() {
                let identifier = value.identifier().to_owned();
                let config_credentials = value.credentials_mut();

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

            for value in config.capella_orgs_mut() {
                let identifier = value.identifier().to_owned();
                let config_credentials = value.credentials_mut();

                for cred in &standalone.capella_orgs {
                    if cred.identifier() == identifier {
                        if config_credentials.secret_key.is_empty() && !cred.secret_key.is_empty() {
                            config_credentials.secret_key = cred.secret_key.clone()
                        }
                        if config_credentials.access_key.is_empty() && !cred.access_key.is_empty() {
                            config_credentials.access_key = cred.access_key.clone()
                        }
                    }
                }
            }

            for llm in config.llms_mut() {
                for creds in &standalone.llms {
                    if llm.identifier == creds.identifier
                        && llm.api_key.is_none()
                        && !creds.api_key.is_empty()
                    {
                        llm.api_key = Some(creds.api_key.clone())
                    }
                }
            }
        }

        Some(config)
    }

    pub fn new_from_clusters(
        clusters: Vec<ClusterConfig>,
        capella_orgs: Vec<CapellaOrganizationConfig>,
    ) -> Self {
        Self {
            clusters,
            path: None,
            version: 1,
            capella_orgs,
            llms: vec![],
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

    pub fn capella_orgs(&self) -> &Vec<CapellaOrganizationConfig> {
        &self.capella_orgs
    }

    pub fn capella_orgs_mut(&mut self) -> &mut Vec<CapellaOrganizationConfig> {
        &mut self.capella_orgs
    }

    pub fn llms(&self) -> &Vec<LLMConfig> {
        &self.llms
    }

    pub fn llms_mut(&mut self) -> &mut Vec<LLMConfig> {
        &mut self.llms
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            clusters: vec![],
            version: 1,
            path: None,
            capella_orgs: vec![],
            llms: vec![],
        }
    }
}

fn try_config_from_dot_path(mut path: PathBuf) -> Option<ShellConfig> {
    path.push(".cbsh");
    try_config_from_path(path)
}

fn try_config_from_path(mut path: PathBuf) -> Option<ShellConfig> {
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

fn try_credentials_from_dot_path(mut path: PathBuf) -> Option<CredentialsFromFile> {
    path.push(".cbsh");
    try_credentials_from_path(path)
}

fn try_credentials_from_path(mut path: PathBuf) -> Option<CredentialsFromFile> {
    path.push("credentials");

    let read = fs::read_to_string(&path);
    match read {
        Ok(r) => Some(CredentialsFromFile::from_str(&r)),
        Err(e) => {
            debug!("Could not locate {:?} because of {:?}", path, e);
            None
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CapellaOrganizationConfig {
    identifier: String,
    #[serde(flatten)]
    credentials: OrganizationCredentials,
    #[serde(default)]
    #[serde(
        rename(deserialize = "management-timeout", serialize = "management-timeout"),
        with = "humantime_serde"
    )]
    management_timeout: Option<Duration>,
    #[serde(rename(deserialize = "default-project", serialize = "default-project"))]
    default_project: Option<String>,
}

impl CapellaOrganizationConfig {
    pub fn new(
        identifier: String,
        secret_key: String,
        access_key: String,
        management_timeout: Option<Duration>,
        default_project: Option<String>,
    ) -> Self {
        Self {
            identifier,
            credentials: OrganizationCredentials {
                access_key,
                secret_key,
            },
            management_timeout,
            default_project,
        }
    }
    pub fn identifier(&self) -> String {
        self.identifier.clone()
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
    pub fn default_project(&self) -> Option<String> {
        self.default_project.as_ref().cloned()
    }

    pub fn credentials_mut(&mut self) -> &mut OrganizationCredentials {
        &mut self.credentials
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LLMConfig {
    identifier: String,
    api_key: Option<String>,
    provider: Provider,
    embed_model: Option<String>,
    chat_model: Option<String>,
}

impl LLMConfig {
    pub fn identifier(&self) -> String {
        self.identifier.clone()
    }

    pub fn api_key(&self) -> Option<String> {
        self.api_key.clone()
    }

    pub fn provider(&self) -> Provider {
        self.provider.clone()
    }

    pub fn embed_model(&self) -> Option<String> {
        self.embed_model.clone()
    }

    pub fn chat_model(&self) -> Option<String> {
        self.chat_model.clone()
    }
}

#[derive(Debug)]
pub struct ClusterConfigBuilder {
    identifier: String,
    conn_string: String,
    default_bucket: Option<String>,
    default_scope: Option<String>,
    default_collection: Option<String>,
    display_name: Option<String>,
    credentials: ClusterCredentials,
    tls: Option<ClusterTlsConfig>,
}

impl ClusterConfigBuilder {
    pub fn new(
        identifier: impl Into<String>,
        conn_string: impl Into<String>,
        credentials: ClusterCredentials,
    ) -> ClusterConfigBuilder {
        Self {
            identifier: identifier.into(),
            conn_string: conn_string.into(),
            default_bucket: None,
            default_scope: None,
            default_collection: None,
            display_name: None,
            credentials,
            tls: None,
        }
    }

    pub fn default_bucket(mut self, bucket: impl Into<Option<String>>) -> ClusterConfigBuilder {
        self.default_bucket = bucket.into();
        self
    }

    pub fn default_scope(mut self, scope: impl Into<Option<String>>) -> ClusterConfigBuilder {
        self.default_scope = scope.into();
        self
    }

    pub fn default_collection(
        mut self,
        collection: impl Into<Option<String>>,
    ) -> ClusterConfigBuilder {
        self.default_collection = collection.into();
        self
    }

    pub fn tls_config(
        mut self,
        tls_config: impl Into<Option<ClusterTlsConfig>>,
    ) -> ClusterConfigBuilder {
        self.tls = tls_config.into();
        self
    }

    pub fn build(self) -> ClusterConfig {
        ClusterConfig {
            identifier: self.identifier,
            conn_string: self.conn_string,
            default_bucket: self.default_bucket,
            default_scope: self.default_scope,
            default_collection: self.default_collection,
            display_name: self.display_name,
            credentials: self.credentials,
            timeouts: ClusterConfigTimeouts::default(),
            tls: self.tls.unwrap_or_default(),
            kv_batch_size: None,
            capella_org: None,
            cluster_type: None,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClusterConfig {
    identifier: String,
    #[serde(rename(deserialize = "connstr", serialize = "connstr"))]
    conn_string: String,
    #[serde(rename(deserialize = "default-bucket", serialize = "default-bucket"))]
    default_bucket: Option<String>,
    #[serde(rename(deserialize = "default-scope", serialize = "default-scope"))]
    default_scope: Option<String>,
    #[serde(rename(deserialize = "default-collection", serialize = "default-collection"))]
    default_collection: Option<String>,
    #[serde(rename(deserialize = "user-display-name", serialize = "user-display-name"))]
    display_name: Option<String>,

    #[serde(flatten)]
    credentials: ClusterCredentials,
    #[serde(flatten)]
    timeouts: ClusterConfigTimeouts,
    #[serde(flatten)]
    tls: ClusterTlsConfig,

    #[serde(rename(deserialize = "kv-batch-size", serialize = "kv-batch-size"))]
    kv_batch_size: Option<u32>,

    #[serde(default)]
    #[serde(rename(
        deserialize = "capella-organization",
        serialize = "capella-organization"
    ))]
    capella_org: Option<String>,

    #[serde(rename(deserialize = "type"))]
    cluster_type: Option<RemoteClusterType>,
}

impl ClusterConfig {
    pub fn identifier(&self) -> &str {
        self.identifier.as_ref()
    }

    pub fn conn_string(&self) -> &String {
        &self.conn_string
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
    pub fn cloud_org(&self) -> Option<String> {
        self.capella_org.clone()
    }
    pub fn kv_batch_size(&self) -> Option<u32> {
        self.kv_batch_size
    }
    pub fn display_name(&self) -> Option<String> {
        self.display_name.clone()
    }
    pub fn cluster_type(&self) -> Option<RemoteClusterType> {
        self.cluster_type
    }
}

impl From<(String, &RemoteCluster)> for ClusterConfig {
    fn from(cluster: (String, &RemoteCluster)) -> Self {
        let cloud = cluster.1.capella_org();

        let tls_config = if let Some(tls_config) = cluster.1.tls_config() {
            ClusterTlsConfig {
                enabled: true,
                cert_path: tls_config.cert_path(),
                accept_all_certs: tls_config.accept_all_certs(),
            }
        } else {
            ClusterTlsConfig {
                enabled: false,
                cert_path: None,
                accept_all_certs: false,
            }
        };

        Self {
            identifier: cluster.0,
            conn_string: cluster.1.hostnames().join(","),
            default_collection: cluster.1.active_collection(),
            default_scope: cluster.1.active_scope(),
            default_bucket: cluster.1.active_bucket(),
            timeouts: ClusterConfigTimeouts {
                data_timeout: Some(cluster.1.timeouts().data_timeout()),
                query_timeout: Some(cluster.1.timeouts().query_timeout()),
                analytics_timeout: Some(cluster.1.timeouts().analytics_timeout()),
                search_timeout: Some(cluster.1.timeouts().search_timeout()),
                management_timeout: Some(cluster.1.timeouts().management_timeout()),
                transaction_timeout: Some(cluster.1.timeouts().transaction_timeout()),
            },
            tls: tls_config,
            credentials: ClusterCredentials {
                username: Some(cluster.1.username().to_string()),
                password: Some(cluster.1.password().to_string()),
            },
            capella_org: cloud,
            kv_batch_size: Some(cluster.1.kv_batch_size()),
            display_name: cluster.1.display_name(),
            // This is a config option for dev ony so we won't want to write to file
            cluster_type: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrganizationCredentials {
    #[serde(default)]
    #[serde(rename(deserialize = "access-key", serialize = "access-key"))]
    access_key: String,
    #[serde(default)]
    #[serde(rename(deserialize = "secret-key", serialize = "secret-key"))]
    secret_key: String,
}

impl OrganizationCredentials {}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClusterCredentials {
    username: Option<String>,
    password: Option<String>,
}

impl ClusterCredentials {
    pub fn new(username: Option<String>, password: Option<String>) -> Self {
        Self { username, password }
    }
}

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
    #[serde(default)]
    #[serde(
        rename(deserialize = "transaction-timeout", serialize = "transaction-timeout"),
        with = "humantime_serde"
    )]
    transaction_timeout: Option<Duration>,
}

impl Default for ClusterConfigTimeouts {
    fn default() -> Self {
        ClusterConfigTimeouts {
            data_timeout: Some(DEFAULT_DATA_TIMEOUT),
            query_timeout: Some(DEFAULT_QUERY_TIMEOUT),
            analytics_timeout: Some(DEFAULT_ANALYTICS_TIMEOUT),
            search_timeout: Some(DEFAULT_SEARCH_TIMEOUT),
            management_timeout: Some(DEFAULT_MANAGEMENT_TIMEOUT),
            transaction_timeout: Some(DEFAULT_MANAGEMENT_TIMEOUT),
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

    pub fn transaction_timeout(&self) -> Option<&Duration> {
        self.transaction_timeout.as_ref()
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
        deserialize = "tls-accept-all-certs",
        serialize = "tls-accept-all-certs"
    ))]
    #[serde(default)]
    accept_all_certs: bool,
}

impl ClusterTlsConfig {
    pub fn new(enabled: bool, cert_path: Option<String>, accept_all_certs: bool) -> Self {
        Self {
            enabled,
            cert_path,
            accept_all_certs,
        }
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn cert_path(&self) -> &Option<String> {
        &self.cert_path
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
            accept_all_certs: false,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CredentialsFromFile {
    #[allow(dead_code)]
    version: usize,
    /// Note: clusters is kept for backwards compatibility and convenience
    #[serde(alias = "cluster", default)]
    #[serde(alias = "clusters")]
    clusters: Vec<ClusterCredentialsFromFile>,

    #[serde(alias = "capella-organization", default)]
    capella_orgs: Vec<OrganizationCredentialsFromFile>,

    #[serde(alias = "llm", default)]
    llms: Vec<LLMCredentials>,
}

impl CredentialsFromFile {
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

impl Default for CredentialsFromFile {
    fn default() -> Self {
        Self {
            clusters: vec![],
            version: 1,
            capella_orgs: vec![],
            llms: vec![],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrganizationCredentialsFromFile {
    identifier: String,
    #[serde(default)]
    #[serde(rename(deserialize = "access-key", serialize = "access-key"))]
    access_key: String,
    #[serde(default)]
    #[serde(rename(deserialize = "secret-key", serialize = "secret-key"))]
    secret_key: String,
}

impl OrganizationCredentialsFromFile {
    fn identifier(&self) -> String {
        self.identifier.clone()
    }
}

#[derive(Debug, Deserialize)]
pub struct ClusterCredentialsFromFile {
    identifier: String,
    username: Option<String>,
    password: Option<String>,
}

impl ClusterCredentialsFromFile {
    fn identifier(&self) -> &str {
        self.identifier.as_ref()
    }
}

fn default_as_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct LLMCredentials {
    identifier: String,
    api_key: String,
}
