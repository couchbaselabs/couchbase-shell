extern crate core;

pub mod support;
mod test_functions;
pub mod tests;
pub mod util;

use crate::util::config::{ClusterType, Config};
use crate::util::mock::MockCluster;
use crate::util::standalone::StandaloneCluster;
use ansi_term::Colour;
use env_logger::Env;
use std::fmt::{Display, Formatter};
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::Instant;
use util::*;

async fn setup() -> Arc<ClusterUnderTest> {
    let loaded_config = Config::parse();
    println!("Config: {:?}", &loaded_config);
    let server = match loaded_config.cluster_type() {
        ClusterType::Standalone => {
            ClusterUnderTest::Standalone(StandaloneCluster::start(loaded_config).await)
        }
        ClusterType::Mock => ClusterUnderTest::Mocked(MockCluster::start(loaded_config).await),
    };
    println!("Cluster: {:?}", &server);

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
            TestResultStatus::Success => Colour::Green.paint("ok"),
            TestResultStatus::Failure => Colour::Red.paint("FAILED"),
            TestResultStatus::Skipped => Colour::Yellow.paint("ignored"),
        };

        write!(f, "{}", alias)
    }
}

#[derive(Debug)]
struct TestOutcome {
    result: TestResultStatus,
}

impl Display for TestOutcome {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let out = format!("{}", self.result);
        write!(f, "{}", out)
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let start = Instant::now();
    let cluster = setup().await;

    let mut success = 0;
    let mut failures = vec![];
    let mut skipped = 0;
    let tests = test_functions::tests(cluster.clone());
    println!();
    println!("running {} tests", tests.len());
    for t in tests {
        let handle = tokio::spawn(t.func);
        let result = match handle.await {
            Ok(was_skipped) => {
                if was_skipped {
                    skipped += 1;
                    TestOutcome {
                        result: TestResultStatus::Skipped,
                    }
                } else {
                    success += 1;
                    TestOutcome {
                        result: TestResultStatus::Success,
                    }
                }
            }
            Err(_e) => {
                // The JoinError here doesn't tell us anything interesting but the panic will be
                // output to stderr anyway.
                failures.push(t.name.clone());
                TestOutcome {
                    result: TestResultStatus::Failure,
                }
            }
        };

        println!("test {} ... {}", t.name.clone(), result);
    }

    teardown();
    let elapsed = start.elapsed();

    let overall = if failures.len() == 0 {
        Colour::Green.paint("ok")
    } else {
        Colour::Red.paint("FAILED")
    };

    println!();
    println!(
        "test result: {}. {} passed; {} failed; {} ignored; 0 measured; 0 filtered out; finished in {}.{:.2}s",
        overall,
        success,
        failures.len(),
        skipped,
        elapsed.as_secs(),
        elapsed.subsec_millis()
    );
    println!();

    if failures.len() == 0 {
        Ok(())
    } else {
        Err(std::io::Error::new(ErrorKind::Other, "test failures"))
    }
}
