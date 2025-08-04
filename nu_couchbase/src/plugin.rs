use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::{Arc, Mutex};
use log::{error, warn};
use nu_plugin::Plugin;
use tokio::runtime::Runtime;
use crate::cli::{AllowIP, Analytics, AnalyticsBuckets, AnalyticsDatasets, AnalyticsDataverses, AnalyticsIndexes, AnalyticsLinks, AnalyticsPendingMutations, Ask, Buckets, BucketsConfig, BucketsCreate, BucketsDrop, BucketsFlush, BucketsGet, BucketsSample, BucketsUpdate, CBEnvManaged, CbEnvCluster, CbEnvLLM, CbEnvRegister, CbEnvUnregister, Clusters, ClustersCreate, ClustersDrop, ClustersGet, Collections, CollectionsCreate, CollectionsDrop, Columnar, ColumnarClusters, ColumnarClustersCreate, ColumnarClustersDrop, ColumnarDatabases, ColumnarQuery, Credentials, CredentialsCreate, CredentialsDrop, Doc, DocGet, DocImport, DocInsert, DocRemove, DocReplace, DocUpsert, FakeData, HealthCheck, Help, Nodes, Organizations, Ping, Projects, ProjectsCreate, ProjectsDrop, Query, QueryAdvise, QueryIndexes, QueryTransactions, Scopes, ScopesCreate, ScopesDrop, Search, SubDocGet, Transactions, TransactionsListAtrs, Tutorial, TutorialNext, TutorialPage, TutorialPrev, UseBucket, UseCapellaOrganization, UseCmd, UseCollection, UseProject, UseScope, UseTimeouts, Users, UsersDrop, UsersGet, UsersRoles, UsersUpsert, Vector, VectorCreateIndex, VectorEnrichDoc, VectorEnrichText, VectorSearch, Version};
use crate::client::{RustTlsConfig, CLOUD_URL};
use crate::config::{ShellConfig, DEFAULT_ANALYTICS_TIMEOUT, DEFAULT_DATA_TIMEOUT, DEFAULT_KV_BATCH_SIZE, DEFAULT_MANAGEMENT_TIMEOUT, DEFAULT_QUERY_TIMEOUT, DEFAULT_SEARCH_TIMEOUT, DEFAULT_TRANSACTION_TIMEOUT};
use crate::remote_cluster::{ClusterTimeouts, RemoteCluster, RemoteClusterResources};
use crate::state::{Llm, RemoteCapellaOrganization, State};

pub struct CouchbasePlugin {
    pub(crate) runtime: Runtime,
    pub(crate) state: Arc<Mutex<State>>
}

impl CouchbasePlugin {
    pub fn new(runtime: Runtime) -> Self {
        let mut clusters = HashMap::new();
        let config = Self::load_config(&mut clusters);
        let state = Self::make_state(config, clusters);
        Self {
            runtime,
            state,
        }
    }

    fn load_config(
        clusters: &mut HashMap<String, RemoteCluster>,
    ) -> Option<ShellConfig> {
        match ShellConfig::new(None) {
            Some(c) => Some(c),
            None => {
                println!("No config file found");
                let cluster = Self::default_remote_cluster();
                clusters.insert("default".to_string(), cluster);
                None

             /*   println!("Would you like to create one now (Y/n)?");

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

                    }
                }*/
            }
        }
    }

    fn default_remote_cluster() -> RemoteCluster {
        let (cluster_type, hostnames) =
            RemoteCluster::validate_hostnames("localhost".split(',').map(|v| v.to_owned()).collect());
        RemoteCluster::new(
            RemoteClusterResources {
                hostnames,
                username : "username".to_string(),
                password : "password".to_string(),
                active_bucket: None,
                active_scope: None,
                active_collection: None,
                display_name: None,
            },
            None,
            ClusterTimeouts::default(),
            None,
            None,
            DEFAULT_KV_BATCH_SIZE,
            cluster_type,
        )
    }

    fn make_state(
        config: Option<ShellConfig>,
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

                let (cluster_type, hostnames) = RemoteCluster::validate_hostnames(
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
}

impl Plugin for CouchbasePlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![
            Box::new(Buckets::new(self.state.clone()))
  /*          Box::new(AllowIP::new(self.state.clone())),
            Box::new(Analytics::new(self.state.clone())),
            Box::new(AnalyticsBuckets::new(self.state.clone())),
            Box::new(AnalyticsDatasets::new(self.state.clone())),
            Box::new(AnalyticsDataverses::new(self.state.clone())),
            Box::new(AnalyticsIndexes::new(self.state.clone())),
            Box::new(AnalyticsLinks::new(self.state.clone())),
            Box::new(AnalyticsPendingMutations::new(self.state.clone())),
            Box::new(Ask::new(self.state.clone())),
            Box::new(Buckets::new(self.state.clone())),
            Box::new(BucketsConfig::new(self.state.clone())),
            Box::new(BucketsCreate::new(self.state.clone())),
            Box::new(BucketsDrop::new(self.state.clone())),
            Box::new(BucketsFlush::new(self.state.clone())),
            Box::new(BucketsGet::new(self.state.clone())),
            Box::new(BucketsSample::new(self.state.clone())),
            Box::new(BucketsUpdate::new(self.state.clone())),
            Box::new(CbEnvCluster::new(self.state.clone())),
            Box::new(CbEnvLLM::new(self.state.clone())),
            Box::new(CBEnvManaged::new(self.state.clone())),
            Box::new(CbEnvRegister::new(self.state.clone())),
            Box::new(CbEnvUnregister::new(self.state.clone())),
            Box::new(Clusters::new(self.state.clone())),
            Box::new(ClustersCreate::new(self.state.clone())),
            Box::new(ClustersDrop::new(self.state.clone())),
            Box::new(ClustersGet::new(self.state.clone())),
            Box::new(Collections::new(self.state.clone())),
            Box::new(CollectionsCreate::new(self.state.clone())),
            Box::new(CollectionsDrop::new(self.state.clone())),
            Box::new(Columnar),
            Box::new(ColumnarClusters::new(self.state.clone())),
            Box::new(ColumnarClustersCreate::new(self.state.clone())),
            Box::new(ColumnarClustersDrop::new(self.state.clone())),
            Box::new(ColumnarDatabases::new(self.state.clone())),
            Box::new(ColumnarQuery::new(self.state.clone())),
            Box::new(Credentials::new(self.state.clone())),
            Box::new(CredentialsCreate::new(self.state.clone())),
            Box::new(CredentialsDrop::new(self.state.clone())),
            Box::new(Doc),
            Box::new(DocGet::new(self.state.clone())),
            Box::new(DocImport::new(self.state.clone())),
            Box::new(DocInsert::new(self.state.clone())),
            Box::new(DocReplace::new(self.state.clone())),
            Box::new(DocRemove::new(self.state.clone())),
            Box::new(DocUpsert::new(self.state.clone())),
            Box::new(HealthCheck::new(self.state.clone())),
            Box::new(Help),
            Box::new(FakeData::new(self.state.clone())),
            Box::new(Nodes::new(self.state.clone())),
            Box::new(Organizations::new(self.state.clone())),
            Box::new(Ping::new(self.state.clone())),
            Box::new(Projects::new(self.state.clone())),
            Box::new(ProjectsCreate::new(self.state.clone())),
            Box::new(ProjectsDrop::new(self.state.clone())),
            Box::new(Query::new(self.state.clone())),
            Box::new(QueryAdvise::new(self.state.clone())),
            Box::new(QueryIndexes::new(self.state.clone())),
            Box::new(QueryTransactions::new(self.state.clone())),
            Box::new(Scopes::new(self.state.clone())),
            Box::new(ScopesCreate::new(self.state.clone())),
            Box::new(ScopesDrop::new(self.state.clone())),
            Box::new(Search::new(self.state.clone())),
            Box::new(SubDocGet::new(self.state.clone())),
            Box::new(Transactions),
            Box::new(TransactionsListAtrs::new(self.state.clone())),
            Box::new(Tutorial::new(self.state.clone())),
            Box::new(TutorialNext::new(self.state.clone())),
            Box::new(TutorialPage::new(self.state.clone())),
            Box::new(TutorialPrev::new(self.state.clone())),
            Box::new(UseBucket::new(self.state.clone())),
            Box::new(UseCapellaOrganization::new(self.state.clone())),
            Box::new(UseCmd::new(self.state.clone())),
            Box::new(UseCollection::new(self.state.clone())),
            Box::new(UseProject::new(self.state.clone())),
            Box::new(UseScope::new(self.state.clone())),
            Box::new(UseTimeouts::new(self.state.clone())),
            Box::new(Users::new(self.state.clone())),
            Box::new(UsersGet::new(self.state.clone())),
            Box::new(UsersDrop::new(self.state.clone())),
            Box::new(UsersRoles::new(self.state.clone())),
            Box::new(UsersUpsert::new(self.state.clone())),
            Box::new(Vector),
            Box::new(VectorEnrichDoc::new(self.state.clone())),
            Box::new(VectorEnrichText::new(self.state.clone())),
            Box::new(Version),
            Box::new(VectorCreateIndex::new(self.state.clone())),
            Box::new(VectorSearch::new(self.state.clone()))*/
        ]
    }


}