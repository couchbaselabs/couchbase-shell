#![recursion_limit = "256"]

mod cli;
mod config;
mod state;
mod ui;

use crate::cli::*;
use crate::config::{ClusterTimeouts, ShellConfig};
use crate::state::RemoteCluster;
use crate::ui::*;
use ansi_term::Color;
use log::{debug, warn};
use serde::Deserialize;
use state::State;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::sync::Arc;
use std::time::Duration;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    const DEFAULT_PASSWORD: &str = "password";
    const DEFAULT_HOSTNAME: &str = "localhost";
    const DEFAULT_USERNAME: &str = "Administrator";

    pretty_env_logger::init();

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
        let connstr = if let Some(certpath) = opt.cert_path {
            format!(
                "couchbases://{}?certpath={}&{}",
                hostnames, certpath, timeouts,
            )
        } else {
            format!("couchbase://{}?{}", hostnames, timeouts)
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
            connstr,
            username,
            rpassword,
            opt.bucket,
            opt.scope,
            opt.collection,
        );
        clusters.insert("default".into(), cluster);
        String::from("default")
    } else {
        let mut active = None;
        for v in config.clusters() {
            let name = v.identifier().to_owned();

            let mut hostnames = v.hostnames().join(",");
            let mut username = v.username();
            let mut cpassword = v.password();
            let mut default_bucket = v.default_bucket();
            let mut scope = v.default_scope();
            let mut collection = v.default_collection();

            if opt.cluster.as_ref().is_some() {
                if &name == opt.cluster.as_ref().unwrap() {
                    active = Some(name.clone());
                    if let Some(hosts) = opt.hostnames.clone() {
                        hostnames = hosts;
                    }
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
                if let Some(hosts) = opt.hostnames.clone() {
                    hostnames = hosts;
                }
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

            let connstr = if let Some(certpath) = v.cert_path() {
                format!(
                    "couchbases://{}?certpath={}&{}",
                    hostnames,
                    certpath,
                    v.timeouts().export_lcb_args()
                )
            } else {
                format!(
                    "couchbase://{}?{}",
                    hostnames,
                    v.timeouts().export_lcb_args()
                )
            };

            if default_scope.is_none() && scope.is_some() {
                default_scope = scope.clone();
            }
            if default_collection.is_none() && collection.is_some() {
                default_collection = collection.clone();
            }

            let cluster = RemoteCluster::new(
                connstr,
                username,
                cpassword,
                default_bucket,
                scope,
                collection,
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

    if opt.ui {
        tokio::task::spawn(spawn_and_serve(state.clone()));
    }

    if !opt.no_motd && opt.script.is_none() && opt.command.is_none() {
        fetch_and_print_motd().await;
    }

    let syncer = nu_cli::EnvironmentSyncer::new();
    let mut context = nu_cli::create_default_context(true)?;
    context.add_commands(vec![
        // Performs analytics queries
        nu_cli::whole_stream_command(Analytics::new(state.clone())),
        // Performs kv get operations
        nu_cli::whole_stream_command(DocGet::new(state.clone())),
        // Performs kv upsert operations
        nu_cli::whole_stream_command(DocUpsert::new(state.clone())),
        // Performs kv insert operations
        nu_cli::whole_stream_command(DocInsert::new(state.clone())),
        // Performs kv replace operations
        nu_cli::whole_stream_command(DocReplace::new(state.clone())),
        nu_cli::whole_stream_command(DocRemove::new(state.clone())),
        nu_cli::whole_stream_command(DataStats::new(state.clone())),
        // Displays cluster manager node infos
        nu_cli::whole_stream_command(Nodes::new(state.clone())),
        // Displays cluster manager bucket infos
        nu_cli::whole_stream_command(Buckets::new(state.clone())),
        nu_cli::whole_stream_command(BucketsConfig::new(state.clone())),
        // Performs n1ql queries
        nu_cli::whole_stream_command(Query::new(state.clone())),
        // Manages local cluster references
        nu_cli::whole_stream_command(Clusters::new(state.clone())),
        nu_cli::whole_stream_command(ClustersHealth::new(state.clone())),
        // Create fake data based on templates
        nu_cli::whole_stream_command(FakeData::new(state.clone())),
        // Displays indexes
        nu_cli::whole_stream_command(QueryIndexes::new(state.clone())),
        nu_cli::whole_stream_command(QueryAdvise::new(state.clone())),
        nu_cli::whole_stream_command(AnalyticsIndexes::new(state.clone())),
        nu_cli::whole_stream_command(AnalyticsDatasets::new(state.clone())),
        nu_cli::whole_stream_command(AnalyticsDataverses::new(state.clone())),
        // Allows to switch clusters, buckets and collections on the fly
        nu_cli::whole_stream_command(UseCmd::new(state.clone())),
        nu_cli::whole_stream_command(UseBucket::new(state.clone())),
        nu_cli::whole_stream_command(UseCluster::new(state.clone())),
        nu_cli::whole_stream_command(UseCollection::new(state.clone())),
        nu_cli::whole_stream_command(UseScope::new(state.clone())),
        nu_cli::whole_stream_command(Whoami::new(state.clone())),
        nu_cli::whole_stream_command(Version::new()),
        #[cfg(not(target_os = "windows"))]
        nu_cli::whole_stream_command(Map::new(state.clone())),
        nu_cli::whole_stream_command(Doc {}),
        nu_cli::whole_stream_command(Data {}),
        nu_cli::whole_stream_command(Users::new(state.clone())),
        nu_cli::whole_stream_command(UsersGet::new(state.clone())),
        nu_cli::whole_stream_command(UsersUpsert::new(state.clone())),
        nu_cli::whole_stream_command(UsersRoles::new(state.clone())),
        nu_cli::whole_stream_command(Search::new(state.clone())),
        nu_cli::whole_stream_command(Ping::new(state.clone())),
    ]);

    if let Some(c) = opt.command {
        nu_cli::run_pipeline_standalone(c, opt.stdin, &mut context, true).await?;
        return Ok(());
    }

    if let Some(s) = opt.script {
        let file = File::open(s)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            if !line.starts_with('#') {
                nu_cli::run_pipeline_standalone(line, opt.stdin, &mut context, true).await?;
            }
        }
        return Ok(());
    }

    let prompt = CouchbasePrompt {
        state: state.clone(),
    };

    nu_cli::cli(syncer, context, Some(Box::new(prompt))).await
}

struct CouchbasePrompt {
    state: Arc<State>,
}

impl nu_cli::Prompt for CouchbasePrompt {
    fn get(&self) -> String {
        let ac = self.state.active_cluster();

        if let Some(b) = ac.active_bucket() {
            let bucket_emoji = match b.to_lowercase().as_ref() {
                "travel-sample" => "🛫 ",
                "beer-sample" => "🍺 ",
                _ => "🗄 ",
            };

            format!(
                "👤 {} at 🏠 {} in {} {}\n> ",
                Color::Blue.bold().paint(ac.username()),
                Color::Yellow.bold().paint(self.state.active()),
                bucket_emoji,
                Color::White.bold().paint(b)
            )
        } else {
            format!(
                "👤 {} at 🏠 {}\n> ",
                Color::Blue.bold().paint(ac.username()),
                Color::Yellow.bold().paint(self.state.active())
            )
        }
    }
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
    #[structopt(long = "ui")]
    ui: bool,
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
