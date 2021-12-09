#![recursion_limit = "256"]

mod cli;
mod client;
mod config;
mod state;
mod tutorial;

use crate::config::{
    ShellConfig, DEFAULT_ANALYTICS_TIMEOUT, DEFAULT_DATA_TIMEOUT, DEFAULT_KV_BATCH_SIZE,
    DEFAULT_MANAGEMENT_TIMEOUT, DEFAULT_QUERY_TIMEOUT, DEFAULT_SEARCH_TIMEOUT,
};
use crate::state::{RemoteCloud, RemoteCloudOrganization, RemoteCluster};
use crate::{cli::*, state::ClusterTimeouts};
use config::ClusterTlsConfig;
use env_logger::Env;
use isahc::{prelude::*, Request};
use log::{debug, warn, LevelFilter};
use log::{error, info};
use nu_cli::app::NuScript;
use serde::Deserialize;
use state::State;
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use structopt::StructOpt;
use temp_dir::TempDir;

fn main() -> Result<(), Box<dyn Error>> {
    let mut logger_builder = env_logger::Builder::from_env(
        Env::default().default_filter_or("info,isahc=error,surf=error"),
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

    const DEFAULT_PASSWORD: &str = "password";
    const DEFAULT_HOSTNAME: &str = "localhost";
    const DEFAULT_USERNAME: &str = "Administrator";

    let opt = CliOptions::from_args();
    if opt.silent {
        logger_builder.filter_level(LevelFilter::Error);
    }
    logger_builder.init();

    debug!("Effective {:?}", opt);

    let config = ShellConfig::new();
    debug!("Config {:?}", config);

    let mut clusters = HashMap::new();
    let mut clouds = HashMap::new();
    let mut control_planes = HashMap::new();

    let password = match opt.password {
        true => Some(rpassword::read_password_from_tty(Some("Password: ")).unwrap()),
        false => None,
    };

    let mut default_project: Option<String> = None;
    let mut active_cloud = None;
    let mut active_control_plane = None;
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
        for c in config.cloud_orgs() {
            let management_timeout = match c.management_timeout() {
                Some(t) => t.to_owned(),
                None => DEFAULT_MANAGEMENT_TIMEOUT,
            };
            let name = c.identifier();

            let plane =
                RemoteCloudOrganization::new(c.secret_key(), c.access_key(), management_timeout);

            if active_control_plane.is_none() {
                active_control_plane = Some(name.clone());
            }

            control_planes.insert(name, plane);
        }

        for c in config.clouds() {
            let name = c.identifier();

            let cloud = RemoteCloud::new(c.default_project());

            if active_cloud.is_none() {
                default_project = c.default_project();
                active_cloud = Some(name.clone());
            }

            clouds.insert(name, cloud);
        }
        active.unwrap()
    };

    let state = Arc::new(Mutex::new(State::new(
        clusters,
        active,
        config.location().clone(),
        clouds,
        control_planes,
        active_cloud,
        active_control_plane,
        default_project,
    )));

    if !opt.silent && !opt.no_motd && opt.script.is_none() && opt.command.is_none() {
        fetch_and_print_motd();
    }

    let context = nu_cli::create_default_context(true)?;
    context.add_commands(vec![
        nu_engine::whole_stream_command(Addresses::new(state.clone())),
        nu_engine::whole_stream_command(AddressesAdd::new(state.clone())),
        nu_engine::whole_stream_command(AddressesDrop::new(state.clone())),
        nu_engine::whole_stream_command(Analytics::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsDatasets::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsDataverses::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsIndexes::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsLinks::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsBuckets::new(state.clone())),
        nu_engine::whole_stream_command(AnalyticsPendingMutations::new(state.clone())),
        nu_engine::whole_stream_command(Buckets::new(state.clone())),
        nu_engine::whole_stream_command(BucketsConfig::new(state.clone())),
        nu_engine::whole_stream_command(BucketsCreate::new(state.clone())),
        nu_engine::whole_stream_command(BucketsDrop::new(state.clone())),
        nu_engine::whole_stream_command(BucketsFlush::new(state.clone())),
        nu_engine::whole_stream_command(BucketsGet::new(state.clone())),
        nu_engine::whole_stream_command(BucketsSample::new(state.clone())),
        nu_engine::whole_stream_command(BucketsUpdate::new(state.clone())),
        nu_engine::whole_stream_command(Clouds::new(state.clone())),
        nu_engine::whole_stream_command(CloudsClusters::new(state.clone())),
        nu_engine::whole_stream_command(CloudsClustersCreate::new(state.clone())),
        nu_engine::whole_stream_command(CloudsClustersDrop::new(state.clone())),
        nu_engine::whole_stream_command(CloudsClustersGet::new(state.clone())),
        nu_engine::whole_stream_command(CloudsStatus::new(state.clone())),
        nu_engine::whole_stream_command(Clusters::new(state.clone())),
        nu_engine::whole_stream_command(ClustersHealth::new(state.clone())),
        nu_engine::whole_stream_command(ClustersRegister::new(state.clone())),
        nu_engine::whole_stream_command(ClustersUnregister::new(state.clone())),
        nu_engine::whole_stream_command(CollectionsCreate::new(state.clone())),
        nu_engine::whole_stream_command(CollectionsDrop::new(state.clone())),
        nu_engine::whole_stream_command(Collections::new(state.clone())),
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
        nu_engine::whole_stream_command(PluginFromBson::new()),
        nu_engine::whole_stream_command(Projects::new(state.clone())),
        nu_engine::whole_stream_command(ProjectsCreate::new(state.clone())),
        nu_engine::whole_stream_command(ProjectsDrop::new(state.clone())),
        nu_engine::whole_stream_command(Query::new(state.clone())),
        nu_engine::whole_stream_command(QueryAdvise::new(state.clone())),
        nu_engine::whole_stream_command(QueryIndexes::new(state.clone())),
        nu_engine::whole_stream_command(ScopesCreate::new(state.clone())),
        nu_engine::whole_stream_command(ScopesDrop::new(state.clone())),
        nu_engine::whole_stream_command(Scopes::new(state.clone())),
        nu_engine::whole_stream_command(Search::new(state.clone())),
        nu_engine::whole_stream_command(Transactions {}),
        nu_engine::whole_stream_command(TransactionsListAtrs::new(state.clone())),
        nu_engine::whole_stream_command(Tutorial::new(state.clone())),
        nu_engine::whole_stream_command(TutorialNext::new(state.clone())),
        nu_engine::whole_stream_command(TutorialPage::new(state.clone())),
        nu_engine::whole_stream_command(TutorialPrev::new(state.clone())),
        nu_engine::whole_stream_command(Users::new(state.clone())),
        nu_engine::whole_stream_command(UsersDrop::new(state.clone())),
        nu_engine::whole_stream_command(UsersGet::new(state.clone())),
        nu_engine::whole_stream_command(UsersRoles::new(state.clone())),
        nu_engine::whole_stream_command(UsersUpsert::new(state.clone())),
        nu_engine::whole_stream_command(UseBucket::new(state.clone())),
        nu_engine::whole_stream_command(UseCloud::new(state.clone())),
        nu_engine::whole_stream_command(UseCloudOrganization::new(state.clone())),
        nu_engine::whole_stream_command(UseCluster::new(state.clone())),
        nu_engine::whole_stream_command(UseCmd::new(state.clone())),
        nu_engine::whole_stream_command(UseCollection::new(state.clone())),
        nu_engine::whole_stream_command(UseProject::new(state.clone())),
        nu_engine::whole_stream_command(UseScope::new(state.clone())),
        nu_engine::whole_stream_command(UseTimeouts::new(state.clone())),
        nu_engine::whole_stream_command(Whoami::new(state)),
        nu_engine::whole_stream_command(Version::new()),
        /*
        nu_engine::whole_stream_command(DataStats::new(state.clone())),
        nu_engine::whole_stream_command(Data {}),
        */
    ]);

    let mut options = nu_cli::app::CliOptions::new();

    let d = TempDir::new().unwrap();
    let f = d.child("config.toml");

    let history_path: String = if let Some(p) = config.location() {
        let mut p = p.clone();
        p.pop();
        p.push("history.txt");
        format!("history-path = '{}'", p.to_str().unwrap())
    } else {
        "".into()
    };

    let prompt = if cfg!(windows) {
        r##"prompt = "build-string (ansi ub) (use | get username) (ansi reset) ' at ' (ansi yb) (use | get cluster) (ansi reset) ' in ' (ansi wb) (use | get bucket) (use | select scope collection | each { if $it.scope == \"\" && $it.collection == \"\" { } { build-string (if $it.scope == \"\" { build-string \".<notset>\" } {build-string \".\" $it.scope}) (if $it.collection == \"\" { build-string \".<notset>\"} {build-string \".\" $it.collection})}}) (ansi reset) '\n' '> '""##
    } else {
        r##"prompt = "build-string '👤 ' (ansi ub) (use | get username) (ansi reset) ' 🏠 ' (ansi yb) (use | get cluster) (ansi reset) ' in 🗄 ' (ansi wb) (use | get bucket) (use | select scope collection | each { if $it.scope == \"\" && $it.collection == \"\" { } { build-string (if $it.scope == \"\" { build-string \".<notset>\" } {build-string \".\" $it.scope}) (if $it.collection == \"\" { build-string \".<notset>\"} {build-string \".\" $it.collection})}}) (ansi reset) '\n' '> '""##
    };

    let config = format!("skip_welcome_message = true\n{}\n{}", history_path, prompt);

    std::fs::write(&f, config.as_bytes()).unwrap();

    options.config = Some(std::ffi::OsString::from(f));

    if let Some(c) = opt.command {
        options.scripts = vec![NuScript::code(c.as_str())?];
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
    #[structopt(short = "s", long = "silent")]
    silent: bool,
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
