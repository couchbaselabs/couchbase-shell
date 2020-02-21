use super::util::convert_json_value_to_nu_value;
use couchbase::{Cluster, QueryOptions};
use futures::executor::block_on;
use futures::stream::StreamExt;
use log::debug;
use nu::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
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
        Signature::build("query").required("statement", SyntaxShape::String, "the query statement")
    }

    fn usage(&self) -> &str {
        "Performs a n1ql query"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once(registry)?;
        let statement = args.nth(0).expect("need statement").as_string()?;

        debug!("Running n1ql query {}", &statement);
        let mut result = block_on(self.cluster.query(statement, QueryOptions::default())).unwrap();
        let stream = result
            .rows::<serde_json::Value>()
            .map(|v| convert_json_value_to_nu_value(&v.unwrap(), Tag::default()));
        Ok(OutputStream::from_input(stream))
    }
}
