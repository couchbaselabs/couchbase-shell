use couchbase::{QueryOptions, Cluster};
use log::debug;
use std::error::Error;
use structopt::StructOpt;
use warp::Filter;
use nu_protocol::{Value, UntaggedValue, Signature};
use nu_errors::ShellError;
use nu::{CommandRegistry, CommandArgs, OutputStream};
use std::sync::Arc;
use futures::executor::{block_on, block_on_stream};
use futures::stream::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let opt = CliOptions::from_args();
    debug!("Effective {:?}", opt);

    let cluster = Arc::new(Cluster::connect(opt.connection_string, opt.username, opt.password));

    if opt.ui {
        tokio::task::spawn(async {
            let hello =
                warp::path!("hello" / String).map(|name| format!("Couchbase says, {}!", name));
            warp::serve(hello).run(([127, 0, 0, 1], 1908)).await;
        });
    }

    let mut syncer = nu::EnvironmentSyncer::new();
    let mut context = nu::create_default_context(&mut syncer)?;
    context.add_commands(vec![nu::whole_stream_command(Query::new(cluster.clone()))]);

    nu::cli(Some(syncer), Some(context)).await
}

struct Query {
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
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let mut result = block_on(self.cluster.query("select 1=1", QueryOptions::default())).unwrap();
        let stream = result.rows::<serde_json::Value>().map(|v| {
            // this is just a prototype...
            let raw = serde_json::to_string(&v.unwrap()).unwrap();
            raw.into()
        });
        Ok(OutputStream::from_input(stream))
    }
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "The Couchbase Shell",
    about = "Alternative Shell and UI for Couchbase Server and Cloud"
)]
struct CliOptions {
    #[structopt(
        short = "c",
        long = "connstring",
        default_value = "couchbase://localhost"
    )]
    connection_string: String,
    #[structopt(long = "ui")]
    ui: bool,
    #[structopt(short = "u", long = "username", default_value = "Administrator")]
    username: String,
    #[structopt(short = "p", long = "password", default_value = "password")]
    password: String,
}
