use crate::cli::util::cluster_identifiers_from;
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
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

pub struct Search {
    state: Arc<Mutex<State>>,
}

impl Search {
    pub fn new(state: Arc<Mutex<State>>) -> Self {
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
            .named(
                "clusters",
                SyntaxShape::String,
                "the clusters which should be contacted",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Performs a search query"
    }

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args)
    }
}

fn run(state: Arc<Mutex<State>>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let ctrl_c = args.ctrl_c();
    let index: String = args.req(0)?;
    let query: String = args.req(1)?;

    debug!("Running search query {} against {}", &query, &index);

    let cluster_identifiers = cluster_identifiers_from(&state, &args, true)?;
    let guard = state.lock().unwrap();

    let mut results = vec![];
    for identifier in cluster_identifiers {
        let active_cluster = match guard.clusters().get(&identifier) {
            Some(c) => c,
            None => {
                return Err(ShellError::untagged_runtime_error("Cluster not found"));
            }
        };
        let response = active_cluster
            .cluster()
            .http_client()
            .search_query_request(
                SearchQueryRequest::Execute {
                    query: query.clone(),
                    index: index.clone(),
                },
                Instant::now().add(active_cluster.timeouts().search_timeout()),
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

        for row in rows.hits {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("id", row.id);
            collected.insert_value("score", format!("{}", row.score));
            collected.insert_value("index", row.index);
            collected.insert_value("cluster", identifier.clone());

            results.push(collected.into_value());
        }
    }
    Ok(OutputStream::from(results))
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
