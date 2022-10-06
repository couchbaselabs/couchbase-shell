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
mod tutorial;

use crate::cli::*;
use crate::cli_options::{parse_commandline_args, parse_shell_args, CliOptions};
use crate::config::{
    ClusterConfigBuilder, ClusterCredentials, ShellConfig, DEFAULT_ANALYTICS_TIMEOUT,
    DEFAULT_DATA_TIMEOUT, DEFAULT_KV_BATCH_SIZE, DEFAULT_MANAGEMENT_TIMEOUT, DEFAULT_QUERY_TIMEOUT,
    DEFAULT_SEARCH_TIMEOUT,
};
use crate::config_files::{read_nu_config_file, CBSHELL_FOLDER};
use crate::default_context::create_default_context;
use crate::remote_cluster::{
    ClusterTimeouts, RemoteCluster, RemoteClusterResources, RemoteClusterType,
};
use crate::state::RemoteCapellaOrganization;
use chrono::Local;
use config::ClusterTlsConfig;
use env_logger::Env;
use log::{debug, warn};
use log::{error, info};
use nu_cli::{add_plugin_file, gather_parent_env_vars, read_plugin_file};

use nu_protocol::engine::{Stack, StateWorkingSet};
use nu_protocol::{BufferedReader, PipelineData, RawStream, Span};
use serde::Deserialize;
use state::State;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    let entire_start_time = std::time::Instant::now();
    let mut logger_builder = create_logger_builder();

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
    let mut context = create_default_context();

    gather_parent_env_vars(&mut context, &init_cwd);
    let mut stack = Stack::new();

    let (shell_commandline_args, args_to_script) = parse_shell_args();

    let opt = match parse_commandline_args(&shell_commandline_args, &mut context) {
        Ok(o) => o,
        Err(_) => std::process::exit(1),
    };

    let opt_clone = opt.clone();
    logger_builder.format(move |buf, record| {
        let mut style = buf.style();
        style.set_intense(true);
        style.set_bold(true);
        if let Some(l) = &opt_clone.logger_prefix {
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

    debug!("Effective {:?}", opt);

    let password = match opt.password {
        true => Some(rpassword::prompt_password("Password: ").unwrap()),
        false => None,
    };

    let mut clusters = HashMap::new();
    let config_path = if let Some(p) = opt.clone().config_path {
        Some(PathBuf::from(p))
    } else {
        None
    };
    let config = match ShellConfig::new(config_path) {
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
    };

    debug!("Config {:?}", config);

    let mut capella_orgs = HashMap::new();
    let mut active_capella_org = None;
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
            let kv_batch_size = match v.kv_batch_size() {
                Some(b) => b,
                None => DEFAULT_KV_BATCH_SIZE,
            };

            let (cluster_type, hostnames) = validate_hostnames(
                v.conn_string()
                    .split(",")
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            );
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
                cluster_type,
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
            let default_project = c.default_project();

            let plane = RemoteCapellaOrganization::new(
                c.secret_key(),
                c.access_key(),
                management_timeout,
                default_project,
            );

            if active_capella_org.is_none() {
                active_capella_org = Some(name.clone());
            }

            capella_orgs.insert(name, plane);
        }

        (active.unwrap_or_default(), c.location().clone())
    } else {
        (String::from("default"), None)
    };

    let state = Arc::new(Mutex::new(State::new(
        clusters,
        active,
        config_location,
        capella_orgs,
        active_capella_org,
    )));

    if !opt.no_motd && opt.script.is_none() && opt.command.is_none() {
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
        let mut working_set = StateWorkingSet::new(&context);
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
        working_set.add_decl(Box::new(Databases::new(state.clone())));
        working_set.add_decl(Box::new(DatabasesCreate::new(state.clone())));
        working_set.add_decl(Box::new(DatabasesDrop::new(state.clone())));
        working_set.add_decl(Box::new(DatabasesGet::new(state.clone())));
        working_set.add_decl(Box::new(HealthCheck::new(state.clone())));
        working_set.add_decl(Box::new(CBEnvManaged::new(state.clone())));
        working_set.add_decl(Box::new(CbEnvRegister::new(state.clone())));
        working_set.add_decl(Box::new(CbEnvUnregister::new(state.clone())));
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
        working_set.add_decl(Box::new(CbEnvDatabase::new(state.clone())));
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
        working_set.add_decl(Box::new(Version));

        working_set.render()
    };
    let _ = context.merge_delta(delta);

    let input = if opt.stdin {
        let stdin = std::io::stdin();
        let buf_reader = BufReader::new(stdin);

        PipelineData::ExternalStream {
            stdout: Some(RawStream::new(
                Box::new(BufferedReader::new(buf_reader)),
                Some(ctrlc),
                Span::new(0, 0),
                None,
            )),
            stderr: None,
            exit_code: None,
            span: Span::new(0, 0),
            metadata: None,
            trim_end_newline: false,
        }
    } else {
        PipelineData::new_with_metadata(None, Span::new(0, 0))
    };

    if let Some(c) = opt.command {
        add_plugin_file(&mut context, None, CBSHELL_FOLDER);
        nu_cli::evaluate_commands(&c, &mut context, &mut stack, input, None)
            .expect("Failed to run command");
        return Ok(());
    }

    if let Some(filepath) = opt.script {
        add_plugin_file(&mut context, None, CBSHELL_FOLDER);
        nu_cli::evaluate_file(filepath, &args_to_script, &mut context, &mut stack, input)
            .expect("Failed to run script");

        return Ok(());
    }

    read_plugin_file(&mut context, &mut stack, None, CBSHELL_FOLDER);
    read_nu_config_file(&mut context, &mut stack);

    nu_cli::evaluate_repl(
        &mut context,
        &mut stack,
        "CouchbaseShell",
        None,
        entire_start_time,
    )
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
            hostname.to_string()
        };

        validated.push(host);
    }

    (RemoteClusterType::from(hostnames), validated)
}

fn create_logger_builder() -> env_logger::Builder {
    let logger_builder = env_logger::Builder::from_env(
        Env::default().filter_or("CBSH_LOG", "info,isahc=error,surf=error,nu=warn"),
    );

    logger_builder
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

    let tls_config = ClusterTlsConfig::new(
        !opt.disable_tls,
        opt.tls_cert_path.clone(),
        opt.tls_cert_path.is_none(),
    );
    if !tls_config.enabled() {
        warn!(
                "Using PLAIN authentication for cluster default, credentials will sent in plaintext - configure tls to disable this warning"
            );
    }
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
        DEFAULT_KV_BATCH_SIZE,
        cluster_type,
    )
}

fn maybe_write_config_file(opt: CliOptions, password: Option<String>) -> PathBuf {
    let identifier = if let Some(c) = opt.cluster {
        println!("Using {} as database identifier", c);
        c
    } else {
        println!("Please enter an identifier for the default database:");
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
        println!("Please enter connection string:");
        let mut answer = String::new();
        std::io::stdin()
            .read_line(&mut answer)
            .expect("Failed to read user input");
        answer.trim().to_string()
    };
    validate_hostnames(
        conn_string
            .clone()
            .split(",")
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
    println!("Write password to config file (Y/n)?");
    let mut write_password = String::new();
    std::io::stdin()
        .read_line(&mut write_password)
        .expect("Failed to read user input");
    let password = match write_password.to_lowercase().trim() {
        "y" | "" => {
            if let Some(pass) = password {
                println!("Using password entered as password");
                Some(pass)
            } else {
                println!("Please enter password:");
                read_input()
            }
        }
        _ => None,
    };

    let bucket = if let Some(bucket) = opt.bucket {
        println!("Using {} as default bucket", &bucket);
        Some(bucket)
    } else {
        println!("Please enter default bucket:");
        read_input()
    };
    let scope = if let Some(scope) = opt.scope {
        println!("Using {} as scope", &scope);
        Some(scope)
    } else {
        println!("Please enter default scope:");
        read_input()
    };
    let collection = if let Some(collection) = opt.collection {
        println!("Using {} as collection", &collection);
        Some(collection)
    } else {
        println!("Please enter default collection:");
        read_input()
    };
    println!("Please enter directory for config file (.cbsh/):");
    let mut path_answer = String::new();
    std::io::stdin()
        .read_line(&mut path_answer)
        .expect("Failed to read user input");

    let path = match path_answer.to_lowercase().trim() {
        "" => {
            let mut buf = std::env::current_dir().unwrap();
            buf.push(".cbsh");
            buf
        }
        _ => PathBuf::from(path_answer.trim().to_string()),
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
    to_write_to.push("config");
    println!("Writing config to {:?}", &to_write_to);
    fs::write(
        to_write_to,
        config.to_str().expect("Failed to convert config to string"),
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
