mod analytics;
mod analytics_datasets;
mod analytics_dataverses;
mod analytics_indexes;
mod buckets;
mod buckets_config;
mod clusters;
mod fake_data;
mod kv;
mod kv_get;
mod kv_insert;
mod kv_remove;
mod kv_replace;
mod kv_upsert;
mod map;
mod nodes;
mod query;
mod query_advise;
mod query_indexes;
mod use_bucket;
mod use_cluster;
mod use_cmd;
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
pub use fake_data::FakeData;
pub use kv::Kv;
pub use kv_get::KvGet;
pub use kv_insert::KvInsert;
pub use kv_remove::KvRemove;
pub use kv_replace::KvReplace;
pub use kv_upsert::KvUpsert;
pub use map::Map;
pub use nodes::Nodes;
pub use query::Query;
pub use query_advise::QueryAdvise;
pub use query_indexes::QueryIndexes;
pub use use_bucket::UseBucket;
pub use use_cluster::UseCluster;
pub use use_cmd::UseCmd;
pub use version::Version;
pub use whoami::Whoami;

use couchbase::CouchbaseError;
use nu_errors::ShellError;

fn convert_cb_error<T>(input: Result<T, CouchbaseError>) -> Result<T, ShellError> {
    input.map_err(|e| ShellError::untagged_runtime_error(format!("Couchbase SDK Error: {}", e)))
}
