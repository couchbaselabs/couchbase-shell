use crate::client::SearchQueryRequest;
use crate::state::State;
use async_trait::async_trait;
use log::debug;
use nu_cli::TaggedDictBuilder;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use nu_stream::OutputStream;
use serde_derive::Deserialize;
use std::ops::Add;
use std::sync::Arc;
use tokio::time::Instant;

pub struct Search {
    state: Arc<State>,
}

impl Search {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for Search {
    fn name(&self) -> &str {
        "search"
    }

    fn signature(&self) -> Signature {
        Signature::build("search")
            .required("index", SyntaxShape::String, "the index name")
            .required(
                "query",
                SyntaxShape::String,
                "the text to query for using a query string query",
            )
    }

    fn usage(&self) -> &str {
        "Performs a search query"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args)
    }
}

fn run(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let args = args.evaluate_once()?;
    let index = args.nth(0).expect("need index name").as_string()?;
    let query = args.nth(1).expect("need query text").as_string()?;

    debug!("Running search query {} against {}", &query, &index);

    let active_cluster = match args.call_info.args.get("cluster") {
        Some(c) => {
            let identifier = match c.as_string() {
                Ok(s) => s,
                Err(e) => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Could not convert cluster name to string: {}",
                        e
                    )));
                }
            };
            match state.clusters().get(identifier.as_str()) {
                Some(c) => c,
                None => {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Could not get cluster from available clusters",
                    )));
                }
            }
        }
        None => state.active_cluster(),
    };

    let response = active_cluster.cluster().search_query_request(
        SearchQueryRequest::Execute { query, index },
        Instant::now().add(active_cluster.timeouts().query_timeout()),
        ctrl_c.clone(),
    )?;

    let rows: SearchResultData = match response.status() {
        200 => match serde_json::from_str(response.content()) {
            Ok(m) => m,
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Failed to decode response body {}",
                    e,
                )));
            }
        },
        _ => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Request failed {}",
                response.content(),
            )));
        }
    };

    let mut entries = vec![];
    for row in rows.hits {
        let mut collected = TaggedDictBuilder::new(Tag::default());
        collected.insert_value("id", row.id);
        collected.insert_value("score", format!("{}", row.score));
        collected.insert_value("index", row.index);

        entries.push(collected.into_value());
    }

    Ok(entries.into())
}

#[derive(Debug, Deserialize)]
struct SearchResultHit {
    score: f32,
    index: String,
    id: String,
}

#[derive(Debug, Deserialize)]
struct SearchResultData {
    hits: Vec<SearchResultHit>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    data: SearchResultData,
}
