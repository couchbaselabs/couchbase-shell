use crate::state::State;
use async_trait::async_trait;
use nu_cli::TaggedDictBuilder;
use nu_engine::CommandArgs;
use nu_errors::ShellError;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};
use nu_source::Tag;
use nu_stream::OutputStream;
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

    fn run(&self, args: CommandArgs) -> Result<OutputStream, ShellError> {
        run_tutorial_page(self.state.clone(), args)
    }
}

fn run_tutorial_page(state: Arc<State>, args: CommandArgs) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once()?;

    let name = match args.nth(0) {
        Some(a) => Some(a.as_string()?),
        None => None,
    };

    let tutorial = state.tutorial();
    if let Some(n) = name {
        Ok(OutputStream::one(
            UntaggedValue::string(tutorial.goto_step(n)?).into_value(Tag::unknown()),
        ))
    } else {
        let mut results: Vec<Value> = vec![];
        let (current_step, steps) = tutorial.step_names();
        for s in steps {
            let mut collected = TaggedDictBuilder::new(Tag::default());
            let mut step_name = s.clone();
            if s == current_step {
                step_name = step_name + " (active)";
            }
            collected.insert_value("page_name", step_name);
            results.push(collected.into_value());
        }
        Ok(OutputStream::from(results))
    }
}
