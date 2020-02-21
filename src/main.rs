use crate::cli::analytics::Analytics;
use crate::cli::get::Get;
use crate::cli::query::Query;

use couchbase::Cluster;
use log::debug;
use std::error::Error;
use std::sync::Arc;
use structopt::StructOpt;
use warp::Filter;
mod cli;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let opt = CliOptions::from_args();
    debug!("Effective {:?}", opt);

    let cluster = Arc::new(Cluster::connect(
        opt.connection_string,
        opt.username,
        opt.password,
    ));

    if opt.ui {
        tokio::task::spawn(async {
            let hello =
                warp::path!("hello" / String).map(|name| format!("Couchbase says, {}!", name));
            warp::serve(hello).run(([127, 0, 0, 1], 1908)).await;
        });
    }

    let mut syncer = nu::EnvironmentSyncer::new();
    let mut context = nu::create_default_context(&mut syncer)?;
    context.add_commands(vec![
        nu::whole_stream_command(Query::new(cluster.clone())),
        nu::whole_stream_command(Analytics::new(cluster.clone())),
        nu::whole_stream_command(Get::new(cluster.clone())),
    ]);

    nu::cli(Some(syncer), Some(context)).await
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
