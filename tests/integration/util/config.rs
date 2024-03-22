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
    capella_conn_string: Option<String>,
    capella_username: Option<String>,
    capella_password: Option<String>,
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
    pub fn capella_conn_string(&self) -> Option<String> {
        self.capella_conn_string.clone()
    }
    pub fn capella_username(&self) -> Option<String> {
        self.capella_username.clone()
    }
    pub fn capella_password(&self) -> Option<String> {
        self.capella_password.clone()
    }
    pub fn capella_access_key(&self) -> Option<String> {
        self.capella_access_key.clone()
    }
    pub fn capella_secret_key(&self) -> Option<String> {
        self.capella_secret_key.clone()
    }

    pub fn parse() -> Config {
        let cli_config = CLIConfig::init_from_env().unwrap();

        let str_features = cli_config.features();
        let features = if str_features.is_empty() {
            vec![]
        } else {
            let mut features = vec![];
            for feature in str_features.split(',') {
                features.push(TestFeature::from(feature))
            }
            features
        };

        let mut config = Config {
            cluster_type: ClusterType::Mock,
            username: None,
            password: None,
            conn_string: None,
            caves_version: cli_config.caves_version(),
            bucket: None,
            enabled_features: features,
            data_timeout: cli_config.data_timeout().unwrap_or_else(|| "5s".into()),
            capella_conn_string: None,
            capella_username: None,
            capella_password: None,
            capella_access_key: None,
            capella_secret_key: None,
        };

        if let Some(conn_str) = cli_config.conn_string() {
            config.cluster_type = ClusterType::Standalone;
            config.username = Some(cli_config.username());
            config.password = Some(cli_config.password());
            config.conn_string = Some(conn_str);
            config.bucket = Some(cli_config.default_bucket());
        };

        if let Some(capella_conn_str) = cli_config.capella_conn_string() {
            config.cluster_type = ClusterType::Standalone;
            config.capella_username = Some(cli_config.capella_username().unwrap());
            config.capella_password = Some(cli_config.capella_password().unwrap());
            config.capella_conn_string = Some(capella_conn_str);
            config.capella_access_key = Some(cli_config.capella_access_key().unwrap());
            config.capella_secret_key = Some(cli_config.capella_secret_key().unwrap());
        }

        config
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
    #[envconfig(from = "CAPELLA_CONN_STRING")]
    capella_conn_string: Option<String>,
    #[envconfig(from = "CAPELLA_USERNAME")]
    capella_username: Option<String>,
    #[envconfig(from = "CAPELLA_PASSWORD")]
    capella_password: Option<String>,
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
    pub fn capella_conn_string(&self) -> Option<String> {
        self.capella_conn_string.clone()
    }
    pub fn capella_username(&self) -> Option<String> {
        self.capella_username.clone()
    }
    pub fn capella_password(&self) -> Option<String> {
        self.capella_password.clone()
    }
    pub fn capella_access_key(&self) -> Option<String> {
        self.capella_access_key.clone()
    }
    pub fn capella_secret_key(&self) -> Option<String> {
        self.capella_secret_key.clone()
    }
}
