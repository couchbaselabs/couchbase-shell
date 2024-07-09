use envconfig::Envconfig;

#[derive(Debug, Clone)]
pub struct Config {
    username: Option<String>,
    password: Option<String>,
    conn_string: Option<String>,
    bucket: Option<String>,
    data_timeout: String,
    access_key: Option<String>,
    secret_key: Option<String>,
}

impl Config {
    pub fn username(&self) -> Option<String> {
        self.username.clone()
    }
    pub fn password(&self) -> Option<String> {
        self.password.clone()
    }
    pub fn conn_string(&self) -> Option<String> {
        self.conn_string.clone()
    }
    pub fn bucket(&self) -> Option<String> {
        self.bucket.clone()
    }
    pub fn data_timeout(&self) -> String {
        self.data_timeout.clone()
    }
    pub fn access_key(&self) -> Option<String> {
        self.access_key.clone()
    }
    pub fn secret_key(&self) -> Option<String> {
        self.secret_key.clone()
    }

    pub fn parse() -> Config {
        let config = CLIConfig::init_from_env().unwrap();

        return Config {
            username: Some(config.username()),
            password: Some(config.password()),
            conn_string: Some(config.conn_string()),
            bucket: Some(config.default_bucket()),
            data_timeout: config.data_timeout().unwrap_or_else(|| "5s".into()),
            access_key: config.access_key(),
            secret_key: config.secret_key(),
        };
    }
}

#[derive(Debug, Clone, Envconfig)]
pub struct CLIConfig {
    #[envconfig(from = "USERNAME", default = "Administrator")]
    username: String,
    #[envconfig(from = "PASSWORD", default = "password")]
    password: String,
    #[envconfig(from = "CONN_STRING")]
    conn_string: String,
    #[envconfig(from = "BUCKET", default = "default")]
    default_bucket: String,
    #[envconfig(from = "DATA_TIMEOUT")]
    data_timeout: Option<String>,
    #[envconfig(from = "ACCESS_KEY")]
    access_key: Option<String>,
    #[envconfig(from = "SECRET_KEY")]
    secret_key: Option<String>,
}

impl CLIConfig {
    pub fn username(&self) -> String {
        self.username.clone()
    }
    pub fn password(&self) -> String {
        self.password.clone()
    }
    pub fn conn_string(&self) -> String {
        self.conn_string.clone()
    }
    pub fn default_bucket(&self) -> String {
        self.default_bucket.clone()
    }
    pub fn data_timeout(&self) -> Option<String> {
        self.data_timeout.clone()
    }
    pub fn access_key(&self) -> Option<String> {
        self.access_key.clone()
    }
    pub fn secret_key(&self) -> Option<String> {
        self.secret_key.clone()
    }
}
