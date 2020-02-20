use couchbase::{Cluster, QueryOptions};
use futures::executor::{block_on};
use futures::stream::StreamExt;
use nu::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature};
use std::sync::Arc;

pub struct Query {
    cluster: Arc<Cluster>,
}

impl Query {
    pub fn new(cluster: Arc<Cluster>) -> Self {
        Self { cluster }
    }
}

impl nu::WholeStreamCommand for Query {
    fn name(&self) -> &str {
        "query"
    }

    fn signature(&self) -> Signature {
        Signature::build("query")
    }

    fn usage(&self) -> &str {
        "Performs a N1QL query"
    }

    fn run(
        &self,
        _args: CommandArgs,
        _registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let mut result =
            block_on(self.cluster.query("select 1=1", QueryOptions::default())).unwrap();
        let stream = result.rows::<serde_json::Value>().map(|v| {
            // this is just a prototype...
            let raw = serde_json::to_string(&v.unwrap()).unwrap();
            raw.into()
        });
        Ok(OutputStream::from_input(stream))
    }
}
