use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use couchbase::GetOptions;

use futures::executor::block_on;
use log::debug;
use nu::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, TaggedDictBuilder, UntaggedValue};
use nu_source::Tag;
use std::sync::Arc;

pub struct Get {
    state: Arc<State>,
}

impl Get {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu::WholeStreamCommand for Get {
    fn name(&self) -> &str {
        "kv-get"
    }

    fn signature(&self) -> Signature {
        Signature::build("kv-get").required("id", SyntaxShape::String, "the document id")
    }

    fn usage(&self) -> &str {
        "Fetches a document through Key/Value"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once(registry)?;
        let id = args.nth(0).expect("need id").as_string()?;

        let bucket = self
            .state
            .active_cluster()
            .cluster()
            .bucket("travel-sample");
        let collection = bucket.default_collection();
        debug!("Running kv get for doc {}", &id);

        let output = match block_on(collection.get(&id, GetOptions::default())) {
            Ok(res) => {
                let tag = Tag::default();
                let mut collected = TaggedDictBuilder::new(&tag);
                collected.insert_value("id", id);
                collected.insert_value("cas", UntaggedValue::int(res.cas()).into_untagged_value());
                let content = res.content::<serde_json::Value>().unwrap();
                let content_converted = convert_json_value_to_nu_value(&content, Tag::default());
                collected.insert_value("content", content_converted);
                OutputStream::one(Ok(ReturnSuccess::Value(collected.into_value())))
            }
            Err(_e) => OutputStream::empty(),
        };

        Ok(output)
    }
}
