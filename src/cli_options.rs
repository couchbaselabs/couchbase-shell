use nu_cli::report_error;
use nu_engine::{get_full_help, CallExt};
use nu_parser::{escape_quote_string, parse};
use nu_protocol::ast::{Call, Expr, Expression, PipelineElement};
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Value,
};
use std::io::Write;

#[derive(Clone, Debug)]
pub struct CliOptions {
    pub conn_string: Option<String>,
    pub username: Option<String>,
    pub password: bool,
    pub cluster: Option<String>,
    pub bucket: Option<String>,
    pub scope: Option<String>,
    pub collection: Option<String>,
    pub command: Option<Spanned<String>>,
    pub script: Option<String>,
    pub stdin: bool,
    pub no_motd: bool,
    pub disable_tls: bool,
    pub tls_cert_path: Option<String>,
    pub config_path: Option<String>,
    pub logger_prefix: Option<String>,
    pub display_name: Option<String>,
    pub no_config_prompt: bool,
}

#[derive(Clone)]
struct Cbsh;

impl Command for Cbsh {
    fn name(&self) -> &str {
        "cbsh"
    }

    fn signature(&self) -> Signature {
        Signature::build("cbsh")
            .usage("The Couchbase Shell.")
            .named(
                "conn-string",
                SyntaxShape::String,
                "connection string to use",
                None,
            )
            .named(
                "username",
                SyntaxShape::String,
                "username to authenticate as",
                Some('u'),
            )
            .named(
                "display-name",
                SyntaxShape::String,
                "name to show in the shell",
                None,
            )
            .switch(
                "password",
                "use to specify a password to use for authentication",
                Some('p'),
            )
            .named(
                "database",
                SyntaxShape::String,
                "name to give to this configuration",
                None,
            )
            .named(
                "bucket",
                SyntaxShape::String,
                "name of the bucket to run operations against",
                None,
            )
            .named(
                "scope",
                SyntaxShape::String,
                "name of the scope to run operations against",
                None,
            )
            .named(
                "collection",
                SyntaxShape::String,
                "name of the collection to run operations against",
                None,
            )
            .named(
                "command",
                SyntaxShape::String,
                "command to run without starting an interactive shell session",
                Some('c'),
            )
            .named(
                "script",
                SyntaxShape::String,
                "filename of script to run without starting an interactive shell session",
                None,
            )
            .switch("stdin", "redirect stdin", None)
            .switch("no-motd", "disable message of the day", None)
            .switch("disable-tls", "disable TLS", None)
            .named(
                "tls-cert-path",
                SyntaxShape::String,
                "path to certificate to use for TLS",
                None,
            )
            .switch("version", "print the version", Some('v'))
            .named(
                "config-dir",
                SyntaxShape::String,
                "path to the directory containing the config/credentials files",
                None,
            )
            .named(
                "logger-prefix",
                SyntaxShape::String,
                "prefix to use for each log line",
                None,
            )
            .switch(
                "disable-config-prompt",
                "disable the prompt to create a new config",
                None,
            )
            .category(Category::Custom("couchbase".to_string()))
    }

    fn usage(&self) -> &str {
        "Alternative Shell and UI for Couchbase Server and Capella."
    }

    fn run(
        &self,
        context: &EngineState,
        stack: &mut Stack,
        call: &Call,
        _input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        Ok(Value::String {
            val: get_full_help(
                &Cbsh.signature(),
                &Cbsh.examples(),
                context,
                stack,
                self.is_parser_keyword(),
            ),
            span: call.head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "Run a script",
                example: "cbsh myfile.nu",
                result: None,
            },
            Example {
                description: "Run cbshell interactively (as a shell or REPL)",
                example: "cbsh",
                result: None,
            },
        ]
    }
}

pub fn parse_commandline_args(
    commandline_args: &str,
    context: &mut EngineState,
) -> Result<CliOptions, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(context);
        working_set.add_decl(Box::new(Cbsh));

        let (output, err) = parse(
            &mut working_set,
            None,
            commandline_args.as_bytes(),
            false,
            &[],
        );
        if let Some(err) = err {
            report_error(&working_set, &err);

            std::process::exit(1);
        }

        working_set.hide_decl(b"cbsh");
        (output, working_set.render())
    };

    let _ = context.merge_delta(delta);

    let mut stack = Stack::new();

    // We should have a successful parse now
    if let Some(pipeline) = block.pipelines.get(0) {
        if let Some(PipelineElement::Expression(
            _,
            Expression {
                expr: Expr::Call(call),
                ..
            },
        )) = pipeline.elements.get(0)
        {
            let conn_string: Option<String> = call.get_flag(context, &mut stack, "conn-string")?;
            let username: Option<String> = call.get_flag(context, &mut stack, "username")?;
            let password = call.has_flag("password");
            let cluster: Option<String> = call.get_flag(context, &mut stack, "cluster")?;
            let bucket: Option<String> = call.get_flag(context, &mut stack, "bucket")?;
            let scope: Option<String> = call.get_flag(context, &mut stack, "scope")?;
            let collection: Option<String> = call.get_flag(context, &mut stack, "collection")?;
            let command: Option<Expression> = call.get_flag_expr("command");
            let script: Option<String> = call.get_flag(context, &mut stack, "script")?;
            let stdin = call.has_flag("stdin");
            let no_motd = call.has_flag("no-motd");
            let disable_tls = call.has_flag("disable-tls");
            let tls_cert_path: Option<String> =
                call.get_flag(context, &mut stack, "tls-cert-path")?;
            let config_path: Option<String> = call.get_flag(context, &mut stack, "config-dir")?;
            let logger_prefix: Option<String> =
                call.get_flag(context, &mut stack, "logger-prefix")?;
            let display_name: Option<String> =
                call.get_flag(context, &mut stack, "display-name")?;
            let no_config_prompt = call.has_flag("disable-config-prompt");

            fn extract_contents(
                expression: Option<Expression>,
            ) -> Result<Option<Spanned<String>>, ShellError> {
                if let Some(expr) = expression {
                    let str = expr.as_string();
                    if let Some(str) = str {
                        Ok(Some(Spanned {
                            item: str,
                            span: expr.span,
                        }))
                    } else {
                        Err(ShellError::TypeMismatch("string".to_string(), expr.span))
                    }
                } else {
                    Ok(None)
                }
            }

            let command = extract_contents(command)?;

            let help = call.has_flag("help");

            if help {
                let full_help = get_full_help(
                    &Cbsh.signature(),
                    &Cbsh.examples(),
                    context,
                    &mut stack,
                    false,
                );

                let _ = std::panic::catch_unwind(move || {
                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();
                    let _ = stdout.write_all(full_help.as_bytes());
                });

                std::process::exit(1);
            }

            if call.has_flag("version") {
                let version = env!("CARGO_PKG_VERSION").to_string();
                let _ = std::panic::catch_unwind(move || {
                    let stdout = std::io::stdout();
                    let mut stdout = stdout.lock();
                    let _ = stdout.write_all(format!("{}\n", version).as_bytes());
                });

                std::process::exit(0);
            }

            return Ok(CliOptions {
                conn_string,
                username,
                password,
                cluster,
                bucket,
                scope,
                collection,
                command,
                script,
                stdin,
                no_motd,
                disable_tls,
                tls_cert_path,
                config_path,
                logger_prefix,
                display_name,
                no_config_prompt,
            });
        }
    }

    // Just give the help and exit if the above fails
    let full_help = get_full_help(
        &Cbsh.signature(),
        &Cbsh.examples(),
        context,
        &mut stack,
        false,
    );
    print!("{}", full_help);
    std::process::exit(1);
}

pub fn parse_shell_args() -> (String, Vec<String>) {
    let mut args_to_cbshell = vec![];
    let mut args_to_script = vec![];

    let mut collect_arg_script = false;
    let mut collect_arg_filename = false;
    for arg in std::env::args().skip(1) {
        if collect_arg_script {
            if collect_arg_filename {
                args_to_cbshell.push(if arg.contains(' ') {
                    escape_quote_string(&arg)
                } else {
                    arg
                });
                collect_arg_filename = false;
            } else {
                args_to_script.push(if arg.contains(' ') {
                    escape_quote_string(&arg)
                } else {
                    arg
                });
            }
        } else if arg == "--script" {
            collect_arg_script = true;
            collect_arg_filename = true;
            args_to_cbshell.push(if arg.contains(' ') {
                escape_quote_string(&arg)
            } else {
                arg
            });
        } else if arg == "-c" || arg == "--command" {
            args_to_cbshell.push(arg);
        } else {
            args_to_cbshell.push(if arg.contains(' ') {
                escape_quote_string(&arg)
            } else {
                arg
            });
        }
    }

    args_to_cbshell.insert(0, "cbsh".to_string());

    (args_to_cbshell.join(" "), args_to_script)
}
