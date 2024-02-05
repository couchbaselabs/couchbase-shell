use crate::features::TestFeature;
use envconfig::Envconfig;

#[derive(Debug, Copy, Clone)]
pub enum ClusterType {
    Standalone,
    Mock,
}

#[derive(Debug, Clone)]
pub struct Config {
    cluster_type: ClusterType,
    username: Option<String>,
    password: Option<String>,
    conn_string: Option<String>,
    caves_version: Option<String>,
    bucket: Option<String>,
    enabled_features: Vec<TestFeature>,
    data_timeout: String,
    capella_access_key: Option<String>,
    capella_secret_key: Option<String>,
}

impl Config {
    pub fn cluster_type(&self) -> ClusterType {
        self.cluster_type
    }
    pub fn enabled_features(&self) -> Vec<TestFeature> {
        self.enabled_features.clone()
    }
    pub fn username(&self) -> Option<String> {
        self.username.clone()
    }
    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }
    pub fn conn_string(&self) -> Option<String> {
        self.conn_string.clone()
    }
    pub fn caves_version(&self) -> Option<String> {
        self.caves_version.clone()
    }
    pub fn bucket(&self) -> Option<String> {
        self.bucket.clone()
    }
    pub fn data_timeout(&self) -> String {
        self.data_timeout.clone()
    }
    pub fn capella_access_key(&self) -> Option<String> {
        self.capella_access_key.clone()
    }
    pub fn capella_secret_key(&self) -> Option<String> {
        self.capella_secret_key.clone()
    }

    pub fn parse() -> Config {
        let config = CLIConfig::init_from_env().unwrap();

        let str_features = config.features();
        let features = if str_features.is_empty() {
            vec![]
        } else {
            let mut features = vec![];
            for feature in str_features.split(',') {
                features.push(TestFeature::from(feature))
            }
            features
        };

        if let Some(conn_str) = config.conn_string() {
            let username = config.username();
            let password = config.password();
            let bucket = config.default_bucket();

            return Config {
                cluster_type: ClusterType::Standalone,
                username: Some(username),
                password: Some(password),
                conn_string: Some(conn_str),
                caves_version: None,
                bucket: Some(bucket),
                enabled_features: features,
                data_timeout: config.data_timeout().unwrap_or_else(|| "5s".into()),
                capella_access_key: config.capella_access_key(),
                capella_secret_key: config.capella_secret_key(),
            };
        }

        Config {
            cluster_type: ClusterType::Mock,
            username: None,
            password: None,
            conn_string: None,
            caves_version: config.caves_version(),
            bucket: None,
            enabled_features: features,
            data_timeout: config.data_timeout().unwrap_or_else(|| "5s".into()),
            capella_access_key: config.capella_access_key(),
            capella_secret_key: config.capella_secret_key(),
        }
    }
}

#[derive(Debug, Clone, Envconfig)]
pub struct CLIConfig {
    #[envconfig(from = "USERNAME", default = "Administrator")]
    username: String,
    #[envconfig(from = "PASSWORD", default = "password")]
    password: String,
    #[envconfig(from = "CONN_STRING")]
    conn_string: Option<String>,
    #[envconfig(from = "BUCKET", default = "default")]
    default_bucket: String,
    #[envconfig(from = "CAVES_VERSION")]
    caves_version: Option<String>,
    #[envconfig(from = "FEATURES", default = "")]
    features: String,
    #[envconfig(from = "DATA_TIMEOUT")]
    data_timeout: Option<String>,
    #[envconfig(from = "CAPELLA_ACCESS_KEY")]
    capella_access_key: Option<String>,
    #[envconfig(from = "CAPELLA_SECRET_KEY")]
    capella_secret_key: Option<String>,
}

impl CLIConfig {
    pub fn username(&self) -> String {
        self.username.clone()
    }
    pub fn password(&self) -> String {
        self.password.clone()
    }
    pub fn conn_string(&self) -> Option<String> {
        self.conn_string.clone()
    }
    pub fn default_bucket(&self) -> String {
        self.default_bucket.clone()
    }
    pub fn caves_version(&self) -> Option<String> {
        self.caves_version.clone()
    }
    pub fn features(&self) -> String {
        self.features.clone()
    }
    pub fn data_timeout(&self) -> Option<String> {
        self.data_timeout.clone()
    }
    pub fn capella_access_key(&self) -> Option<String> {
        self.capella_access_key.clone()
    }
    pub fn capella_secret_key(&self) -> Option<String> {
        self.capella_secret_key.clone()
    }
}
