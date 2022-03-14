#![recursion_limit = "256"]

mod cli;
mod client;
mod config;
mod config_files;
mod default_context;
mod state;
mod tutorial;

use crate::config::{
    ShellConfig, DEFAULT_ANALYTICS_TIMEOUT, DEFAULT_DATA_TIMEOUT, DEFAULT_KV_BATCH_SIZE,
    DEFAULT_MANAGEMENT_TIMEOUT, DEFAULT_QUERY_TIMEOUT, DEFAULT_SEARCH_TIMEOUT,
};
use crate::config_files::{read_nu_config_file, CBSHELL_FOLDER};
use crate::default_context::create_default_context;
use crate::state::{RemoteCapellaOrganization, RemoteCluster};
use crate::{cli::*, state::ClusterTimeouts};
use config::ClusterTlsConfig;
use env_logger::Env;
use isahc::{prelude::*, Request};
use log::{debug, warn, LevelFilter};
use log::{error, info};
use nu_cli::{add_plugin_file, gather_parent_env_vars, read_plugin_file, report_error};
use nu_command::BufferedReader;
use nu_engine::{get_full_help, CallExt};
use nu_parser::parse;
use nu_protocol::ast::{Call, Expr, Expression};
use nu_protocol::engine::{Command, EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    Category, Example, IntoPipelineData, PipelineData, RawStream, ShellError, Signature, Span,
    Spanned, SyntaxShape, Value, CONFIG_VARIABLE_ID,
};
use serde::Deserialize;
use state::State;
use std::collections::HashMap;
use std::error::Error;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use temp_dir::TempDir;

fn main() -> Result<(), Box<dyn Error>> {
    let mut logger_builder = env_logger::Builder::from_env(
        Env::default().filter_or("CBSH_LOG", "info,isahc=error,surf=error"),
    );
    logger_builder.format(|buf, record| {
        let mut style = buf.style();
        style.set_intense(true);
        style.set_bold(true);
        writeln!(
            buf,
            "{}: {}",
            buf.default_styled_level(record.level()),
            style.value(record.args())
        )
    });

    let init_cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(_) => match std::env::var("PWD") {
            Ok(cwd) => PathBuf::from(cwd),
            Err(_) => match nu_path::home_dir() {
                Some(cwd) => cwd,
                None => PathBuf::new(),
            },
        },
    };
    let mut context = create_default_context(&init_cwd);

    gather_parent_env_vars(&mut context);
    let mut stack = nu_protocol::engine::Stack::new();

    stack.vars.insert(
        CONFIG_VARIABLE_ID,
        Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span::new(0, 0),
        },
    );

    let mut args_to_cbshell = vec![];
    let mut args_to_script = vec![];

    let mut collect_arg_script = false;
    let mut collect_arg_filename = false;
    for arg in std::env::args().skip(1) {
        if collect_arg_script {
            if collect_arg_filename {
                args_to_cbshell.push(if arg.contains(' ') {
                    format!("'{}'", arg)
                } else {
                    arg
                });
                collect_arg_filename = false;
            } else {
                args_to_script.push(if arg.contains(' ') {
                    format!("'{}'", arg)
                } else {
                    arg
                });
            }
        } else if arg == "--script" {
            collect_arg_script = true;
            collect_arg_filename = true;
            args_to_cbshell.push(if arg.contains(' ') {
                format!("'{}'", arg)
            } else {
                arg
            });
        } else if arg == "-c" || arg == "--command" {
            args_to_cbshell.push(arg);
        } else {
            args_to_cbshell.push(if arg.contains(' ') {
                format!("'{}'", arg)
            } else {
                arg
            });
        }
    }

    args_to_cbshell.insert(0, "cbsh".into());

    let shell_commandline_args = args_to_cbshell.join(" ");

    let opt = match parse_commandline_args(&shell_commandline_args, &init_cwd, &mut context) {
        Ok(o) => o,
        Err(_) => std::process::exit(1),
    };

    if opt.silent {
        logger_builder.filter_level(LevelFilter::Error);
    }
    logger_builder.init();

    debug!("Effective {:?}", opt);

    let config = ShellConfig::new();
    debug!("Config {:?}", config);

    const DEFAULT_PASSWORD: &str = "password";
    const DEFAULT_HOSTNAME: &str = "localhost";
    const DEFAULT_USERNAME: &str = "Administrator";

    let mut clusters = HashMap::new();
    let mut capella_orgs = HashMap::new();

    let password = match opt.password {
        true => Some(rpassword::read_password_from_tty(Some("Password: ")).unwrap()),
        false => None,
    };

    let mut active_capella_org = None;
    let active = if config.clusters().is_empty() && config.capella_orgs().is_empty() {
        let hostnames = if let Some(hosts) = opt.hostnames {
            hosts
        } else {
            DEFAULT_HOSTNAME.into()
        };

        let username = if let Some(user) = opt.username {
            user
        } else {
            DEFAULT_USERNAME.into()
        };

        let rpassword = if let Some(pass) = password {
            pass
        } else {
            DEFAULT_PASSWORD.into()
        };

        let tls_config = ClusterTlsConfig::new(
            !opt.disable_tls,
            opt.tls_cert_path.clone(),
            !opt.dont_validate_hostnames,
            opt.tls_cert_path.is_none(),
        );
        if !tls_config.enabled() {
            warn!(
                "Using PLAIN authentication for cluster default, credentials will sent in plaintext - configure tls to disable this warning"
            );
        }
        let cluster = RemoteCluster::new(
            validate_hostnames(hostnames.split(',').map(|v| v.to_owned()).collect()),
            username,
            rpassword,
            opt.bucket,
            opt.scope,
            opt.collection,
            tls_config,
            ClusterTimeouts::default(),
            None,
            DEFAULT_KV_BATCH_SIZE,
        );
        clusters.insert("default".into(), cluster);
        String::from("default")
    } else {
        let mut active = None;
        for v in config.clusters() {
            let name = v.identifier().to_owned();

            let mut username = v.username();
            let mut cpassword = v.password();
            let mut default_bucket = v.default_bucket();
            let mut scope = v.default_scope();
            let mut collection = v.default_collection();

            if opt.cluster.as_ref().is_some() {
                if &name == opt.cluster.as_ref().unwrap() {
                    active = Some(name.clone());
                    if let Some(user) = opt.username.clone() {
                        username = user;
                    }
                    if let Some(pass) = password.clone() {
                        cpassword = pass;
                    }
                    if let Some(bucket) = opt.bucket.clone() {
                        default_bucket = Some(bucket);
                    }
                    if let Some(s) = opt.scope.clone() {
                        scope = Some(s);
                    }
                    if let Some(c) = opt.collection.clone() {
                        collection = Some(c);
                    }
                }
            } else if active.is_none() {
                active = Some(v.identifier().to_owned());
                if let Some(user) = opt.username.clone() {
                    username = user;
                }
                if let Some(pass) = password.clone() {
                    cpassword = pass;
                }
                if let Some(bucket) = opt.bucket.clone() {
                    default_bucket = Some(bucket);
                }
                if let Some(s) = opt.scope.clone() {
                    scope = Some(s);
                }
                if let Some(c) = opt.collection.clone() {
                    collection = Some(c);
                }
            }

            let timeouts = v.timeouts();
            let data_timeout = match timeouts.data_timeout() {
                Some(t) => t.to_owned(),
                None => DEFAULT_DATA_TIMEOUT,
            };
            let query_timeout = match timeouts.query_timeout() {
                Some(t) => t.to_owned(),
                None => DEFAULT_QUERY_TIMEOUT,
            };
            let analytics_timeout = match timeouts.analytics_timeout() {
                Some(t) => t.to_owned(),
                None => DEFAULT_ANALYTICS_TIMEOUT,
            };
            let search_timeout = match timeouts.search_timeout() {
                Some(t) => t.to_owned(),
                None => DEFAULT_SEARCH_TIMEOUT,
            };
            let management_timeout = match timeouts.management_timeout() {
                Some(t) => t.to_owned(),
                None => DEFAULT_MANAGEMENT_TIMEOUT,
            };
            let kv_batch_size = match v.kv_batch_size() {
                Some(b) => b,
                None => DEFAULT_KV_BATCH_SIZE,
            };

            let cluster = RemoteCluster::new(
                validate_hostnames(v.hostnames().clone()),
                username,
                cpassword,
                default_bucket,
                scope,
                collection,
                v.tls().clone(),
                ClusterTimeouts::new(
                    data_timeout,
                    query_timeout,
                    analytics_timeout,
                    search_timeout,
                    management_timeout,
                ),
                v.cloud_org(),
                kv_batch_size,
            );
            if !v.tls().clone().enabled() {
                warn!(
                    "Using PLAIN authentication for cluster {}, credentials will sent in plaintext - configure tls to disable this warning",
                    name.clone()
                );
            }
            clusters.insert(name.clone(), cluster);
        }
        for c in config.capella_orgs() {
            let management_timeout = match c.management_timeout() {
                Some(t) => t.to_owned(),
                None => DEFAULT_MANAGEMENT_TIMEOUT,
            };
            let name = c.identifier();
            let default_cloud = c.default_cloud();
            let default_project = c.default_project();

            let plane = RemoteCapellaOrganization::new(
                c.secret_key(),
                c.access_key(),
                management_timeout,
                default_project,
                default_cloud,
            );

            if active_capella_org.is_none() {
                active_capella_org = Some(name.clone());
            }

            capella_orgs.insert(name, plane);
        }

        active.unwrap_or_else(|| "".into())
    };

    let state = Arc::new(Mutex::new(State::new(
        clusters,
        active,
        config.location().clone(),
        capella_orgs,
        active_capella_org,
    )));

    if !opt.silent && !opt.no_motd && opt.script.is_none() && opt.command.is_none() {
        fetch_and_print_motd();
    }

    let ctrlc = Arc::new(AtomicBool::new(false));
    let handler_ctrlc = ctrlc.clone();
    let context_ctrlc = ctrlc.clone();

    ctrlc::set_handler(move || {
        handler_ctrlc.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    context.ctrlc = Some(context_ctrlc);

    let delta = {
        let mut working_set = nu_protocol::engine::StateWorkingSet::new(&context);
        working_set.add_decl(Box::new(AllowLists::new(state.clone())));
        working_set.add_decl(Box::new(AllowListsAdd::new(state.clone())));
        working_set.add_decl(Box::new(AllowListsDrop::new(state.clone())));
        working_set.add_decl(Box::new(Analytics::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsBuckets::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsDatasets::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsDataverses::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsIndexes::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsLinks::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsPendingMutations::new(state.clone())));
        working_set.add_decl(Box::new(Buckets::new(state.clone())));
        working_set.add_decl(Box::new(BucketsConfig::new(state.clone())));
        working_set.add_decl(Box::new(BucketsCreate::new(state.clone())));
        working_set.add_decl(Box::new(BucketsDrop::new(state.clone())));
        working_set.add_decl(Box::new(BucketsFlush::new(state.clone())));
        working_set.add_decl(Box::new(BucketsGet::new(state.clone())));
        working_set.add_decl(Box::new(BucketsSample::new(state.clone())));
        working_set.add_decl(Box::new(BucketsUpdate::new(state.clone())));
        working_set.add_decl(Box::new(Clouds::new(state.clone())));
        working_set.add_decl(Box::new(Clusters::new(state.clone())));
        working_set.add_decl(Box::new(ClustersCreate::new(state.clone())));
        working_set.add_decl(Box::new(ClustersDrop::new(state.clone())));
        working_set.add_decl(Box::new(ClustersGet::new(state.clone())));
        working_set.add_decl(Box::new(ClustersHealth::new(state.clone())));
        working_set.add_decl(Box::new(ClustersManaged::new(state.clone())));
        working_set.add_decl(Box::new(ClustersRegister::new(state.clone())));
        working_set.add_decl(Box::new(ClustersUnregister::new(state.clone())));
        working_set.add_decl(Box::new(Collections::new(state.clone())));
        working_set.add_decl(Box::new(CollectionsCreate::new(state.clone())));
        working_set.add_decl(Box::new(CollectionsDrop::new(state.clone())));
        working_set.add_decl(Box::new(Doc));
        working_set.add_decl(Box::new(DocGet::new(state.clone())));
        working_set.add_decl(Box::new(DocInsert::new(state.clone())));
        working_set.add_decl(Box::new(DocReplace::new(state.clone())));
        working_set.add_decl(Box::new(DocRemove::new(state.clone())));
        working_set.add_decl(Box::new(DocUpsert::new(state.clone())));
        working_set.add_decl(Box::new(Help));
        working_set.add_decl(Box::new(FakeData::new(state.clone())));
        working_set.add_decl(Box::new(Nodes::new(state.clone())));
        working_set.add_decl(Box::new(Ping::new(state.clone())));
        working_set.add_decl(Box::new(Projects::new(state.clone())));
        working_set.add_decl(Box::new(ProjectsCreate::new(state.clone())));
        working_set.add_decl(Box::new(ProjectsDrop::new(state.clone())));
        working_set.add_decl(Box::new(Query::new(state.clone())));
        working_set.add_decl(Box::new(QueryAdvise::new(state.clone())));
        working_set.add_decl(Box::new(QueryIndexes::new(state.clone())));
        working_set.add_decl(Box::new(Scopes::new(state.clone())));
        working_set.add_decl(Box::new(ScopesCreate::new(state.clone())));
        working_set.add_decl(Box::new(ScopesDrop::new(state.clone())));
        working_set.add_decl(Box::new(Search::new(state.clone())));
        working_set.add_decl(Box::new(Transactions));
        working_set.add_decl(Box::new(TransactionsListAtrs::new(state.clone())));
        working_set.add_decl(Box::new(Tutorial::new(state.clone())));
        working_set.add_decl(Box::new(TutorialNext::new(state.clone())));
        working_set.add_decl(Box::new(TutorialPage::new(state.clone())));
        working_set.add_decl(Box::new(TutorialPrev::new(state.clone())));
        working_set.add_decl(Box::new(UseBucket::new(state.clone())));
        working_set.add_decl(Box::new(UseCapellaOrganization::new(state.clone())));
        working_set.add_decl(Box::new(UseCloud::new(state.clone())));
        working_set.add_decl(Box::new(UseCluster::new(state.clone())));
        working_set.add_decl(Box::new(UseCmd::new(state.clone())));
        working_set.add_decl(Box::new(UseCollection::new(state.clone())));
        working_set.add_decl(Box::new(UseProject::new(state.clone())));
        working_set.add_decl(Box::new(UseScope::new(state.clone())));
        working_set.add_decl(Box::new(UseTimeouts::new(state.clone())));
        working_set.add_decl(Box::new(Users::new(state.clone())));
        working_set.add_decl(Box::new(Users::new(state.clone())));
        working_set.add_decl(Box::new(UsersDrop::new(state.clone())));
        working_set.add_decl(Box::new(UsersRoles::new(state.clone())));
        working_set.add_decl(Box::new(UsersUpsert::new(state.clone())));
        working_set.add_decl(Box::new(Version));
        working_set.add_decl(Box::new(Whoami::new(state.clone())));

        working_set.render()
    };
    let _ = context.merge_delta(delta, None, &init_cwd);

    let input = if opt.stdin {
        let stdin = std::io::stdin();
        let buf_reader = BufReader::new(stdin);

        PipelineData::ExternalStream {
            stdout: Some(RawStream::new(
                Box::new(BufferedReader::new(buf_reader)),
                Some(ctrlc),
                Span::new(0, 0),
            )),
            stderr: None,
            exit_code: None,
            span: Span::new(0, 0),
            metadata: None,
        }
    } else {
        PipelineData::new(Span::new(0, 0))
    };

    if let Some(c) = opt.command {
        add_plugin_file(&mut context, CBSHELL_FOLDER);
        nu_cli::evaluate_commands(&c, &init_cwd, &mut context, &mut stack, input, false)
            .expect("Failed to run command");
        return Ok(());
    }

    if let Some(filepath) = opt.script {
        add_plugin_file(&mut context, CBSHELL_FOLDER);
        let _ret_val = nu_cli::evaluate_file(
            filepath,
            &args_to_script,
            &mut context,
            &mut stack,
            input,
            false,
        )
        .expect("Failed to run script");

        return Ok(());
    }

    let d = TempDir::new().unwrap();
    let f = d.child("config.nu");

    let prompt = if cfg!(windows) {
        r##"let-env PROMPT_COMMAND = {build-string (ansi ub) (cb-env | get username) (ansi reset) (ansi yb) (cb-env | get cluster) (ansi reset) ' in  (ansi wb) (cb-env | get bucket) (cb-env | select scope collection | each { |it| if $it.scope == "" && $it.collection == "" { } else { build-string (if $it.scope == "" { build-string ".<notset>" } else {build-string "." $it.scope}) (if $it.collection == "" { build-string ".<notset>"} else {build-string "." $it.collection})}}) (ansi reset)}"##
    } else {
        r##"let-env PROMPT_COMMAND = {build-string '👤 ' (ansi ub) (cb-env | get username) (ansi reset) ' 🏠 ' (ansi yb) (cb-env | get cluster) (ansi reset) ' in 🗄 ' (ansi wb) (cb-env | get bucket) (cb-env | select scope collection | each { |it| if $it.scope == "" && $it.collection == "" { } else { build-string (if $it.scope == "" { build-string ".<notset>" } else {build-string "." $it.scope}) (if $it.collection == "" { build-string ".<notset>"} else {build-string "." $it.collection})}}) (ansi reset)}"##
    };

    let config_string = format!(
        "{}\nlet-env PROMPT_INDICATOR = \"\r\n> \"\nlet-env PROMPT_COMMAND_RIGHT = \"\"",
        prompt
    );

    std::fs::write(&f, config_string.as_bytes()).unwrap();

    read_plugin_file(&mut context, &mut stack, CBSHELL_FOLDER, false);
    read_nu_config_file(&mut context, &mut stack, f);
    let history_path = config_files::create_history_path(config);

    nu_cli::evaluate_repl(&mut context, &mut stack, history_path, false)
        .expect("evaluate loop failed");
    // nu_cli::evaluate_repl(&mut context, None, false).expect("evaluate loop failed");
    Ok(())
}

/// Fetches a helpful MOTD from couchbase.sh
///
/// Note that this can be disabled with the --no-motd cli flag if needed.
fn fetch_and_print_motd() {
    let agent = format!(
        "cbsh {} {}/{}",
        option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    let mut response = match Request::get("http://motd.couchbase.sh/motd")
        .timeout(Duration::from_millis(500))
        .header("User-Agent", agent)
        .body(())
        .expect("An empty body should not cause a panic - ignoring.")
        .send()
    {
        Ok(r) => r,
        Err(_e) => {
            debug!("Failed to load MOTD, ignoring.");
            return;
        }
    };

    let encoded = match response.text() {
        Ok(v) => v,
        Err(_e) => {
            debug!("Could not decode MOTD, ignoring.");
            return;
        }
    };

    let motd: Motd = match serde_json::from_str(encoded.as_str()) {
        Ok(v) => v,
        Err(_e) => {
            debug!("Failed to turn MOTD into JSON, ignoring.");
            return;
        }
    };

    info!("{}", motd.msg);
}

#[derive(Debug, Deserialize)]
struct Motd {
    msg: String,
}

#[derive(Clone, Debug)]
struct CliOptions {
    hostnames: Option<String>,
    username: Option<String>,
    password: bool,
    cluster: Option<String>,
    bucket: Option<String>,
    scope: Option<String>,
    collection: Option<String>,
    command: Option<Spanned<String>>,
    script: Option<String>,
    stdin: bool,
    no_motd: bool,
    disable_tls: bool,
    dont_validate_hostnames: bool,
    tls_cert_path: Option<String>,
    silent: bool,
}

#[derive(Clone)]
struct Cbsh;

impl Command for Cbsh {
    fn name(&self) -> &str {
        "cbsh"
    }

    fn signature(&self) -> Signature {
        Signature::build("cbsh")
            .desc("The Couchbase Shell.")
            .named(
                "hostnames",
                SyntaxShape::String,
                "hostnames to connect to",
                None,
            )
            .named(
                "username",
                SyntaxShape::String,
                "username to authenticate as",
                Some('u'),
            )
            .switch(
                "password",
                "use to specify a password to use for authentication",
                Some('p'),
            )
            .named(
                "cluster",
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
            .switch(
                "dont-validate-hostnames",
                "disable validation of hostnames for TLS certificates",
                None,
            )
            .named(
                "tls-cert-path",
                SyntaxShape::String,
                "path to certificate to use for TLS",
                None,
            )
            .switch("silent", "run in silent mode", Some('s'))
            .switch("version", "print the version", Some('v'))
            .category(Category::Custom("couchbase".into()))
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
    ) -> Result<nu_protocol::PipelineData, nu_protocol::ShellError> {
        Ok(Value::String {
            val: get_full_help(&Cbsh.signature(), &Cbsh.examples(), context, stack),
            span: call.head,
        }
        .into_pipeline_data())
    }

    fn examples(&self) -> Vec<nu_protocol::Example> {
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

fn parse_commandline_args(
    commandline_args: &str,
    init_cwd: &Path,
    context: &mut EngineState,
) -> Result<CliOptions, ShellError> {
    let (block, delta) = {
        let mut working_set = StateWorkingSet::new(context);
        working_set.add_decl(Box::new(Cbsh));

        let (output, err) = parse(&mut working_set, None, commandline_args.as_bytes(), false);
        if let Some(err) = err {
            report_error(&working_set, &err);

            std::process::exit(1);
        }

        working_set.hide_decl(b"cbsh");
        (output, working_set.render())
    };

    let _ = context.merge_delta(delta, None, init_cwd);

    let mut stack = Stack::new();
    stack.add_var(
        CONFIG_VARIABLE_ID,
        Value::Record {
            cols: vec![],
            vals: vec![],
            span: Span::new(0, 0),
        },
    );

    // We should have a successful parse now
    if let Some(pipeline) = block.pipelines.get(0) {
        if let Some(Expression {
            expr: Expr::Call(call),
            ..
        }) = pipeline.expressions.get(0)
        {
            let hostnames: Option<String> = call.get_flag(context, &mut stack, "hostnames")?;
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
            let dont_validate_hostnames = call.has_flag("dont-validate-hostnames");
            let tls_cert_path: Option<String> =
                call.get_flag(context, &mut stack, "tls-cert-path")?;
            let silent = call.has_flag("silent");

            fn extract_contents(
                expression: Option<Expression>,
                context: &mut EngineState,
            ) -> Option<Spanned<String>> {
                expression.map(|expr| {
                    let contents = context.get_span_contents(&expr.span);

                    Spanned {
                        item: String::from_utf8_lossy(contents).to_string(),
                        span: expr.span,
                    }
                })
            }

            let command = extract_contents(command, context);

            let help = call.has_flag("help");

            if help {
                let full_help =
                    get_full_help(&Cbsh.signature(), &Cbsh.examples(), context, &mut stack);

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
                hostnames,
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
                dont_validate_hostnames,
                tls_cert_path,
                silent,
            });
        }
    }

    // Just give the help and exit if the above fails
    let full_help = get_full_help(&Cbsh.signature(), &Cbsh.examples(), context, &mut stack);
    print!("{}", full_help);
    std::process::exit(1);
}

fn validate_hostnames(hostnames: Vec<String>) -> Vec<String> {
    let mut validated = vec![];
    for hostname in hostnames {
        let host = if let Some(stripped_couchbase) = hostname.strip_prefix("couchbase://") {
            if let Some(stripped_port) = stripped_couchbase.strip_suffix(":11210") {
                stripped_port.to_string()
            } else if stripped_couchbase.contains(':') {
                error!("Couchbase scheme and non-default port detected, http scheme must be used with custom port (management port)");
                std::process::exit(1);
            } else {
                stripped_couchbase.to_string()
            }
        } else if let Some(stripped_couchbase) = hostname.strip_prefix("couchbases://") {
            if let Some(stripped_port) = stripped_couchbase.strip_suffix(":11211") {
                stripped_port.to_string()
            } else if stripped_couchbase.contains(':') {
                error!("Couchbases scheme and non-default port detected, http scheme must be used with custom port (management port)");
                std::process::exit(1);
            } else {
                stripped_couchbase.to_string()
            }
        } else if hostname.strip_suffix(":11210").is_some() {
            error!("Memcached port detected, http scheme must be used with custom port (management port)");
            std::process::exit(1);
        } else if hostname.strip_suffix(":11211").is_some() {
            error!("Memcached port detected, http scheme must be used with custom port (management port)");
            std::process::exit(1);
        } else if let Some(stripped_http) = hostname.strip_prefix("http://") {
            stripped_http.to_string()
        } else if let Some(stripped_http) = hostname.strip_prefix("https://") {
            stripped_http.to_string()
        } else {
            hostname
        };

        validated.push(host);
    }

    validated
}
