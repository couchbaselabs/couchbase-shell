mod analytics;
mod analytics_datasets;
mod analytics_dataverses;
mod analytics_indexes;
mod buckets;
mod buckets_config;
mod clusters;
mod clusters_health;
mod ctrlc_future;
mod data;
mod data_stats;
mod doc;
mod doc_get;
mod doc_insert;
mod doc_remove;
mod doc_replace;
mod doc_upsert;
mod fake_data;
#[cfg(not(target_os = "windows"))]
mod map;
mod nodes;
mod ping;
mod query;
mod query_advise;
mod query_indexes;
mod search;
mod use_bucket;
mod use_cluster;
mod use_cmd;
mod users;
mod users_get;
mod users_roles;
mod users_upsert;
mod util;
mod version;
mod whoami;

pub use analytics::Analytics;
pub use analytics_datasets::AnalyticsDatasets;
pub use analytics_dataverses::AnalyticsDataverses;
pub use analytics_indexes::AnalyticsIndexes;
pub use buckets::Buckets;
pub use buckets_config::BucketsConfig;
pub use clusters::Clusters;
pub use clusters_health::ClustersHealth;
pub use data::Data;
pub use data_stats::DataStats;
pub use doc::Doc;
pub use doc_get::DocGet;
pub use doc_insert::DocInsert;
pub use doc_remove::DocRemove;
pub use doc_replace::DocReplace;
pub use doc_upsert::DocUpsert;
pub use fake_data::FakeData;
#[cfg(not(target_os = "windows"))]
pub use map::Map;
pub use nodes::Nodes;
pub use ping::Ping;
pub use query::Query;
pub use query_advise::QueryAdvise;
pub use query_indexes::QueryIndexes;
pub use search::Search;
pub use use_bucket::UseBucket;
pub use use_cluster::UseCluster;
pub use use_cmd::UseCmd;
pub use users::Users;
pub use users_get::UsersGet;
pub use users_roles::UsersRoles;
pub use users_upsert::UsersUpsert;
pub use version::Version;
pub use whoami::Whoami;

use couchbase::CouchbaseError;
use nu_errors::ShellError;

fn convert_cb_error<T>(input: Result<T, CouchbaseError>) -> Result<T, ShellError> {
    input.map_err(|e| ShellError::untagged_runtime_error(format!("Couchbase SDK Error: {}", e)))
}
