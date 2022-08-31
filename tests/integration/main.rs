extern crate core;

pub mod support;
mod test_functions;
pub mod tests;
pub mod util;

use crate::util::config::{ClusterType, Config};
use crate::util::mock::MockCluster;
use crate::util::standalone::StandaloneCluster;
use env_logger::Env;

use nu_protocol::ShellError;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;
use std::sync::Arc;
use util::*;

async fn setup() -> Arc<ClusterUnderTest> {
    let loaded_config = Config::parse();
    println!("Config: {:?}", &loaded_config);
    let server = match loaded_config.cluster_type() {
        ClusterType::Standalone => {
            ClusterUnderTest::Standalone(StandaloneCluster::start(loaded_config, vec![]).await)
        }
        ClusterType::Mock => {
            ClusterUnderTest::Mocked(MockCluster::start(loaded_config, vec![]).await)
        }
    };

    Arc::new(server)
}

fn teardown() {}

#[derive(Debug, Copy, Clone)]
enum TestResultStatus {
    Success,
    Failure,
    Skipped,
}

impl Display for TestResultStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let alias = match *self {
            TestResultStatus::Success => "success",
            TestResultStatus::Failure => "failure",
            TestResultStatus::Skipped => "skipped",
        };

        write!(f, "{}", alias)
    }
}

#[derive(Debug)]
pub struct TestError {
    reason: String,
}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason.clone())
    }
}

impl Error for TestError {}

impl From<ShellError> for TestError {
    fn from(e: ShellError) -> Self {
        Self {
            reason: e.to_string(),
        }
    }
}

#[derive(Debug)]
struct TestOutcome {
    name: String,
    result: TestResultStatus,
    error: Option<TestError>,
}

impl Display for TestOutcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut out = format!("{} -> {}", self.name.clone(), self.result);
        if let Some(e) = &self.error {
            out = format!("{}: {}", out, e);
        }
        write!(f, "{}", out)
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let cluster = setup().await;

    let mut success = 0;
    let mut failures = vec![];
    let mut skipped = 0;
    for t in test_functions::tests(cluster.clone()) {
        if cluster.config().test_enabled(t.name.clone()) {
            println!();
            println!("Running {}", t.name.clone());
            let handle = tokio::spawn(t.func);
            let result = match handle.await {
                Ok(was_skipped) => {
                    if was_skipped {
                        skipped += 1;
                        TestOutcome {
                            name: t.name.to_string(),
                            result: TestResultStatus::Skipped,
                            error: None,
                        }
                    } else {
                        success += 1;
                        TestOutcome {
                            name: t.name.to_string(),
                            result: TestResultStatus::Success,
                            error: None,
                        }
                    }
                }
                Err(_e) => {
                    // The JoinError here doesn't tell us anything interesting but the panic will be
                    // output to stderr anyway.
                    failures.push(t.name.clone());
                    TestOutcome {
                        name: t.name.to_string(),
                        result: TestResultStatus::Failure,
                        error: None,
                    }
                }
            };

            println!("{}", result);
            println!();
        } else {
            println!("Skipping {}, not enabled", t.name.clone());
            skipped += 1;
        }
    }

    teardown();

    println!();
    println!(
        "Success: {}, Failures: {}, Skipped: {}",
        success,
        failures.len(),
        skipped
    );
    if failures.len() > 0 {
        println!("Failed: {}", failures.join(", "));
    }
    println!();

    if failures.len() == 0 {
        Ok(())
    } else {
        Err(std::io::Error::new(ErrorKind::Other, "test failures"))
    }
}
