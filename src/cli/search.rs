use crate::state::State;
use async_trait::async_trait;
use couchbase::{QueryStringQuery, SearchOptions};
use futures::stream::StreamExt;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream, TaggedDictBuilder};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
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

    async fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        run(self.state.clone(), args, registry).await
    }
}

async fn run(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry).await?;
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
            let stream = r.rows().map(|v| {
                let row = v.unwrap();
                let mut collected = TaggedDictBuilder::new(Tag::default());
                collected.insert_value("id", row.id());
                collected.insert_value("score", format!("{}", row.score()));
                collected.insert_value("index", row.index());
                collected.into_value()
            });
            Ok(OutputStream::from_input(stream))
        }
        Err(e) => Err(ShellError::untagged_runtime_error(format!("{}", e))),
    }
}
