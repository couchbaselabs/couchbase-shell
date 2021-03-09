use crate::state::State;
use async_trait::async_trait;
use nu_cli::{OutputStream, TaggedDictBuilder};
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape, Value};
use nu_source::Tag;
use std::sync::Arc;

pub struct TutorialPage {
    state: Arc<State>,
}

impl TutorialPage {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl nu_engine::WholeStreamCommand for TutorialPage {
    fn name(&self) -> &str {
        "tutorial page"
    }

    fn signature(&self) -> Signature {
        Signature::build("tutorial page").optional(
            "name",
            SyntaxShape::String,
            "the name of the page to go to",
        )
    }

    fn usage(&self) -> &str {
        "Step to a specific page in the Couchbase Shell tutorial"
    }

    async fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_tutorial_page(self.state.clone(), args).await
    }
}

async fn run_tutorial_page(
    state: Arc<State>,
    args: CommandArgs,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once().await?;

    let name = match args.nth(0) {
        Some(a) => Some(a.as_string()?),
        None => None,
    };

    let tutorial = state.tutorial();
    if let Some(n) = name {
        println!("{}", tutorial.goto_step(n)?);
        Ok(OutputStream::empty())
    } else {
        let mut results: Vec<Value> = vec![];
        for s in tutorial.step_names() {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            collected.insert_value("page_name", s);
            results.push(collected.into_value());
        }
        Ok(OutputStream::from(results))
    }
}
