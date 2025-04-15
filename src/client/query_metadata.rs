use std::time::Duration;

use serde::Deserialize;
use serde_derive::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum QueryStatus {
    Running,
    Success,
    Errors,
    Completed,
    Stopped,
    Timeout,
    Closed,
    Fatal,
    Aborted,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetaData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared: Option<String>,
    pub request_id: String,
    pub client_context_id: String,
    pub status: QueryStatus,
    pub metrics: QueryMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<Value>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<QueryWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryWarning {
    pub code: u32,
    pub message: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetrics {
    pub elapsed_time: Duration,
    pub execution_time: Duration,
    pub result_count: u64,
    pub result_size: u64,
    pub mutation_count: u64,
    pub sort_count: u64,
    pub error_count: u64,
    pub warning_count: u64,
}

impl From<couchbase_core::queryx::query_result::Metrics> for QueryMetrics {
    fn from(metrics: couchbase_core::queryx::query_result::Metrics) -> Self {
        Self {
            elapsed_time: metrics.elapsed_time,
            execution_time: metrics.execution_time,
            result_count: metrics.result_count,
            result_size: metrics.result_size,
            mutation_count: metrics.mutation_count,
            sort_count: metrics.sort_count,
            error_count: metrics.error_count,
            warning_count: metrics.warning_count,
        }
    }
}

impl From<couchbase_core::queryx::query_result::Warning> for QueryWarning {
    fn from(warning: couchbase_core::queryx::query_result::Warning) -> Self {
        Self {
            code: warning.code,
            message: warning.message,
        }
    }
}

impl From<couchbase_core::queryx::query_result::Status> for QueryStatus {
    fn from(status: couchbase_core::queryx::query_result::Status) -> Self {
        match status {
            couchbase_core::queryx::query_result::Status::Running => QueryStatus::Running,
            couchbase_core::queryx::query_result::Status::Success => QueryStatus::Success,
            couchbase_core::queryx::query_result::Status::Errors => QueryStatus::Errors,
            couchbase_core::queryx::query_result::Status::Completed => QueryStatus::Completed,
            couchbase_core::queryx::query_result::Status::Stopped => QueryStatus::Stopped,
            couchbase_core::queryx::query_result::Status::Timeout => QueryStatus::Timeout,
            couchbase_core::queryx::query_result::Status::Closed => QueryStatus::Closed,
            couchbase_core::queryx::query_result::Status::Fatal => QueryStatus::Fatal,
            couchbase_core::queryx::query_result::Status::Aborted => QueryStatus::Aborted,
            _ => QueryStatus::Unknown,
        }
    }
}

impl From<couchbase_core::queryx::query_result::MetaData> for QueryMetaData {
    fn from(meta_data: couchbase_core::queryx::query_result::MetaData) -> Self {
        Self {
            prepared: meta_data.prepared,
            request_id: meta_data.request_id,
            client_context_id: meta_data.client_context_id,
            status: QueryStatus::from(meta_data.status),
            metrics: QueryMetrics::from(meta_data.metrics),
            signature: serde_json::to_value(meta_data.signature)
                .map(|v| Some(v))
                .unwrap_or(None),
            warnings: meta_data
                .warnings
                .into_iter()
                .map(QueryWarning::from)
                .collect(),
        }
    }
}
