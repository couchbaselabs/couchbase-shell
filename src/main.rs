#![recursion_limit = "256"]

extern crate core;

mod cli;
mod cli_options;
mod client;
mod config;
mod config_files;
mod default_context;
mod remote_cluster;
mod state;
#[cfg(unix)]
mod terminal;
mod tutorial;

use crate::cli::*;
use crate::cli_options::{parse_commandline_args, parse_shell_args, CliOptions};
use crate::config::{
    ClusterConfigBuilder, ClusterCredentials, ShellConfig, DEFAULT_ANALYTICS_TIMEOUT,
    DEFAULT_DATA_TIMEOUT, DEFAULT_KV_BATCH_SIZE, DEFAULT_MANAGEMENT_TIMEOUT, DEFAULT_QUERY_TIMEOUT,
    DEFAULT_SEARCH_TIMEOUT, DEFAULT_TRANSACTION_TIMEOUT,
};
use crate::config_files::read_nu_config_file;
use crate::default_context::create_default_context;
use crate::remote_cluster::{
    ClusterTimeouts, RemoteCluster, RemoteClusterResources, RemoteClusterType,
};
use crate::state::Llm;
use crate::state::RemoteCapellaOrganization;
use state::State;

use chrono::Local;
use config::ClusterTlsConfig;
use env_logger::Env;
use log::{debug, warn};
use log::{error, info};
use serde::Deserialize;

use nu_cli::{eval_source, gather_parent_env_vars, read_plugin_file, EvaluateCommandsOpts};
use nu_protocol::engine::{EngineState, Stack, StateWorkingSet};
use nu_protocol::{
    report_shell_error, ByteStream, ByteStreamSource, ByteStreamType, IntoPipelineData,
    PipelineData, PluginIdentity, RegisteredPlugin, Signals, Span, Value,
};

use crate::client::{RustTlsConfig, CLOUD_URL};
use nu_path::canonicalize_with;
use nu_plugin_engine::{GetPlugin, PluginDeclaration};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    let entire_start_time = std::time::Instant::now();

    let init_cwd = current_dir_from_environment();
    let mut context = create_default_context();

    gather_parent_env_vars(&mut context, init_cwd.as_path());
    let mut stack = Stack::new();

    let (shell_commandline_args, args_to_script) = parse_shell_args();

    let opt = match parse_commandline_args(&shell_commandline_args, &mut context) {
        Ok(o) => o,
        Err(_) => std::process::exit(1),
    };

    let opt_clone = opt.clone();
    create_logger_builder(opt_clone.logger_prefix);

    debug!("Effective {:?}", opt);

    let password = if let Some(p) = &opt.password {
        if p == "-" {
            Some(rpassword::prompt_password("Password: ").unwrap())
        } else {
            Some(p.clone())
        }
    } else {
        None
    };

    let mut clusters = HashMap::new();
    let config = load_config(&opt, &password, &mut clusters);

    debug!("Config {:?}", config);
    let state = make_state(&opt, config, password, clusters);

    if !opt.no_motd && opt.script.is_none() && opt.command.is_none() {
        fetch_and_print_motd();
    }

    let signals = Signals::new(Arc::new(AtomicBool::new(false)));
    let handler_signals = signals.clone();
    let context_signals = signals.clone();

    ctrlc::set_handler(move || {
        handler_signals.trigger();
    })
    .expect("Error setting Ctrl-C handler");

    context.set_signals(context_signals);

    merge_couchbase_delta(&mut context, state);

    let input = if opt.stdin {
        let stdin = std::io::stdin();
        let buf_reader = BufReader::new(stdin);
        PipelineData::ByteStream(
            ByteStream::new(
                ByteStreamSource::Read(Box::new(buf_reader)),
                Span::new(0, 0),
                signals,
                ByteStreamType::String,
            ),
            None,
        )

        // PipelineData::ByteStream {
        //     ByteStream
        //     stdout: Some(RawStream::new(
        //         Box::new(BufferedReader::new(buf_reader)),
        //         Some(ctrlc),
        //         Span::new(0, 0),
        //         None,
        //     )),
        //     stderr: None,
        //     exit_code: None,
        //     span: Span::new(0, 0),
        //     metadata: None,
        //     trim_end_newline: false,
        //     1: None,
        // }
    } else {
        // PipelineData::new_with_metadata(None, Span::new(0, 0))
        Value::nothing(Span::new(0, 0)).into_pipeline_data()
    };

    // This is throwing errors at me, looks like it's something in nu stdlib itself.
    // load_standard_library(&mut context).unwrap();
    if let Some(plugins) = opt.plugins {
        let mut working_set = StateWorkingSet::new(&context);
        for plugin_filename in plugins {
            // Make sure the plugin filenames are canonicalized
            let filename = canonicalize_with(&plugin_filename, &init_cwd)?;

            let identity = PluginIdentity::new(&filename, None)
                .unwrap_or_else(|_| panic!("Invalid plugin name: {}", plugin_filename));

            // Create the plugin and add it to the working set
            let plugin = nu_plugin_engine::add_plugin_to_working_set(&mut working_set, &identity)?;

            // Spawn the plugin to get the metadata and signatures
            let interface = plugin.clone().get_plugin(None)?;

            // Set its metadata
            plugin.set_metadata(Some(interface.get_metadata()?));

            // Add the commands from the signature to the working set
            for signature in interface.get_signature()? {
                let decl = PluginDeclaration::new(plugin.clone(), signature);
                working_set.add_decl(Box::new(decl));
            }
        }
        context.merge_delta(working_set.render())?;
    }

    if let Some(c) = opt.command {
        context.generate_nu_constant();
        read_plugin_file(&mut context, None);
        let opts = EvaluateCommandsOpts {
            table_mode: None,
            error_style: None,
            no_newline: false,
        };
        nu_cli::evaluate_commands(&c, &mut context, &mut stack, input, opts)
            .expect("Failed to run command");
        return Ok(());
    }

    if let Some(filepath) = opt.script {
        context.generate_nu_constant();
        read_plugin_file(&mut context, None);
        nu_cli::evaluate_file(filepath, &args_to_script, &mut context, &mut stack, input)
            .expect("Failed to run script");

        return Ok(());
    }

    context.is_interactive = true;

    // This is required else we encounter a bug where a shell hangs on a missing command
    #[cfg(unix)]
    terminal::acquire();
    read_plugin_file(&mut context, None);
    read_nu_config_file(&mut context, &mut stack);

    use_query_autocomplete(&mut context, &mut stack);

    nu_cli::evaluate_repl(&mut context, stack, None, None, entire_start_time)
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

    let response = match reqwest::blocking::Client::new()
        .get("http://motd.couchbase.sh/motd")
        .timeout(Duration::from_millis(500))
        .header("User-Agent", agent)
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

fn validate_hostnames(hostnames: Vec<String>) -> (RemoteClusterType, Vec<String>) {
    let mut validated = vec![];
    for hostname in &hostnames {
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
        } else if hostname.strip_suffix(":11210").is_some()
            || hostname.strip_suffix(":11211").is_some()
        {
            error!("Memcached port detected, http scheme must be used with custom port (management port)");
            std::process::exit(1);
        } else if let Some(stripped_http) = hostname.strip_prefix("http://") {
            stripped_http.to_string()
        } else if let Some(stripped_http) = hostname.strip_prefix("https://") {
            stripped_http.to_string()
        } else {
            hostname.to_string()
        };

        validated.push(host);
    }

    (RemoteClusterType::from(hostnames), validated)
}

fn create_logger_builder(logger_prefix: Option<String>) {
    let mut logger_builder = env_logger::Builder::from_env(
        Env::default().filter_or("CBSH_LOG", "info,isahc=error,surf=error,nu=warn"),
    );

    logger_builder.format(move |buf, record| {
        let mut style = buf.style();
        style.set_intense(true);
        style.set_bold(true);
        if let Some(l) = logger_prefix.clone() {
            return writeln!(
                buf,
                "{} [{}] {} {}",
                style.value(l),
                buf.default_styled_level(record.level()),
                style.value(Local::now().format("%Y-%m-%d %H:%M:%S%.3f")),
                style.value(record.args())
            );
        }
        writeln!(
            buf,
            "[{}] {} {}",
            buf.default_styled_level(record.level()),
            style.value(Local::now().format("%Y-%m-%d %H:%M:%S%.3f")),
            style.value(record.args())
        )
    });

    logger_builder.init();
}

fn remote_cluster_from_opts(opt: CliOptions, password: Option<String>) -> RemoteCluster {
    const DEFAULT_PASSWORD: &str = "password";
    const DEFAULT_HOSTNAME: &str = "localhost";
    const DEFAULT_USERNAME: &str = "Administrator";

    let conn_string = if let Some(hosts) = opt.conn_string {
        hosts
    } else {
        DEFAULT_HOSTNAME.to_string()
    };

    let username = if let Some(user) = opt.username {
        user
    } else {
        DEFAULT_USERNAME.to_string()
    };

    let rpassword = if let Some(pass) = password {
        pass
    } else {
        DEFAULT_PASSWORD.to_string()
    };

    let tls_config = if opt.disable_tls {
        warn!(
                "Using PLAIN authentication for cluster default, credentials will sent in plaintext - configure tls to disable this warning"
            );
        None
    } else {
        Some(RustTlsConfig::new(opt.tls_accept_all_certs, opt.tls_cert_path).unwrap())
    };
    let (cluster_type, hostnames) =
        validate_hostnames(conn_string.split(',').map(|v| v.to_owned()).collect());
    RemoteCluster::new(
        RemoteClusterResources {
            hostnames,
            username,
            password: rpassword,
            active_bucket: opt.bucket,
            active_scope: opt.scope,
            active_collection: opt.collection,
            display_name: opt.display_name,
        },
        tls_config,
        ClusterTimeouts::default(),
        None,
        None,
        DEFAULT_KV_BATCH_SIZE,
        cluster_type,
    )
}

fn maybe_write_config_file(opt: CliOptions, password: Option<String>) -> PathBuf {
    let identifier = if let Some(c) = opt.cluster {
        println!("Using {} as cluster identifier", c);
        c
    } else {
        println!("Please enter an identifier for the default cluster:");
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .expect("Failed to read user input");
        answer.trim().to_string()
    };
    let conn_string = if let Some(c) = opt.conn_string {
        println!("Using {} as connection string", c);
        c
    } else {
        println!("Please enter connection string (.e.g. couchbases://cb.<cluster-identifier>.sdk.cloud.couchbase.com)");
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .expect("Failed to read user input");
        answer.trim().to_string()
    };
    validate_hostnames(
        conn_string
            .clone()
            .split(',')
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
    );
    let username = if let Some(user) = opt.username {
        println!("Using {} as username", &user);
        Some(user)
    } else {
        println!("Please enter username:");
        read_input()
    };

    let password = if let Some(pass) = password {
        println!("Using password entered as password");
        Some(pass)
    } else {
        println!("Please enter password:");
        Some(rpassword::read_password().expect("Failed to read password"))
    };

    let bucket = if let Some(bucket) = opt.bucket {
        println!("Using {} as default bucket", &bucket);
        Some(bucket)
    } else {
        println!("Please enter default bucket:");
        read_input()
    };
    let scope = opt.scope;
    let collection = opt.collection;
    println!("Please enter directory for config file (~/.cbsh/):");
    let mut path_answer = String::new();
    std::io::stdin()
        .read_line(&mut path_answer)
        .expect("Failed to read user input");

    let path = match path_answer.to_lowercase().trim() {
        "" => {
            let mut buf = dirs::home_dir().unwrap();
            buf.push(".cbsh");
            buf
        }
        _ => {
            let path = path_answer.trim().to_string();
            let path = shellexpand::full(path.as_str()).expect("Failed to read path");
            PathBuf::from(path.to_string())
        }
    };

    let config_builder = ClusterConfigBuilder::new(
        identifier,
        conn_string,
        ClusterCredentials::new(username, password),
    )
    .default_bucket(bucket)
    .default_scope(scope)
    .default_collection(collection)
    .tls_config(ClusterTlsConfig::new(!opt.disable_tls, None, false));

    let config = ShellConfig::new_from_clusters(vec![config_builder.build()], vec![]);
    let mut to_write_to = path.clone();
    if !to_write_to.exists() {
        std::fs::create_dir_all(&to_write_to).expect("Failed to create config directory");
    }

    to_write_to.push("config");
    let mut output = File::create(&to_write_to).expect("Failed to create config file");

    println!("Writing config to {:?}", &to_write_to);

    write!(
        output,
        "{}",
        config.to_str().expect("Failed to convert config to string")
    )
    .expect("Failed to write config file");

    path
}

fn read_input() -> Option<String> {
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .expect("Failed to read user input");

    answer = answer.trim().to_string();
    if answer.is_empty() {
        None
    } else {
        Some(answer)
    }
}

fn load_config(
    opt: &CliOptions,
    password: &Option<String>,
    clusters: &mut HashMap<String, RemoteCluster>,
) -> Option<ShellConfig> {
    let config_path = opt.clone().config_path.map(PathBuf::from);
    match ShellConfig::new(config_path) {
        Some(c) => Some(c),
        None => {
            if opt.command.is_some() || opt.script.is_some() || opt.clone().no_config_prompt {
                let cluster = remote_cluster_from_opts(opt.clone(), password.clone());
                clusters.insert("default".to_string(), cluster);
                None
            } else {
                println!("No config file found");
                println!("Would you like to create one now (Y/n)?");

                let mut answer = String::new();
                std::io::stdin()
                    .read_line(&mut answer)
                    .expect("Failed to read user input");

                match answer.to_lowercase().trim() {
                    "y" | "" => {
                        let path = maybe_write_config_file(opt.clone(), password.clone());
                        ShellConfig::new(Some(path))
                    }
                    _ => {
                        let cluster = remote_cluster_from_opts(opt.clone(), password.clone());
                        clusters.insert("default".to_string(), cluster);
                        None
                    }
                }
            }
        }
    }
}

fn make_state(
    opt: &CliOptions,
    config: Option<ShellConfig>,
    password: Option<String>,
    mut clusters: HashMap<String, RemoteCluster>,
) -> Arc<Mutex<State>> {
    let mut capella_orgs = HashMap::new();
    let mut active_capella_org = None;
    let mut active_project = None;
    let mut llms = HashMap::new();
    let mut active_llm = None;
    let (active, config_location) = if let Some(c) = config {
        let mut active = None;
        for v in c.clusters() {
            let name = v.identifier().to_string();

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
            let transaction_timeout = match timeouts.transaction_timeout() {
                Some(t) => t.to_owned(),
                None => DEFAULT_TRANSACTION_TIMEOUT,
            };
            let kv_batch_size = match v.kv_batch_size() {
                Some(b) => b,
                None => DEFAULT_KV_BATCH_SIZE,
            };

            let (cluster_type, hostnames) = validate_hostnames(
                v.conn_string()
                    .split(',')
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            );
            let cluster_tls_config = v.tls().clone();
            let tls_config = if cluster_tls_config.enabled() {
                Some(RustTlsConfig::try_from(cluster_tls_config).unwrap())
            } else {
                None
            };
            let cluster = RemoteCluster::new(
                RemoteClusterResources {
                    hostnames,
                    username,
                    password: cpassword,
                    active_bucket: default_bucket,
                    active_scope: scope,
                    active_collection: collection,
                    display_name: v.display_name(),
                },
                tls_config,
                ClusterTimeouts::new(
                    data_timeout,
                    query_timeout,
                    analytics_timeout,
                    search_timeout,
                    management_timeout,
                    transaction_timeout,
                ),
                v.cloud_org(),
                v.project(),
                kv_batch_size,
                v.cluster_type().unwrap_or(cluster_type),
            );
            if !v.tls().clone().enabled() {
                warn!(
                    "Using PLAIN authentication for cluster {}, credentials will sent in plaintext - configure tls to disable this warning",
                    name.clone()
                );
            }
            clusters.insert(name.clone(), cluster);
        }
        for c in c.capella_orgs() {
            let management_timeout = match c.management_timeout() {
                Some(t) => t.to_owned(),
                None => DEFAULT_MANAGEMENT_TIMEOUT,
            };
            let name = c.identifier();
            let api_endpoint = c.api_endpoint().unwrap_or(CLOUD_URL.to_string());

            let plane = RemoteCapellaOrganization::new(
                c.secret_key(),
                c.access_key(),
                management_timeout,
                c.default_project(),
                api_endpoint,
            );

            if active_capella_org.is_none() {
                active_capella_org = Some(name.clone());
                active_project = c.default_project()
            }

            capella_orgs.insert(name, plane);
        }

        for config in c.llms() {
            let llm = Llm::new(
                config.api_key(),
                config.provider(),
                config.embed_model(),
                config.chat_model(),
            );
            llms.insert(config.identifier(), llm);

            if active_llm.is_none() {
                active_llm = Some(config.identifier())
            }
        }

        (active.unwrap_or_default(), c.location().clone())
    } else {
        (String::from("default"), None)
    };

    Arc::new(Mutex::new(State::new(
        clusters,
        active,
        config_location,
        capella_orgs,
        active_capella_org,
        active_project,
        llms,
        active_llm,
    )))
}

fn merge_couchbase_delta(context: &mut EngineState, state: Arc<Mutex<State>>) {
    let delta = {
        let mut working_set = StateWorkingSet::new(context);
        working_set.add_decl(Box::new(AllowIP::new(state.clone())));
        working_set.add_decl(Box::new(Analytics::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsBuckets::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsDatasets::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsDataverses::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsIndexes::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsLinks::new(state.clone())));
        working_set.add_decl(Box::new(AnalyticsPendingMutations::new(state.clone())));
        working_set.add_decl(Box::new(Ask::new(state.clone())));
        working_set.add_decl(Box::new(Buckets::new(state.clone())));
        working_set.add_decl(Box::new(BucketsConfig::new(state.clone())));
        working_set.add_decl(Box::new(BucketsCreate::new(state.clone())));
        working_set.add_decl(Box::new(BucketsDrop::new(state.clone())));
        working_set.add_decl(Box::new(BucketsFlush::new(state.clone())));
        working_set.add_decl(Box::new(BucketsGet::new(state.clone())));
        working_set.add_decl(Box::new(BucketsSample::new(state.clone())));
        working_set.add_decl(Box::new(BucketsUpdate::new(state.clone())));
        working_set.add_decl(Box::new(CbEnvCluster::new(state.clone())));
        working_set.add_decl(Box::new(CbEnvLLM::new(state.clone())));
        working_set.add_decl(Box::new(CBEnvManaged::new(state.clone())));
        working_set.add_decl(Box::new(CbEnvRegister::new(state.clone())));
        working_set.add_decl(Box::new(CbEnvUnregister::new(state.clone())));
        working_set.add_decl(Box::new(Clusters::new(state.clone())));
        working_set.add_decl(Box::new(ClustersCreate::new(state.clone())));
        working_set.add_decl(Box::new(ClustersDrop::new(state.clone())));
        working_set.add_decl(Box::new(ClustersGet::new(state.clone())));
        working_set.add_decl(Box::new(Collections::new(state.clone())));
        working_set.add_decl(Box::new(CollectionsCreate::new(state.clone())));
        working_set.add_decl(Box::new(CollectionsDrop::new(state.clone())));
        working_set.add_decl(Box::new(Columnar));
        working_set.add_decl(Box::new(ColumnarClusters::new(state.clone())));
        working_set.add_decl(Box::new(ColumnarClustersCreate::new(state.clone())));
        working_set.add_decl(Box::new(ColumnarClustersDrop::new(state.clone())));
        working_set.add_decl(Box::new(ColumnarDatabases::new(state.clone())));
        working_set.add_decl(Box::new(ColumnarQuery::new(state.clone())));
        working_set.add_decl(Box::new(Credentials::new(state.clone())));
        working_set.add_decl(Box::new(CredentialsCreate::new(state.clone())));
        working_set.add_decl(Box::new(CredentialsDrop::new(state.clone())));
        working_set.add_decl(Box::new(Doc));
        working_set.add_decl(Box::new(DocGet::new(state.clone())));
        working_set.add_decl(Box::new(DocImport::new(state.clone())));
        working_set.add_decl(Box::new(DocInsert::new(state.clone())));
        working_set.add_decl(Box::new(DocReplace::new(state.clone())));
        working_set.add_decl(Box::new(DocRemove::new(state.clone())));
        working_set.add_decl(Box::new(DocUpsert::new(state.clone())));
        working_set.add_decl(Box::new(HealthCheck::new(state.clone())));
        working_set.add_decl(Box::new(Help));
        working_set.add_decl(Box::new(FakeData::new(state.clone())));
        working_set.add_decl(Box::new(Nodes::new(state.clone())));
        working_set.add_decl(Box::new(Organizations::new(state.clone())));
        working_set.add_decl(Box::new(Ping::new(state.clone())));
        working_set.add_decl(Box::new(Projects::new(state.clone())));
        working_set.add_decl(Box::new(ProjectsCreate::new(state.clone())));
        working_set.add_decl(Box::new(ProjectsDrop::new(state.clone())));
        working_set.add_decl(Box::new(Query::new(state.clone())));
        working_set.add_decl(Box::new(QueryAdvise::new(state.clone())));
        working_set.add_decl(Box::new(QueryIndexes::new(state.clone())));
        working_set.add_decl(Box::new(QueryTransactions::new(state.clone())));
        working_set.add_decl(Box::new(Scopes::new(state.clone())));
        working_set.add_decl(Box::new(ScopesCreate::new(state.clone())));
        working_set.add_decl(Box::new(ScopesDrop::new(state.clone())));
        working_set.add_decl(Box::new(Search::new(state.clone())));
        working_set.add_decl(Box::new(SubDocGet::new(state.clone())));
        working_set.add_decl(Box::new(Transactions));
        working_set.add_decl(Box::new(TransactionsListAtrs::new(state.clone())));
        working_set.add_decl(Box::new(Tutorial::new(state.clone())));
        working_set.add_decl(Box::new(TutorialNext::new(state.clone())));
        working_set.add_decl(Box::new(TutorialPage::new(state.clone())));
        working_set.add_decl(Box::new(TutorialPrev::new(state.clone())));
        working_set.add_decl(Box::new(UseBucket::new(state.clone())));
        working_set.add_decl(Box::new(UseCapellaOrganization::new(state.clone())));
        working_set.add_decl(Box::new(UseCmd::new(state.clone())));
        working_set.add_decl(Box::new(UseCollection::new(state.clone())));
        working_set.add_decl(Box::new(UseProject::new(state.clone())));
        working_set.add_decl(Box::new(UseScope::new(state.clone())));
        working_set.add_decl(Box::new(UseTimeouts::new(state.clone())));
        working_set.add_decl(Box::new(Users::new(state.clone())));
        working_set.add_decl(Box::new(UsersGet::new(state.clone())));
        working_set.add_decl(Box::new(UsersDrop::new(state.clone())));
        working_set.add_decl(Box::new(UsersRoles::new(state.clone())));
        working_set.add_decl(Box::new(UsersUpsert::new(state.clone())));
        working_set.add_decl(Box::new(Vector));
        working_set.add_decl(Box::new(VectorEnrichDoc::new(state.clone())));
        working_set.add_decl(Box::new(VectorEnrichText::new(state.clone())));
        working_set.add_decl(Box::new(Version));
        working_set.add_decl(Box::new(VectorCreateIndex::new(state.clone())));
        working_set.add_decl(Box::new(VectorSearch::new(state.clone())));

        working_set.add_decl(Box::new(nu_cli::NuHighlight));
        working_set.add_decl(Box::new(nu_cli::Print));

        working_set.render()
    };

    if let Err(err) = context.merge_delta(delta) {
        report_shell_error(context, &err);
    }
}

/// Get the directory where the Nushell executable is located.
fn current_exe_directory() -> PathBuf {
    let mut path = std::env::current_exe().expect("current_exe() should succeed");
    path.pop();
    path
}

/// Get the current working directory from the environment.
fn current_dir_from_environment() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        return cwd;
    }
    if let Ok(cwd) = std::env::var("PWD") {
        return cwd.into();
    }
    if let Some(home) = nu_path::home_dir() {
        return home.into_std_path_buf();
    }
    current_exe_directory()
}

fn use_query_autocomplete(context: &mut EngineState, stack: &mut Stack) {
    let path_res = current_dir_from_environment();
    eval_source(
        context,
        stack,
        format!("use {}/query_autocomplete FROM", path_res.display()).as_bytes(),
        "entry 0",
        PipelineData::empty(),
        false,
    );

    if let Err(err) = context.merge_env(stack) {
        report_shell_error(context, &err);
    }
}
