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
    pub fn services(&self) -> &Vec<String> {
        &self.services
    }
    pub fn nodes(&self) -> i64 {
        self.nodes
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClustersSummaries {
    data: Vec<JSONCloudClusterSummary>,
}

impl JSONCloudClustersSummaries {
    pub fn items(&self) -> &Vec<JSONCloudClusterSummary> {
        self.data.as_ref()
    }
}
#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClusterSummaryV3 {
    id: String,
    name: String,
    #[serde(rename = "projectId")]
    project_id: String,
    environment: String,
}

impl JSONCloudClusterSummaryV3 {
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
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClustersSummariesItemsV3 {
    items: Vec<JSONCloudClusterSummaryV3>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClustersSummariesV3 {
    data: JSONCloudClustersSummariesItemsV3,
}

impl JSONCloudClustersSummariesV3 {
    pub fn items(&self) -> &Vec<JSONCloudClusterSummaryV3> {
        self.data.items.as_ref()
    }
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

impl Default for JSONCloudClusterHealthResponseNodeServices {
    fn default() -> Self {
        Self {
            node_name: "".to_string(),
            status: "".to_string(),
            services: vec![],
        }
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

impl Default for JSONCloudClusterHealthResponseNodes {
    fn default() -> Self {
        Self {
            service_stats: vec![],
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct JSONCloudClusterHealthResponse {
    status: String,
    health: String,
    #[serde(default)]
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

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudsResponseItem {
    id: String,
    name: String,
    status: String,
    provider: String,
    region: String,
    // #[serde(rename = "virtualNetworkID")]
    // virtual_network_id: String,
    // #[serde(rename = "virtualNetworkCIDR")]
    // virtual_network_cidr: String,
}

impl JSONCloudsResponseItem {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn status(&self) -> &str {
        &self.status
    }
    pub fn provider(&self) -> &str {
        &self.provider
    }
    pub fn region(&self) -> &str {
        &self.region
    }
    // pub fn virtual_network_id(&self) -> &str {
    //     &self.virtual_network_id
    // }
    // pub fn virtual_network_cidr(&self) -> &str {
    //     &self.virtual_network_cidr
    // }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudsResponse {
    data: Vec<JSONCloudsResponseItem>,
}

impl JSONCloudsResponse {
    pub fn items(&self) -> &Vec<JSONCloudsResponseItem> {
        self.data.as_ref()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudsProjectsResponseItem {
    id: String,
    name: String,
    //     #[serde(rename = "tenantId")]
    //     tenant_id: String,
    //     #[serde(rename = "createdAt")]
    //     created_at: String,
}

impl JSONCloudsProjectsResponseItem {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudsProjectsResponse {
    data: Vec<JSONCloudsProjectsResponseItem>,
}

impl JSONCloudsProjectsResponse {
    pub fn items(&self) -> &Vec<JSONCloudsProjectsResponseItem> {
        self.data.as_ref()
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct JSONCloudCreateProjectRequest {
    name: String,
}

impl JSONCloudCreateProjectRequest {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterServerAWS {
    #[serde(rename = "instanceSize")]
    instance_size: String,
    #[serde(rename = "ebsSizeGib")]
    size_gb: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterServerAzure {
    #[serde(rename = "instanceSize")]
    instance_size: String,
    #[serde(rename = "volumeType")]
    volume_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterServer {
    size: u32,
    services: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aws: Option<JSONCloudCreateClusterServerAWS>,
    #[serde(skip_serializing_if = "Option::is_none")]
    azure: Option<JSONCloudCreateClusterServerAzure>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterServerStorageV3 {
    size: u32,
    #[serde(rename = "IOPS")]
    iops: u32,
    #[serde(rename = "type")]
    typ: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterServerV3 {
    size: u32,
    services: Vec<String>,
    compute: String,
    storage: JSONCloudCreateClusterServerStorageV3,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterPlaceHostedV3 {
    provider: String,
    #[serde(rename = "CIDR")]
    cidr: String,
    region: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterPlaceV3 {
    #[serde(rename = "singleAZ")]
    single_az: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    hosted: Option<JSONCloudCreateClusterPlaceHostedV3>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterServerSupportPackage {
    #[serde(rename = "type")]
    support_type: String,
    timezone: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterRequest {
    name: String,
    #[serde(default)]
    #[serde(rename = "cloudId")]
    cloud_id: String,
    #[serde(default)]
    #[serde(rename = "projectId")]
    project_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    servers: Vec<JSONCloudCreateClusterServer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "supportPackage")]
    support_package: Option<JSONCloudCreateClusterServerSupportPackage>,
    version: Option<String>,
}

impl JSONCloudCreateClusterRequest {
    pub fn set_cloud_id(&mut self, id: String) {
        self.cloud_id = id
    }
    pub fn set_project_id(&mut self, id: String) {
        self.project_id = id
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct JSONCloudCreateClusterRequestV3 {
    #[serde(rename = "clusterName")]
    name: String,
    environment: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default)]
    #[serde(rename = "projectId")]
    project_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    servers: Vec<JSONCloudCreateClusterServerV3>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "supportPackage")]
    support_package: Option<JSONCloudCreateClusterServerSupportPackage>,
    place: JSONCloudCreateClusterPlaceV3,
}

impl JSONCloudCreateClusterRequestV3 {
    pub fn set_project_id(&mut self, id: String) {
        self.project_id = id
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClusterVersion {
    name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudCluster {
    id: String,
    name: String,
    #[serde(rename = "tenantId")]
    tenant_id: String,
    #[serde(rename = "cloudId")]
    cloud_id: String,
    #[serde(rename = "projectId")]
    project_id: String,
    status: String,
    version: JSONCloudClusterVersion,
    #[serde(default)]
    #[serde(rename = "endpointsURL")]
    endpoints_url: Vec<String>,
    #[serde(default)]
    #[serde(rename = "endpointsSrv")]
    endpoints_srv: Option<String>,
}

impl JSONCloudCluster {
    pub fn id(&self) -> String {
        self.id.clone()
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn tenant_id(&self) -> String {
        self.tenant_id.clone()
    }
    pub fn cloud_id(&self) -> String {
        self.cloud_id.clone()
    }
    pub fn project_id(&self) -> String {
        self.project_id.clone()
    }
    pub fn status(&self) -> String {
        self.status.clone()
    }
    pub fn version_name(&self) -> String {
        self.version.name.clone()
    }
    pub fn endpoints_url(&self) -> Vec<String> {
        self.endpoints_url.clone()
    }
    pub fn endpoints_srv(&self) -> Option<String> {
        self.endpoints_srv.as_ref().cloned()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClusterV3 {
    id: String,
    name: String,
    #[serde(rename = "tenantId")]
    tenant_id: String,
    #[serde(rename = "projectId")]
    project_id: String,
    status: String,
    version: JSONCloudClusterVersion,
    #[serde(default)]
    #[serde(rename = "endpointsSrv")]
    endpoints_srv: Option<String>,
    environment: String,
}

impl JSONCloudClusterV3 {
    pub fn id(&self) -> String {
        self.id.clone()
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn tenant_id(&self) -> String {
        self.tenant_id.clone()
    }
    pub fn project_id(&self) -> String {
        self.project_id.clone()
    }
    pub fn status(&self) -> String {
        self.status.clone()
    }
    pub fn version_name(&self) -> String {
        self.version.name.clone()
    }
    pub fn endpoints_srv(&self) -> Option<String> {
        self.endpoints_srv.as_ref().cloned()
    }
    pub fn environment(&self) -> String {
        self.environment.clone()
    }
}
