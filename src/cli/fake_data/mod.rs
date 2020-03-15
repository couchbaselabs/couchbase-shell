use super::util::convert_json_value_to_nu_value;
use crate::state::State;
use fake::faker::internet::raw::*;
use fake::faker::name::raw::*;
use fake::locales::*;
use fake::Fake;
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
        Signature::build("fake")
            .named("template", SyntaxShape::Path, "path to the template", None)
            .named(
                "num-rows",
                SyntaxShape::Int,
                "number of rows to generate",
                None,
            )
            .switch(
                "list-functions",
                "List all functions currently registered",
                None,
            )
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

    let list_functions = args.get("list-functions").is_some();

    let ctx = Context::new();
    let mut tera = Tera::default();

    tera.register_function("uuid", fake_uuid);
    tera.register_function("name", fake_name);
    tera.register_function("firstName", fake_first_name);
    tera.register_function("lastName", fake_last_name);
    tera.register_function("title", fake_title);
    tera.register_function("nameWithTitle", fake_name_with_title);
    tera.register_function("userName", fake_username);

    let mut results = Vec::new();

    if list_functions {
        let generated = tera.render_str(LIST_FUNCTIONS, &ctx).unwrap();
        let content = serde_json::from_str(&generated).unwrap();
        match content {
            serde_json::Value::Array(values) => {
                for value in values {
                    let content_converted = convert_json_value_to_nu_value(&value, Tag::default());
                    results.push(content_converted);
                }
            }
            _ => unimplemented!(),
        }
    } else {
        let path = args.get("template").ok_or_else(|| {
            ShellError::labeled_error(
                "No file or directory specified",
                "for command",
                Tag::default(),
            )
        })?;

        let num_rows = args
            .get("num-rows")
            .map(|v| v.as_u64().unwrap())
            .unwrap_or(1);
        let path = path.as_path().unwrap();
        let template = fs::read_to_string(path).unwrap();

        for _ in 0..num_rows {
            let generated = tera.render_str(&template, &ctx).unwrap();
            let content = serde_json::from_str(&generated).unwrap();
            let content_converted = convert_json_value_to_nu_value(&content, Tag::default());
            results.push(content_converted);
        }
    }

    Ok(OutputStream::from(results))
}

static LIST_FUNCTIONS: &str = r#"[
    { "name": "uuid()", "description": "UUID v4", "example": "{{ uuid() }}" },
    { "name": "firstName()", "description": "First name", "example": "{{ firstName() }}" },
    { "name": "lastName()", "description": "Last name", "example": "{{ lastName() }}" },
    { "name": "name()", "description": "firstName and lastName combined", "example": "{{ name() }}" },
    { "name": "title()", "description": "Person title", "example": "{{ title() }}" },
    { "name": "title()", "description": "Person title", "example": "{{ title() }}" },
    { "name": "nameWithTitle()", "description": "name and title combined", "example": "{{ nameWithTitle() }}" },
    { "name": "userName()", "description": "Username", "example": "{{ userName() }}" }
]"#;

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

fn fake_uuid(_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let data: String = format!("{}", uuid::Uuid::new_v4());
    Ok(tera::Value::from(data))
}
