use crate::client::ManagementRequest;
use crate::state::State;
use async_trait::async_trait;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, UntaggedValue};
use nu_source::Tag;
use nu_stream::OutputStream;
use std::ops::Add;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Tutorial {
    state: Arc<Mutex<State>>,
}

impl Tutorial {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Tutorial {
    fn name(&self) -> &str {
        "tutorial"
    }

    fn signature(&self) -> Signature {
        Signature::build("tutorial")
    }

    fn usage(&self) -> &str {
        "Run the Couchbase Shell tutorial"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        let ctrl_c = args.ctrl_c();
        run_tutorial(self.state.clone(), ctrl_c)
    }
}

fn run_tutorial(
    state: Arc<Mutex<State>>,
    ctrl_c: Arc<AtomicBool>,
) -> Result<OutputStream, ShellError> {
    let guard = state.lock().unwrap();
    let tutorial = guard.tutorial();
    let exists = match guard.active_cluster() {
        Some(active_cluster) => {
            if active_cluster.capella_org().is_none() {
                let resp = active_cluster.cluster().http_client().management_request(
                    ManagementRequest::GetBucket {
                        name: "travel-sample".into(),
                    },
                    Instant::now().add(active_cluster.timeouts().management_timeout()),
                    ctrl_c,
                );

                match resp {
                    Ok(r) => matches!(r.status(), 200),
                    Err(_) => false,
                }
            } else {
                // Bit of a hack, if the user is on cloud then they can't enable travel-sample
                true
            }
        }
        None => true,
    };

    Ok(OutputStream::one(
        UntaggedValue::string(tutorial.current_step(exists)).into_value(Tag::unknown()),
    ))
}
