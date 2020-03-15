use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use futures::executor::block_on;
use nu_cli::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use nu_protocol::{Signature, SyntaxShape};
use nu_source::Tag;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tera::{Context, Tera};

pub struct FakeData {
    state: Arc<State>,
}

impl FakeData {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

impl nu_cli::WholeStreamCommand for FakeData {
    fn name(&self) -> &str {
        "fake"
    }

    fn signature(&self) -> Signature {
        Signature::build("fake").named("template", SyntaxShape::Path, "path to the template", None)
    }

    fn usage(&self) -> &str {
        "Creates fake data from a template"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        block_on(run_fake(self.state.clone(), args, registry))
    }
}

async fn run_fake(
    _state: Arc<State>,
    args: CommandArgs,
    registry: &CommandRegistry,
) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;

    let path = args.get("template").ok_or_else(|| {
        ShellError::labeled_error(
            "No file or directory specified",
            "for command",
            Tag::default(),
        )
    })?;

    let path = path.as_path().unwrap();
    let template = fs::read_to_string(path).unwrap();

    let ctx = Context::new();
    let mut tera = Tera::default();

    tera.register_function("name", fake_name);
    tera.register_function("firstName", fake_first_name);
    tera.register_function("lastName", fake_last_name);
    tera.register_function("title", fake_title);
    tera.register_function("nameWithTitle", fake_name_with_title);
    tera.register_function("username", fake_username);

    let mut results = Vec::new();

    let generated = tera.render_str(&template, &ctx).unwrap();
    let content = serde_json::from_str(&generated).unwrap();
    let content_converted = convert_json_value_to_nu_value(&content, Tag::default());
    results.push(content_converted);

    Ok(OutputStream::from(results))
}

use fake::faker::internet::raw::*;
use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;

fn fake_name(_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let data: String = Name(EN).fake();
    Ok(tera::Value::from(data))
}

fn fake_name_with_title(_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let data: String = NameWithTitle(EN).fake();
    Ok(tera::Value::from(data))
}

fn fake_first_name(_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let data: String = FirstName(EN).fake();
    Ok(tera::Value::from(data))
}

fn fake_last_name(_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let data: String = LastName(EN).fake();
    Ok(tera::Value::from(data))
}

fn fake_title(_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let data: String = Title(EN).fake();
    Ok(tera::Value::from(data))
}

fn fake_username(_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let data: String = Username(EN).fake();
    Ok(tera::Value::from(data))
}
