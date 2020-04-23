//! The `kv-replace` command performs a KV replace operation.

use super::util::{json_rows_from_input_columns, json_rows_from_input_optionals};

use crate::state::State;
use couchbase::ReplaceOptions;

use futures::executor::block_on;
use log::debug;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, TaggedDictBuilder};
use nu_source::Tag;
use std::sync::Arc;

pub struct Replace {
    state: Arc<State>,
}

impl Replace {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_cli::WholeStreamCommand for Replace {
    fn name(&self) -> &str {
        "kv-replace"
    }

    fn signature(&self) -> Signature {
        Signature::build("kv-replace")
            .optional("id", SyntaxShape::String, "the document id")
            .optional("content", SyntaxShape::String, "the document content")
            .named(
                "id-column",
                SyntaxShape::String,
                "the name of the id column if used with an input stream",
                None,
            )
            .named(
                "content-column",
                SyntaxShape::String,
                "the name of the content column if used with an input stream",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Replace a document through Key/Value"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(run_replace(self.state.clone(), args, registry))
    }
}

async fn run_replace(
    state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let mut args = args.evaluate_once(registry)?;

    let id_column = args
        .get("id-column")
        .map(|id| id.as_string().unwrap())
        .unwrap_or_else(|| String::from("id"));

    let content_column = args
        .get("content-column")
        .map(|content| content.as_string().unwrap())
        .unwrap_or_else(|| String::from("content"));

    let mut rows = json_rows_from_input_columns(&mut args, &id_column, &content_column).await?;
    rows.extend(json_rows_from_input_optionals(&mut args)?);

    let bucket = state.active_cluster().cluster().bucket("travel-sample");
    let collection = bucket.default_collection();

    debug!("Running kv replace for docs {:?}", &rows);

    let mut results = vec![];
    for (id, content) in rows.iter() {
        match collection
            .replace(id, content, ReplaceOptions::default())
            .await
        {
            Ok(_) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value(&id_column, id.clone());
                results.push(collected.into_value());
            }
            Err(e) => {
                return Err(ShellError::untagged_runtime_error(format!("{}", e)));
            }
        };
    }
    Ok(OutputStream::from(results))
}
