use crate::common::TestConfig;
use crate::common::{config::Config, playground, TestResult};
use std::ops::Add;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time;
use std::time::Instant;
use tokio::runtime::Runtime;
use uuid::Uuid;

static TEST_CONFIG: Mutex<Option<Arc<TestConfig>>> = Mutex::new(None);

pub async fn setup() -> Arc<TestConfig> {
    let loaded_config = Config::parse();
    println!("Loaded Config: {:?}", &loaded_config);
    let test_config = playground::CBPlayground::create_test_config(loaded_config).await;
    println!("Test Config: {:?}", &test_config);

    test_config
}

pub fn test_config() -> Arc<TestConfig> {
    let mut test_config = TEST_CONFIG.lock().unwrap();

    if test_config.is_none() {
        let rt = Runtime::new().unwrap();
        let config = rt.block_on(async { setup().await });
        *test_config = Some(config.clone());
        return config;
    }

    test_config.as_ref().unwrap().clone()
}

#[allow(dead_code)]
pub fn create_index(
    base_cmd: impl Into<String>,
    fields: impl Into<String>,
    keyspace: String,
    cwd: &Path,
    sandbox: &mut playground::CBPlayground,
) -> String {
    let mut uuid = Uuid::new_v4().to_string();
    uuid.truncate(6);
    let index_name = format!("test-{}", uuid);
    let cmd = format!(
        "{} query \"CREATE INDEX `{}` ON `{}`({})\"",
        base_cmd.into(),
        index_name.clone(),
        keyspace,
        fields.into()
    );
    sandbox.retry_until(
        Instant::now().add(time::Duration::from_secs(30)),
        time::Duration::from_millis(200),
        cmd.as_str(),
        cwd,
        playground::RetryExpectations::AllowAny {
            allow_err: true,
            allow_out: true,
        },
        |_json| -> TestResult<bool> { Ok(true) },
    );

    index_name
}
