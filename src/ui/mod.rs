//! This module holds everything that relates to the HTTP UI cbshell supports.

use crate::cli::add_commands;
use crate::state::State;
use log::warn;
use nu_cli::parse_and_eval;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::runtime::Handle;
use warp::http::{Method, StatusCode};
use warp::Filter;

pub fn serve(state: Arc<Mutex<State>>) -> Result<(), Box<dyn Error>> {
    let _handle = thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let cors = warp::cors()
                    .allow_any_origin()
                    .allow_headers(vec!["content-type"])
                    .allow_methods(&[Method::POST]);

                let routes = warp::path!("api" / "notebook" / "exec")
                    .and(warp::post())
                    .map(move || state.clone())
                    .and(warp::body::json())
                    .and_then(execute_script)
                    .with(cors);

                warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
            });
    });
    Ok(())
}

async fn execute_script(
    state: Arc<Mutex<State>>,
    args: ScriptArguments,
) -> Result<impl warp::Reply, Infallible> {
    match args.input_type {
        ScriptInputType::Shell => execute_shell_script(state, args.input_value).await,
        ScriptInputType::Query => {
            execute_shell_script(state, format!(r#"query "{}""#, args.input_value)).await
        }
        ScriptInputType::Analytics => {
            execute_shell_script(state, format!(r#"analytics "{}""#, args.input_value)).await
        }
    }
}

async fn execute_shell_script(
    state: Arc<Mutex<State>>,
    input: String,
) -> Result<impl warp::Reply, Infallible> {
    let ctx = nu_cli::create_default_context(true).unwrap();
    add_commands(&ctx, state.clone());

    let input = format!("{} | to json", input);
    Handle::current()
        .spawn_blocking(move || match parse_and_eval(&input, &ctx) {
            Ok(r) => Ok(warp::reply::with_status(
                warp::reply::json(&ScriptResult {
                    result: r,
                    error: "".into(),
                }),
                StatusCode::OK,
            )),
            Err(e) => {
                warn!("{}", e);
                Ok(warp::reply::with_status(
                    warp::reply::json(&ScriptResult {
                        result: "".into(),
                        error: format!("{}", e),
                    }),
                    StatusCode::BAD_REQUEST,
                ))
            }
        })
        .await
        .unwrap()
}

#[derive(Deserialize, Debug)]
struct ScriptArguments {
    #[serde(alias = "inputType")]
    input_type: ScriptInputType,
    #[serde(alias = "inputValue")]
    input_value: String,
}

#[derive(Deserialize, Debug)]
enum ScriptInputType {
    #[serde(alias = "shell")]
    Shell,
    #[serde(alias = "query")]
    Query,
    #[serde(alias = "analytics")]
    Analytics,
}

#[derive(Serialize)]
struct ScriptResult {
    result: String,
    error: String,
}
