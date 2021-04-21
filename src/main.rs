#![recursion_limit = "256"]

mod cli;
mod client;
mod config;
mod state;
mod tutorial;

use crate::cli::*;
use crate::config::{ClusterTimeouts, ShellConfig};
use crate::state::RemoteCluster;
use config::ClusterTlsConfig;
use log::{debug, warn, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::Config;
use nu_cli::{NuScript, Options};
use serde::Deserialize;
use state::State;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::Duration;
use structopt::StructOpt;
use temp_dir::TempDir;

fn main() -> Result<(), Box<dyn Error>> {
    const DEFAULT_PASSWORD: &str = "password";
    const DEFAULT_HOSTNAME: &str = "localhost";
    const DEFAULT_USERNAME: &str = "Administrator";

    configure_logging();

    let opt = CliOptions::from_args();
    debug!("Effective {:?}", opt);

    let config = ShellConfig::new();
    warn!("Config {:?}", config);

    let mut clusters = HashMap::new();

    let password = match opt.password {
        true => Some(rpassword::read_password_from_tty(Some("Password: ")).unwrap()),
        false => None,
    };

    let mut default_scope: Option<String> = None;
    let mut default_collection: Option<String> = None;
    let active = if config.clusters().is_empty() {
        let timeouts = ClusterTimeouts::default().export_lcb_args();
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

        let cluster = RemoteCluster::new(
            hostnames.split(",").map(|v| v.to_owned()).collect(),
            username,
            rpassword,
            opt.bucket,
            opt.scope,
            opt.collection,
            ClusterTlsConfig::default(),
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

            let cluster = RemoteCluster::new(
                v.hostnames().clone(),
                username,
                cpassword,
                default_bucket,
                scope,
                collection,
                v.tls().clone(),
            );
            clusters.insert(name.clone(), cluster);
        }
        active.unwrap()
    };

    let state = Arc::new(State::new(
        clusters,
        active,
        default_scope,
        default_collection,
    ));

    //if !opt.no_motd && opt.script.is_none() && opt.command.is_none() {
    //    fetch_and_print_motd().await;
    //}

    let context = nu_cli::create_default_context(true)?;
    context.add_commands(vec![
        nu_engine::whole_stream_command(Analytics::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsDatasets::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsDataverses::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsIndexes::new(state.clone())),
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
        nu_engine::whole_stream_command(FakeData::new(state.clone())),
        nu_engine::whole_stream_command(Query::new(state.clone())),
        nu_engine::whole_stream_command(QueryAdvise::new(state.clone())),
        nu_engine::whole_stream_command(QueryIndexes::new(state.clone())),
        nu_engine::whole_stream_command(UseBucket::new(state.clone())),
        nu_engine::whole_stream_command(UseCluster::new(state.clone())),
        nu_engine::whole_stream_command(UseCmd::new(state.clone())),
        nu_engine::whole_stream_command(UseCollection::new(state.clone())),
        nu_engine::whole_stream_command(UseScope::new(state.clone())),
        nu_engine::whole_stream_command(Whoami::new(state.clone())),
        nu_engine::whole_stream_command(Version::new()),
        /*
                nu_engine::whole_stream_command(Analytics::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsIndexes::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsDatasets::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsDataverses::new(state.clone())),
        // Performs analytics queries
        // Performs kv get operations
        nu_engine::whole_stream_command(DocGet::new(state.clone())),
        // Performs kv upsert operations
        nu_engine::whole_stream_command(DocUpsert::new(state.clone())),
        // Performs kv insert operations
        nu_engine::whole_stream_command(DocInsert::new(state.clone())),
        // Performs kv replace operations
        nu_engine::whole_stream_command(DocReplace::new(state.clone())),
        nu_engine::whole_stream_command(DocRemove::new(state.clone())),
        nu_engine::whole_stream_command(DataStats::new(state.clone())),
        // Displays cluster manager node infos
        nu_engine::whole_stream_command(Nodes::new(state.clone())),
        // Displays cluster manager bucket infos
        nu_engine::whole_stream_command(BucketsCreate::new(state.clone())),
        nu_engine::whole_stream_command(BucketsUpdate::new(state.clone())),
        nu_engine::whole_stream_command(BucketsDrop::new(state.clone())),
        nu_engine::whole_stream_command(BucketsFlush::new(state.clone())),
        nu_engine::whole_stream_command(BucketsSample::new(state.clone())),
        // Performs n1ql queries
        // Manages local cluster references
        // Create fake data based on templates
        // Displays indexes
        // Allows to switch clusters, buckets and collections on the fly
        nu_engine::whole_stream_command(Doc {}),
        nu_engine::whole_stream_command(Data {}),
        nu_engine::whole_stream_command(Users::new(state.clone())),
        nu_engine::whole_stream_command(UsersGet::new(state.clone())),
        nu_engine::whole_stream_command(UsersUpsert::new(state.clone())),
        nu_engine::whole_stream_command(UsersRoles::new(state.clone())),
        nu_engine::whole_stream_command(Search::new(state.clone())),
        nu_engine::whole_stream_command(Ping::new(state.clone())),
        nu_engine::whole_stream_command(Collections {}),
        nu_engine::whole_stream_command(CollectionsGet::new(state.clone())),
        nu_engine::whole_stream_command(CollectionsCreate::new(state.clone())),
        nu_engine::whole_stream_command(Scopes {}),
        nu_engine::whole_stream_command(ScopesGet::new(state.clone())),
        nu_engine::whole_stream_command(ScopesCreate::new(state.clone())),
        nu_engine::whole_stream_command(SDKLog {}),
        nu_engine::whole_stream_command(Help {}),
        nu_engine::whole_stream_command(Tutorial::new(state.clone())),
        nu_engine::whole_stream_command(TutorialPage::new(state.clone())),
        nu_engine::whole_stream_command(TutorialPrev::new(state.clone())),
        nu_engine::whole_stream_command(TutorialNext::new(state.clone())),
        */
    ]);

    let mut options = Options::new();

    let d = TempDir::new().unwrap();
    let f = d.child("config.toml");

    let config = r##"
    skip_welcome_message = true
    prompt = "build-string '👤 ' $(ansi ub) $(use | get username) $(ansi reset) ' 🏠 ' $(ansi yb) $(use | get cluster) $(ansi reset) ' in 🗄 ' $(ansi wb) $(use | get bucket) $(ansi reset) '\n' '> '"
    "##;
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
async fn fetch_and_print_motd() {
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
    #[structopt(long = "cert-path")]
    cert_path: Option<String>,
}

fn configure_logging() {
    let stdout = ConsoleAppender::builder().build();

    let mut config_path = cbsh_home_path().unwrap();
    config_path.push("sdk.log");

    let requests = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build(config_path)
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("requests", Box::new(requests)))
        .logger(
            Logger::builder()
                .appender("requests")
                .additive(false)
                .build("couchbase", LevelFilter::Trace),
        )
        .build(Root::builder().appender("stdout").build(LevelFilter::Error))
        .unwrap();

    log4rs::init_config(config).unwrap();
}
