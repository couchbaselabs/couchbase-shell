use crate::cli::nodes::NodeService;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClusterSummary {
    id: String,
    name: String,
    #[serde(rename = "tenantId")]
    tenant_id: String,
    #[serde(rename = "cloudId")]
    cloud_id: String,
    #[serde(rename = "projectId")]
    project_id: String,
    services: Vec<String>,
    nodes: i64,
}

impl JSONCloudClusterSummary {
    pub fn id(&self) -> String {
        self.id.clone()
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    // pub fn tenant_id(&self) -> String {
    //     self.tenant_id.clone()
    // }
    // pub fn cloud_id(&self) -> String {
    //     self.cloud_id.clone()
    // }
    // pub fn project_id(&self) -> String {
    //     self.project_id.clone()
    // }
    // pub fn services(&self) -> &Vec<String> {
    //     &self.services
    // }
    // pub fn nodes(&self) -> i64 {
    //     self.nodes
    // }
}

#[derive(Debug, Serialize)]
pub(crate) struct JSONCloudDeleteBucketRequest {
    name: String,
}

impl JSONCloudDeleteBucketRequest {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Debug, Deserialize)]
pub struct JSONCloudUser {
    #[serde(rename = "userId", default)]
    user_id: Option<String>,
    username: String,
    access: Vec<JSONCloudUserRoles>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JSONCloudUserRoles {
    #[serde(rename = "bucketName")]
    bucket_name: String,
    #[serde(rename = "bucketAccess")]
    bucket_access: Vec<String>,
}

impl JSONCloudUser {
    pub fn username(&self) -> String {
        self.username.clone()
    }

    pub fn roles(&self) -> Vec<JSONCloudUserRoles> {
        self.access.clone()
    }
}

impl JSONCloudUserRoles {
    pub fn new(bucket: String, names: Vec<String>) -> Self {
        Self {
            bucket_name: bucket,
            bucket_access: names,
        }
    }
    pub fn bucket(&self) -> String {
        self.bucket_name.clone()
    }

    pub fn names(&self) -> Vec<String> {
        self.bucket_access.clone()
    }
}

#[derive(Debug, Serialize)]
pub struct JSONCloudCreateUserRequest {
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    password: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    buckets: Vec<JSONCloudUserRoles>,
    #[serde(rename = "allBucketsAccess", skip_serializing_if = "String::is_empty")]
    all_buckets_access: String,
}

impl JSONCloudCreateUserRequest {
    pub fn new(
        username: String,
        password: String,
        buckets: Vec<JSONCloudUserRoles>,
        all_buckets_access: String,
    ) -> Self {
        Self {
            user_id: None,
            username,
            password,
            buckets,
            all_buckets_access,
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct JSONCloudClusterHealthResponseNodeServices {
    #[serde(rename = "nodeName")]
    node_name: String,
    services: Vec<NodeService>,
    status: String,
}

impl JSONCloudClusterHealthResponseNodeServices {
    pub fn name(&self) -> String {
        self.node_name.clone()
    }
    pub fn status(&self) -> String {
        self.status.clone()
    }
    pub fn services(&self) -> &Vec<NodeService> {
        self.services.as_ref()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct JSONCloudClusterHealthResponseNodes {
    #[serde(rename = "serviceStats")]
    service_stats: Vec<JSONCloudClusterHealthResponseNodeServices>,
}

impl JSONCloudClusterHealthResponseNodes {
    pub fn nodes(&self) -> &Vec<JSONCloudClusterHealthResponseNodeServices> {
        self.service_stats.as_ref()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct JSONCloudClusterHealthResponse {
    status: String,
    health: String,
    #[serde(rename = "nodeStats")]
    nodes: JSONCloudClusterHealthResponseNodes,
}

impl JSONCloudClusterHealthResponse {
    pub fn status(&self) -> String {
        self.status.clone()
    }
    pub fn health(&self) -> String {
        self.health.clone()
    }
    pub fn nodes(&self) -> &JSONCloudClusterHealthResponseNodes {
        &self.nodes
    }
}

#[derive(Debug, Serialize)]
pub struct JSONCloudAppendAllowListRequest {
    #[serde(rename = "cidrBlock")]
    cidr_block: String,
    #[serde(rename = "ruleType")]
    rule_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<String>,
}

impl JSONCloudAppendAllowListRequest {
    pub fn new(cidr_block: String, rule_type: String, duration: Option<String>) -> Self {
        Self {
            cidr_block,
            rule_type,
            duration,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct JSONCloudDeleteAllowListRequest {
    #[serde(rename = "cidrBlock")]
    cidr_block: String,
}

impl JSONCloudDeleteAllowListRequest {
    pub fn new(cidr_block: String) -> Self {
        Self { cidr_block }
    }
}

#[derive(Debug, Deserialize)]
pub struct JSONCloudGetAllowListResponse {
    #[serde(rename = "cidrBlock")]
    cidr_block: String,
    #[serde(rename = "ruleType")]
    rule_type: String,
    state: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    duration: Option<String>,
}

impl JSONCloudGetAllowListResponse {
    pub fn address(&self) -> String {
        self.cidr_block.clone()
    }
    pub fn rule_type(&self) -> String {
        self.rule_type.clone()
    }
    pub fn state(&self) -> String {
        self.state.clone()
    }
    pub fn created_at(&self) -> String {
        self.created_at.clone()
    }
    pub fn updated_at(&self) -> String {
        self.updated_at.clone()
    }
    pub fn duration(&self) -> Option<String> {
        self.duration.clone()
    }
}
