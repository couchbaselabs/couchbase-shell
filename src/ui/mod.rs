use crate::state::State;
use http::Uri;
use rust_embed::RustEmbed;
use std::sync::Arc;
use warp::{http::header::HeaderValue, path::Tail, reply::Response, Filter, Rejection, Reply};

#[derive(RustEmbed)]
#[folder = "ui-assets/"]
struct Asset;

pub async fn spawn_and_serve(state: Arc<State>) {
    let index = warp::path::end().and_then(serve_index);

    let p_state = state.clone();
    let pools = warp::path("pools")
        .and(warp::any().map(move || p_state.clone()))
        .and_then(serve_pools);
    let serve_query_service = warp::post()
        .and(warp::path!("_p" / "query" / "query" / "service"))
        .and(warp::any().map(move || state.clone()))
        .and(warp::body::json())
        .and_then(serve_query_service);

    let ui_assets = warp::path("ui").and(warp::path::tail()).and_then(serve);

    let routes = index.or(pools).or(serve_query_service).or(ui_assets);
    warp::serve(routes).run(([127, 0, 0, 1], 1908)).await;
}

pub async fn serve_index() -> Result<impl Reply, Rejection> {
    Ok(warp::redirect(Uri::from_static("/ui/index.html")))
}

pub async fn serve_pools(state: Arc<State>) -> Result<impl Reply, Rejection> {
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

pub async fn serve_query_service(
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

pub async fn serve(path: Tail) -> Result<impl Reply, Rejection> {
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
