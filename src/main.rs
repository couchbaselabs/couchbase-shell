#![recursion_limit = "256"]

mod cli;
mod client;
mod config;
mod state;
mod tutorial;

use crate::config::ShellConfig;
use crate::state::{RemoteCloud, RemoteCluster};
use crate::{cli::*, state::ClusterTimeouts};
use config::ClusterTlsConfig;
use log::{debug, warn};
use nu_cli::{NuScript, Options};
use serde::Deserialize;
use state::State;
use std::collections::HashMap;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use structopt::StructOpt;
use temp_dir::TempDir;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    const DEFAULT_PASSWORD: &str = "password";
    const DEFAULT_HOSTNAME: &str = "localhost";
    const DEFAULT_USERNAME: &str = "Administrator";

    let opt = CliOptions::from_args();
    debug!("Effective {:?}", opt);

    let config = ShellConfig::new();
    warn!("Config {:?}", config);

    let mut clusters = HashMap::new();
    let mut clouds = HashMap::new();

    let password = match opt.password {
        true => Some(rpassword::read_password_from_tty(Some("Password: ")).unwrap()),
        false => None,
    };

    let mut default_scope: Option<String> = None;
    let mut default_collection: Option<String> = None;
    let active = if config.clusters().is_empty() {
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

        default_scope = opt.scope.clone();
        default_collection = opt.collection.clone();

        let tls_config = ClusterTlsConfig::new(
            !opt.disable_tls,
            opt.tls_cert_path.clone(),
            !opt.dont_validate_hostnames,
            opt.tls_cert_path.is_none(),
        );
        if !tls_config.enabled() {
            println!(
                "Using PLAIN authentication for cluster default, credentials will sent in plaintext - configure tls to disable this warning"
            );
        }
        let cluster = RemoteCluster::new(
            hostnames.split(',').map(|v| v.to_owned()).collect(),
            username,
            rpassword,
            opt.bucket,
            opt.scope,
            opt.collection,
            tls_config,
            ClusterTimeouts::default(),
            None,
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

            if default_scope.is_none() && scope.is_some() {
                default_scope = scope.clone();
            }
            if default_collection.is_none() && collection.is_some() {
                default_collection = collection.clone();
            }

            let timeouts = v.timeouts();
            let data_timeout = match timeouts.data_timeout() {
                Some(t) => t.to_owned(),
                None => Duration::from_millis(30000),
            };
            let query_timeout = match timeouts.query_timeout() {
                Some(t) => t.to_owned(),
                None => Duration::from_millis(75000),
            };

            let cluster = RemoteCluster::new(
                v.hostnames().clone(),
                username,
                cpassword,
                default_bucket,
                scope,
                collection,
                v.tls().clone(),
                ClusterTimeouts::new(data_timeout, query_timeout),
                v.cloud_control_pane(),
            );
            if !v.tls().clone().enabled() {
                println!(
                    "Using PLAIN authentication for cluster {}, credentials will sent in plaintext - configure tls to disable this warning",
                    name.clone()
                );
            }
            clusters.insert(name.clone(), cluster);
        }
        for c in config.clouds() {
            let name = c.identifier();
            let cloud = RemoteCloud::new(name.clone(), c.secret_key(), c.access_key());

            clouds.insert(name, cloud);
        }
        active.unwrap()
    };

    let state = Arc::new(Mutex::new(State::new(
        clusters,
        active,
        default_scope,
        default_collection,
        config.location().clone(),
        clouds,
    )));

    //if !opt.no_motd && opt.script.is_none() && opt.command.is_none() {
    //    fetch_and_print_motd().await;
    //}

    let context = nu_cli::create_default_context(true)?;
    context.add_commands(vec![
        nu_engine::whole_stream_command(Analytics::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsDatasets::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsDataverses::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsIndexes::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsLinks::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsBuckets::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsPendingMutations::new(state.clone())),
        nu_engine::whole_stream_command(Buckets {}),
        nu_engine::whole_stream_command(BucketsConfig::new(state.clone())),
        nu_engine::whole_stream_command(BucketsCreate::new(state.clone())),
        nu_engine::whole_stream_command(BucketsDrop::new(state.clone())),
        nu_engine::whole_stream_command(BucketsFlush::new(state.clone())),
        nu_engine::whole_stream_command(BucketsGet::new(state.clone())),
        nu_engine::whole_stream_command(BucketsSample::new(state.clone())),
        nu_engine::whole_stream_command(BucketsUpdate::new(state.clone())),
        nu_engine::whole_stream_command(Clusters::new(state.clone())),
        nu_engine::whole_stream_command(ClustersHealth::new(state.clone())),
        nu_engine::whole_stream_command(ClustersRegister::new(state.clone())),
        nu_engine::whole_stream_command(ClustersUnregister::new(state.clone())),
        nu_engine::whole_stream_command(Collections {}),
        nu_engine::whole_stream_command(CollectionsCreate::new(state.clone())),
        nu_engine::whole_stream_command(CollectionsGet::new(state.clone())),
        nu_engine::whole_stream_command(Doc {}),
        nu_engine::whole_stream_command(DocGet::new(state.clone())),
        nu_engine::whole_stream_command(DocInsert::new(state.clone())),
        nu_engine::whole_stream_command(DocRemove::new(state.clone())),
        nu_engine::whole_stream_command(DocReplace::new(state.clone())),
        nu_engine::whole_stream_command(DocUpsert::new(state.clone())),
        nu_engine::whole_stream_command(FakeData::new(state.clone())),
        nu_engine::whole_stream_command(Help {}),
        nu_engine::whole_stream_command(Nodes::new(state.clone())),
        nu_engine::whole_stream_command(Ping::new(state.clone())),
        nu_engine::whole_stream_command(Query::new(state.clone())),
        nu_engine::whole_stream_command(QueryAdvise::new(state.clone())),
        nu_engine::whole_stream_command(QueryIndexes::new(state.clone())),
        nu_engine::whole_stream_command(Scopes {}),
        nu_engine::whole_stream_command(ScopesCreate::new(state.clone())),
        nu_engine::whole_stream_command(ScopesGet::new(state.clone())),
        nu_engine::whole_stream_command(Search::new(state.clone())),
        nu_engine::whole_stream_command(Tutorial::new(state.clone())),
        nu_engine::whole_stream_command(TutorialNext::new(state.clone())),
        nu_engine::whole_stream_command(TutorialPage::new(state.clone())),
        nu_engine::whole_stream_command(TutorialPrev::new(state.clone())),
        nu_engine::whole_stream_command(Users::new(state.clone())),
        nu_engine::whole_stream_command(UsersGet::new(state.clone())),
        nu_engine::whole_stream_command(UsersRoles::new(state.clone())),
        nu_engine::whole_stream_command(UsersUpsert::new(state.clone())),
        nu_engine::whole_stream_command(UseBucket::new(state.clone())),
        nu_engine::whole_stream_command(UseCluster::new(state.clone())),
        nu_engine::whole_stream_command(UseCmd::new(state.clone())),
        nu_engine::whole_stream_command(UseCollection::new(state.clone())),
        nu_engine::whole_stream_command(UseScope::new(state.clone())),
        nu_engine::whole_stream_command(Whoami::new(state)),
        nu_engine::whole_stream_command(Version::new()),
        /*
        nu_engine::whole_stream_command(DataStats::new(state.clone())),
        nu_engine::whole_stream_command(Data {}),
        */
    ]);

    let mut options = Options::new();

    let d = TempDir::new().unwrap();
    let f = d.child("config.toml");

    let history_path: String = if let Some(p) = config.location() {
        let mut p = p.clone();
        p.pop();
        p.push("history.txt");
        format!("history-path = \"{}\"", p.to_str().unwrap())
    } else {
        "".into()
    };

    let prompt = if cfg!(windows) {
        r##"prompt = "build-string (ansi ub) (use | get username) (ansi reset) ' at ' (ansi yb) (use | get cluster) (ansi reset) ' in ' (ansi wb) (use | get bucket) (ansi reset) '\n' '> '""##
    } else {
        r##"prompt = "build-string '👤 ' (ansi ub) (use | get username) (ansi reset) ' 🏠 ' (ansi yb) (use | get cluster) (ansi reset) ' in 🗄 ' (ansi wb) (use | get bucket) (ansi reset) '\n' '> '""##
    };

    let config = format!("skip_welcome_message = true\n{}\n{}", history_path, prompt);

    std::fs::write(&f, config.as_bytes()).unwrap();

    options.config = Some(std::ffi::OsString::from(f));

    if let Some(c) = opt.command {
        options.scripts = vec![NuScript::code(std::iter::once(c.as_str()))?];
        nu_cli::run_script_file(context, options)?;
        return Ok(());
    }

    if let Some(filepath) = opt.script {
        let filepath = std::ffi::OsString::from(filepath);
        options.scripts = vec![NuScript::source_file(filepath.as_os_str())?];
        nu_cli::run_script_file(context, options)?;
        return Ok(());
    }

    nu_cli::cli(context, options)?;
    Ok(())
}

/// Fetches a helpful MOTD from couchbase.sh
///
/// Note that this can be disabled with the --no-motd cli flag if needed.
async fn _fetch_and_print_motd() {
    let agent = format!(
        "cbsh {} {}/{}",
        option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .user_agent(agent)
        .build();
    if client.is_err() {
        debug!(
            "Could not request MOTD because building the client failed: {}",
            client.err().unwrap()
        );
        return;
    }
    let client = client.unwrap();

    let resp = client.get("http://couchbase.sh/motd").send().await;

    if resp.is_err() {
        debug!(
            "Could not request MOTD because fetching the response failed: {}",
            resp.err().unwrap()
        );
        return;
    }
    let resp = resp.unwrap();

    let data = resp.json::<Motd>().await;
    if data.is_err() {
        debug!(
            "Could not request MOTD because converting the response data failed: {}",
            data.err().unwrap()
        );
        return;
    }
    let data = data.unwrap();
    println!("{}", data.msg);
}

#[derive(Debug, Deserialize)]
struct Motd {
    msg: String,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "The Couchbase Shell",
    about = "Alternative Shell and UI for Couchbase Server and Cloud"
)]
struct CliOptions {
    #[structopt(long = "hostnames")]
    hostnames: Option<String>,
    #[structopt(short = "u", long = "username")]
    username: Option<String>,
    #[structopt(short = "p", long = "password")]
    password: bool,
    #[structopt(long = "cluster")]
    cluster: Option<String>,
    #[structopt(long = "bucket")]
    bucket: Option<String>,
    #[structopt(long = "scope")]
    scope: Option<String>,
    #[structopt(long = "collection")]
    collection: Option<String>,
    #[structopt(long = "command", short = "c")]
    command: Option<String>,
    #[structopt(long = "script")]
    script: Option<String>,
    #[structopt(long = "stdin")]
    stdin: bool,
    #[structopt(long = "no-motd")]
    no_motd: bool,
    #[structopt(long = "disable-tls")]
    disable_tls: bool,
    #[structopt(long = "dont-validate-hostnames")]
    dont_validate_hostnames: bool,
    #[structopt(long = "tls-cert-path")]
    tls_cert_path: Option<String>,
}
