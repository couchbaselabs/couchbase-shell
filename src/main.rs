mod cli;
mod config;
mod state;

use crate::cli::*;
use crate::config::ShellConfig;
use crate::state::RemoteCluster;
use http::Uri;
use log::{debug, warn};
use rust_embed::RustEmbed;
use state::State;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use structopt::StructOpt;
use warp::{http::header::HeaderValue, path::Tail, reply::Response, Filter, Rejection, Reply};
#[derive(RustEmbed)]
#[folder = "ui-assets/"]
struct Asset;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let opt = CliOptions::from_args();
    debug!("Effective {:?}", opt);

    let config = ShellConfig::new();
    warn!("Config {:?}", config);

    let mut clusters = HashMap::new();

    let active = if config.clusters().is_empty() {
        let cluster = RemoteCluster::new(opt.connection_string, opt.username, opt.password);
        clusters.insert("default".into(), cluster);
        String::from("default")
    } else {
        let mut active = None;
        for (k, v) in config.clusters() {
            let name = k.clone();
            let cluster =
                RemoteCluster::new(v.connstr().into(), v.username().into(), v.password().into());
            clusters.insert(name.clone(), cluster);
            if opt.cluster.as_ref().is_some() {
                if &name == opt.cluster.as_ref().unwrap() {
                    active = Some(name.clone())
                }
            } else {
                if active.is_none() {
                    active = Some(k.clone());
                }
            }
        }
        active.unwrap()
    };

    let state = Arc::new(State::new(clusters, active));

    if opt.ui {
        let web_state = state.clone();
        tokio::task::spawn(async {
            let index = warp::path::end().and_then(serve_index);

            let p_state = web_state.clone();
            let pools = warp::path("pools")
                .and(warp::any().map(move || p_state.clone()))
                .and_then(serve_pools);
            let serve_query_service = warp::post()
                .and(warp::path!("_p" / "query" / "query" / "service"))
                .and(warp::any().map(move || web_state.clone()))
                .and(warp::body::json())
                .and_then(serve_query_service);

            let ui_assets = warp::path("ui").and(warp::path::tail()).and_then(serve);

            let routes = index.or(pools).or(serve_query_service).or(ui_assets);
            warp::serve(routes).run(([127, 0, 0, 1], 1908)).await;
        });
    }

    let mut syncer = nu_cli::EnvironmentSyncer::new();
    let mut context = nu_cli::create_default_context(&mut syncer)?;
    context.add_commands(vec![
        // Performs analytics queries
        nu_cli::whole_stream_command(Analytics::new(state.clone())),
        // Performs kv get operations
        nu_cli::whole_stream_command(Get::new(state.clone())),
        // Displays cluster manager node infos
        nu_cli::whole_stream_command(Nodes::new(state.clone())),
        // Displays cluster manager bucket infos
        nu_cli::whole_stream_command(Buckets::new(state.clone())),
        // Performs n1ql queries
        nu_cli::whole_stream_command(Query::new(state.clone())),
        // Manages local cluster references
        nu_cli::whole_stream_command(Clusters::new(state.clone())),
        // Create fake data based on templates
        nu_cli::whole_stream_command(FakeData::new(state.clone())),
    ]);

    nu_cli::cli(Some(syncer), Some(context)).await
}

async fn serve_index() -> Result<impl Reply, Rejection> {
    Ok(warp::redirect(Uri::from_static("/ui/index.html")))
}

async fn serve_pools(state: Arc<State>) -> Result<impl Reply, Rejection> {
    let client = reqwest::Client::new();

    let host = state.active_cluster().connstr().replace("couchbase://", "");
    let uri = format!("http://{}:8091/pools", host);

    let resp = client
        .get(&uri)
        .basic_auth(
            state.active_cluster().username(),
            Some(state.active_cluster().password()),
        )
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    Ok(warp::reply::json(&resp))
}

async fn serve_query_service(
    state: Arc<State>,
    body: serde_json::Value,
) -> Result<impl Reply, Rejection> {
    let client = reqwest::Client::new();

    let host = state.active_cluster().connstr().replace("couchbase://", "");
    let uri = format!("http://{}:8091/_p/query/query/service", host);

    let resp = client
        .post(&uri)
        .basic_auth(
            state.active_cluster().username(),
            Some(state.active_cluster().password()),
        )
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();

    Ok(warp::reply::json(&resp))
}

async fn serve(path: Tail) -> Result<impl Reply, Rejection> {
    serve_impl(path.as_str())
}

fn serve_impl(path: &str) -> Result<impl Reply, Rejection> {
    let asset = Asset::get(path).ok_or_else(warp::reject::not_found)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let mut res = Response::new(asset.into());
    res.headers_mut().insert(
        "content-type",
        HeaderValue::from_str(mime.as_ref()).unwrap(),
    );
    Ok(res)
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
    #[structopt(long = "cluster")]
    cluster: Option<String>,
}
