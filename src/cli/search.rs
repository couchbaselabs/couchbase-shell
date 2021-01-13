use crate::state::State;
use async_trait::async_trait;
use couchbase::{QueryStringQuery, SearchOptions};
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, OutputStream, TaggedDictBuilder};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape};
use nu_source::Tag;
use std::sync::Arc;

pub struct Search {
    state: Arc<State>,
}

impl Search {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_cli::WholeStreamCommand for Search {
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

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args).await
    }
}

async fn run(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;
    let index = args.nth(0).expect("need index name").as_string()?;
    let query = args.nth(1).expect("need query text").as_string()?;

    debug!("Running search query {} against {}", &query, &index);

    let result = state
        .active_cluster()
        .cluster()
        .search_query(
            index,
            QueryStringQuery::new(query),
            SearchOptions::default(),
        )
        .await;

    match result {
        Ok(mut r) => {
            let stream = r.rows().map(|v| match v {
                Ok(row) => {
                    let mut collected = TaggedDictBuilder::new(Tag::default());
                    collected.insert_value("id", row.id());
                    collected.insert_value("score", format!("{}", row.score()));
                    collected.insert_value("index", row.index());
                    Ok(ReturnSuccess::Value(collected.into_value()))
                }
                Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
            });
            Ok(OutputStream::new(stream))
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
