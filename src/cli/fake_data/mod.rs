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
use serde_json::{from_value, Value};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tera::{Context, Tera};
use uuid::Uuid;

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

    register_functions(&mut tera);

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

fn register_functions(tera: &mut Tera) {
    // Group "misc"
    tera.register_function("uuid", |_: &HashMap<String, Value>| {
        Ok(Value::from(format!("{}", Uuid::new_v4())))
    });

    // Group "name"
    tera.register_function("name", |_: &HashMap<String, Value>| {
        Ok(Value::from(Name(EN).fake::<String>()))
    });
    tera.register_function("firstName", |_: &HashMap<String, Value>| {
        Ok(Value::from(FirstName(EN).fake::<String>()))
    });
    tera.register_function("lastName", |_: &HashMap<String, Value>| {
        Ok(Value::from(LastName(EN).fake::<String>()))
    });
    tera.register_function("title", |_: &HashMap<String, Value>| {
        Ok(Value::from(Title(EN).fake::<String>()))
    });
    tera.register_function("nameWithTitle", |_: &HashMap<String, Value>| {
        Ok(Value::from(NameWithTitle(EN).fake::<String>()))
    });
    tera.register_function("suffix", |_: &HashMap<String, Value>| {
        Ok(Value::from(Suffix(EN).fake::<String>()))
    });

    // Group "internet"
    tera.register_function("color", |_: &HashMap<String, Value>| {
        Ok(Value::from(Color(EN).fake::<String>()))
    });
    tera.register_function("domainSuffix", |_: &HashMap<String, Value>| {
        Ok(Value::from(DomainSuffix(EN).fake::<String>()))
    });
    tera.register_function("freeEmail", |_: &HashMap<String, Value>| {
        Ok(Value::from(FreeEmail(EN).fake::<String>()))
    });
    tera.register_function("freeEmailProvider", |_: &HashMap<String, Value>| {
        Ok(Value::from(FreeEmailProvider(EN).fake::<String>()))
    });
    tera.register_function("ipV4", |_: &HashMap<String, Value>| {
        Ok(Value::from(IPv4(EN).fake::<String>()))
    });
    tera.register_function("ipV6", |_: &HashMap<String, Value>| {
        Ok(Value::from(IPv6(EN).fake::<String>()))
    });
    tera.register_function("password", |args: &HashMap<String, Value>| {
        let length = match args.get("length") {
            Some(val) => match from_value::<usize>(val.clone()) {
                Ok(v) => v,
                Err(_) => {
                    return Err(tera::Error::msg(format!(
                        "Function `password` received length={} but `length` can only be a number",
                        val
                    )));
                }
            },
            None => 10,
        };
        Ok(Value::from(
            Password(EN, length..length + 1).fake::<String>(),
        ))
    });
    tera.register_function("safeEmail", |_: &HashMap<String, Value>| {
        Ok(Value::from(SafeEmail(EN).fake::<String>()))
    });
    tera.register_function("userAgent", |_: &HashMap<String, Value>| {
        Ok(Value::from(UserAgent(EN).fake::<String>()))
    });
    tera.register_function("userName", |_: &HashMap<String, Value>| {
        Ok(Value::from(Username(EN).fake::<String>()))
    });
}

static LIST_FUNCTIONS: &str = r#"[
    { "group": "misc", "name": "uuid()", "description": "UUID v4", "example": "{{ uuid() }}" },
    { "group": "name", "name": "firstName()", "description": "First name", "example": "{{ firstName() }}" },
    { "group": "name", "name": "lastName()", "description": "Last name", "example": "{{ lastName() }}" },
    { "group": "name", "name": "name()", "description": "firstName and lastName combined", "example": "{{ name() }}" },
    { "group": "name", "name": "title()", "description": "Person title", "example": "{{ title() }}" },
    { "group": "name", "name": "nameWithTitle()", "description": "name and title combined", "example": "{{ nameWithTitle() }}" },    
    { "group": "name", "name": "suffix()", "description": "Person info/degree", "example": "{{ suffix() }}" },
    { "group": "internet", "name": "color()", "description": "Color hex code", "example": "{{ color() }}" },
    { "group": "internet", "name": "domainSuffix()", "description": "Domain suffix", "example": "{{ domainSuffix() }}" },
    { "group": "internet", "name": "freeEmail()", "description": "Email that might exist", "example": "{{ freeEmail() }}" },
    { "group": "internet", "name": "freeEmailProvider()", "description": "Provider that exists", "example": "{{ freeEmailProvider() }}" },
    { "group": "internet", "name": "ipV4()", "description": "IP v4 address", "example": "{{ ipV4() }}" },
    { "group": "internet", "name": "ipV6()", "description": "IP v6 address", "example": "{{ ipV6() }}" },
    { "group": "internet", "name": "userAgent()", "description": "User Agent", "example": "{{ userAgent() }}" },
    { "group": "internet", "name": "safeEmail()", "description": "Email that does not exist", "example": "{{ safeEmail() }}" },
    { "group": "internet", "name": "userName()", "description": "Username", "example": "{{ userName() }}" },
    { "group": "internet", "name": "password(length=10)", "description": "Password", "example": "{{ password() }}" }
]"#;
