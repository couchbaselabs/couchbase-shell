use log::debug;
use std::error::Error;
use structopt::StructOpt;
use warp::Filter;
use couchbase::Cluster;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let opt = CliOptions::from_args();
    debug!("Effective {:?}", opt);

    let _cluster = Cluster::connect(opt.connection_string, opt.username, opt.password);

    if opt.ui {
        tokio::task::spawn(async {
            let hello = warp::path!("hello" / String).map(|name| format!("Couchbase says, {}!", name));
            warp::serve(hello).run(([127, 0, 0, 1], 1908)).await;
        });
    }

    nu::cli().await
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
    #[structopt(
        short = "u",
        long = "username",
        default_value = "Administrator"
    )]
    username: String,
    #[structopt(
        short = "p",
        long = "password",
        default_value = "password"
    )]
    password: String,
}
