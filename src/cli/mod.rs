mod addresses;
mod addresses_add;
mod addresses_drop;
mod analytics;
mod analytics_buckets;
mod analytics_datasets;
mod analytics_dataverses;
mod analytics_indexes;
mod analytics_links;
mod analytics_pending_mutations;
mod buckets;
mod buckets_builder;
mod buckets_config;
mod buckets_create;
mod buckets_drop;
mod buckets_flush;
mod buckets_get;
mod buckets_sample;
mod buckets_update;
mod cloud_json;
mod clouds;
mod clouds_clusters;
mod clouds_clusters_create;
mod clouds_clusters_drop;
mod clouds_clusters_get;
mod clouds_status;
mod clusters;
mod clusters_health;
mod clusters_register;
mod clusters_unregister;
mod collections;
mod collections_create;
mod ctrlc_future;
mod doc;
mod doc_get;
mod doc_insert;
mod doc_remove;
mod doc_replace;
mod doc_upsert;
mod fake_data;
mod help;
mod nodes;
mod ping;
mod plugin_fetch;
mod projects;
mod projects_create;
mod projects_drop;
mod query;
mod query_advise;
mod query_indexes;
mod scopes;
mod scopes_create;
mod search;
mod transactions;
mod transactions_list_atrs;
mod tutorial;
mod tutorial_next;
mod tutorial_page;
mod tutorial_prev;
mod use_bucket;
mod use_cloud;
mod use_cloud_organization;
mod use_cluster;
mod use_cmd;
mod use_collection;
mod use_project;
mod use_scope;
mod user_builder;
mod users;
mod users_drop;
mod users_get;
mod users_roles;
mod users_upsert;
mod util;
mod version;
mod whoami;

use std::sync::{Arc, Mutex};

pub use addresses::Addresses;
pub use addresses_add::AddressesAdd;
pub use addresses_drop::AddressesDrop;
pub use analytics::Analytics;
pub use analytics_buckets::AnalyticsBuckets;
pub use analytics_datasets::AnalyticsDatasets;
pub use analytics_dataverses::AnalyticsDataverses;
pub use analytics_indexes::AnalyticsIndexes;
pub use analytics_links::AnalyticsLinks;
pub use analytics_pending_mutations::AnalyticsPendingMutations;
pub use buckets::Buckets;
pub use buckets_config::BucketsConfig;
pub use buckets_create::BucketsCreate;
pub use buckets_drop::BucketsDrop;
pub use buckets_flush::BucketsFlush;
pub use buckets_get::BucketsGet;
pub use buckets_sample::BucketsSample;
pub use buckets_update::BucketsUpdate;
pub use clouds::Clouds;
pub use clouds_clusters::CloudsClusters;
pub use clouds_clusters_create::CloudsClustersCreate;
pub use clouds_clusters_drop::CloudsClustersDrop;
pub use clouds_clusters_get::CloudsClustersGet;
pub use clouds_status::CloudsStatus;
pub use clusters::Clusters;
pub use clusters_health::ClustersHealth;
pub use clusters_register::ClustersRegister;
pub use clusters_unregister::ClustersUnregister;
pub use collections::Collections;
pub use collections_create::CollectionsCreate;
pub use ctrlc_future::CtrlcFuture;
pub use doc::Doc;
pub use doc_get::DocGet;
pub use doc_insert::DocInsert;
pub use doc_remove::DocRemove;
pub use doc_replace::DocReplace;
pub use doc_upsert::DocUpsert;
pub use fake_data::FakeData;
pub use help::Help;
pub use nodes::Nodes;
use nu_engine::EvaluationContext;
pub use ping::Ping;
pub use plugin_fetch::PluginFetch;
pub use projects::Projects;
pub use projects_create::ProjectsCreate;
pub use projects_drop::ProjectsDrop;
pub use query::Query;
pub use query_advise::QueryAdvise;
pub use query_indexes::QueryIndexes;
pub use scopes::Scopes;
pub use scopes_create::ScopesCreate;
pub use search::Search;
pub use transactions::Transactions;
pub use transactions_list_atrs::TransactionsListAtrs;
pub use tutorial::Tutorial;
pub use tutorial_next::TutorialNext;
pub use tutorial_page::TutorialPage;
pub use tutorial_prev::TutorialPrev;
pub use use_bucket::UseBucket;
pub use use_cloud::UseCloud;
pub use use_cloud_organization::UseCloudOrganization;
pub use use_cluster::UseCluster;
pub use use_cmd::UseCmd;
pub use use_collection::UseCollection;
pub use use_project::UseProject;
pub use use_scope::UseScope;
pub use user_builder::User;
pub use users::Users;
pub use users_drop::UsersDrop;
pub use users_get::UsersGet;
pub use users_roles::UsersRoles;
pub use users_upsert::UsersUpsert;
pub use version::Version;
pub use whoami::Whoami;

use crate::state::State;

pub fn add_commands(ctx: &EvaluationContext, state: Arc<Mutex<State>>) {
    ctx.add_commands(vec![
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
        nu_engine::whole_stream_command(PluginFetch::new()),
        nu_engine::whole_stream_command(Projects::new(state.clone())),
        nu_engine::whole_stream_command(ProjectsCreate::new(state.clone())),
        nu_engine::whole_stream_command(ProjectsDrop::new(state.clone())),
        nu_engine::whole_stream_command(Query::new(state.clone())),
        nu_engine::whole_stream_command(QueryAdvise::new(state.clone())),
        nu_engine::whole_stream_command(QueryIndexes::new(state.clone())),
        nu_engine::whole_stream_command(ScopesCreate::new(state.clone())),
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
        nu_engine::whole_stream_command(Whoami::new(state.clone())),
        nu_engine::whole_stream_command(Version::new()),
    ]);
}
