use serde::Deserialize;

use nu_engine::{get_full_help, CallExt};
use nu_protocol::ast::Call;
use nu_protocol::engine::{Command, EngineState, Stack};
use nu_protocol::{
    span, Category, IntoInterruptiblePipelineData, IntoPipelineData, PipelineData, ShellError,
    Signature, Spanned, SyntaxShape, Value,
};

#[derive(Clone)]
pub struct Help;

#[derive(Deserialize)]
pub struct HelpArgs {}

impl Command for Help {
    fn name(&self) -> &str {
        "help"
    }

    fn signature(&self) -> Signature {
        Signature::build("help")
            .rest(
                "rest",
                SyntaxShape::String,
                "the name of command to get help on",
            )
            .named(
                "find",
                SyntaxShape::String,
                "string to find in command usage",
                Some('f'),
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Display help information about commands."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        help(engine_state, stack, call)
    }
}

fn help(
    engine_state: &EngineState,
    stack: &mut Stack,
    call: &Call,
) -> Result<PipelineData, ShellError> {
    let head = call.head;
    let find: Option<Spanned<String>> = call.get_flag(engine_state, stack, "find")?;
    let rest: Vec<Spanned<String>> = call.rest(engine_state, stack, 0)?;

    let full_commands = engine_state.get_signatures_with_examples(false);

    if let Some(f) = find {
        let search_string = f.item.to_lowercase();
        let mut found_cmds_vec = Vec::new();

        for (sig, _, is_plugin, is_custom) in full_commands {
            let mut cols = vec![];
            let mut vals = vec![];

            let key = sig.name.clone();
            let c = sig.usage.clone();
            let e = sig.extra_usage.clone();
            if key.to_lowercase().contains(&search_string)
                || c.to_lowercase().contains(&search_string)
                || e.to_lowercase().contains(&search_string)
            {
                cols.push("name".to_string());
                vals.push(Value::String {
                    val: key,
                    span: head,
                });

                cols.push("category".to_string());
                vals.push(Value::String {
                    val: sig.category.to_string(),
                    span: head,
                });

                cols.push("is_plugin".to_string());
                vals.push(Value::Bool {
                    val: is_plugin,
                    span: head,
                });

                cols.push("is_custom".to_string());
                vals.push(Value::Bool {
                    val: is_custom,
                    span: head,
                });

                cols.push("usage".to_string());
                vals.push(Value::String { val: c, span: head });

                cols.push("extra_usage".to_string());
                vals.push(Value::String { val: e, span: head });

                found_cmds_vec.push(Value::Record {
                    cols,
                    vals,
                    span: head,
                });
            }
        }

        return Ok(found_cmds_vec
            .into_iter()
            .into_pipeline_data(engine_state.ctrlc.clone()));
    }

    if !rest.is_empty() {
        let mut found_cmds_vec = Vec::new();

        if rest[0].item == "commands" {
            for (sig, _, is_plugin, is_custom) in full_commands {
                let mut cols = vec![];
                let mut vals = vec![];

                let key = sig.name.clone();
                let c = sig.usage.clone();
                let e = sig.extra_usage.clone();

                cols.push("name".to_string());
                vals.push(Value::String {
                    val: key,
                    span: head,
                });

                cols.push("category".to_string());
                vals.push(Value::String {
                    val: sig.category.to_string(),
                    span: head,
                });

                cols.push("is_plugin".to_string());
                vals.push(Value::Bool {
                    val: is_plugin,
                    span: head,
                });

                cols.push("is_custom".to_string());
                vals.push(Value::Bool {
                    val: is_custom,
                    span: head,
                });

                cols.push("usage".to_string());
                vals.push(Value::String { val: c, span: head });

                cols.push("extra_usage".to_string());
                vals.push(Value::String { val: e, span: head });

                found_cmds_vec.push(Value::Record {
                    cols,
                    vals,
                    span: head,
                });
            }

            Ok(found_cmds_vec
                .into_iter()
                .into_pipeline_data(engine_state.ctrlc.clone()))
        } else {
            let mut name = String::new();

            for r in &rest {
                if !name.is_empty() {
                    name.push(' ');
                }
                name.push_str(&r.item);
            }

            let output = full_commands
                .iter()
                .filter(|(signature, _, _, _)| signature.name == name)
                .map(|(signature, examples, _, _)| {
                    get_full_help(signature, examples, engine_state, stack)
                })
                .collect::<Vec<String>>();

            if !output.is_empty() {
                Ok(Value::String {
                    val: output.join("======================\n\n"),
                    span: call.head,
                }
                .into_pipeline_data())
            } else {
                Err(ShellError::CommandNotFound(span(&[
                    rest[0].span,
                    rest[rest.len() - 1].span,
                ])))
            }
        }
    } else {
        let msg = r#"Welcome to Couchbase Shell, powered by Nushell. Shell Yeah!

Here are some tips to help you get started.
  * help commands - list all available commands
  * help <command name> - display help about a particular command
  * help commands | where category == "couchbase" - list all available Couchbase specific commands

Nushell works on the idea of a "pipeline". Pipelines are commands connected with the '|' character.
Each stage in the pipeline works together to load, parse, and display information to you.

[Examples]

List the files in the current directory, sorted by size:
    ls | sort-by size

Get all of the buckets of type couchbase in your active cluster:
    buckets get | where type == couchbase

Open a JSON file, transform the data, and upsert it into your active bucket:
    open mydoc.json | wrap content | insert id {echo $it.content.airportname} | doc upsert

You can also learn more at https://couchbase.sh/docs/ and https://www.nushell.sh/book/"#;

        Ok(Value::String {
            val: msg.to_string(),
            span: head,
        }
        .into_pipeline_data())
    }
}
