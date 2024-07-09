use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudBucketsV4Response {
    data: Vec<JSONCloudsBucketsV4ResponseItem>,
}

impl JSONCloudBucketsV4Response {
    pub fn items(self) -> Vec<JSONCloudsBucketsV4ResponseItem> {
        self.data
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JSONCloudsBucketsV4ResponseItem {
    name: String,
    #[serde(alias = "type")]
    bucket_type: String,
    memory_allocation_in_mb: u64,
    durability_level: String,
    replicas: u32,
    flush: bool,
    time_to_live_in_seconds: u64,
}

impl JSONCloudsBucketsV4ResponseItem {
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn ram_quota(&self) -> u64 {
        self.memory_allocation_in_mb
    }
    pub fn flush(&self) -> bool {
        self.flush
    }
    pub fn replicas(&self) -> u32 {
        self.replicas
    }
    pub fn bucket_type(&self) -> String {
        self.bucket_type.clone()
    }
    pub fn ttl_seconds(&self) -> u64 {
        self.time_to_live_in_seconds
    }
    pub fn durability_level(&self) -> String {
        self.durability_level.clone()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClustersV4Response {
    data: Vec<JSONCloudsClustersV4ResponseItem>,
}

impl JSONCloudClustersV4Response {
    pub fn items(&self) -> &Vec<JSONCloudsClustersV4ResponseItem> {
        self.data.as_ref()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JSONCloudsClustersV4ResponseItem {
    id: String,
    app_service_id: Option<String>,
    name: String,
    current_state: String,
    configuration_type: String,
    description: String,
    couchbase_server: CouchbaseServer,
    connection_string: String,
    cloud_provider: CloudProvider,
    service_groups: Vec<ServiceGroup>,
    availability: Availability,
    support: Support,
    audit_data: Option<AuditData>,
    cmek_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CloudProvider {
    #[serde(rename(serialize = "type"))]
    #[serde(alias = "type")]
    provider: String,
    region: String,
    cidr: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CouchbaseServer {
    version: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditData {
    created_by: String,
    created_at: String,
    modified_by: String,
    modified_at: String,
    version: i32,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudsOrganizationsResponse {
    data: Vec<JSONCloudsOrganizationsResponseItem>,
}

impl JSONCloudsOrganizationsResponse {
    pub fn items(&self) -> &Vec<JSONCloudsOrganizationsResponseItem> {
        self.data.as_ref()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudsOrganizationsResponseItem {
    id: String,
    name: String,
}

impl JSONCloudsOrganizationsResponseItem {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
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
#[serde(rename_all = "camelCase")]
pub(crate) struct JSONCloudCreateClusterRequestV4 {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    configuration_type: Option<String>,
    cloud_provider: CloudProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    couchbase_server: Option<CouchbaseServer>,
    service_groups: Vec<ServiceGroup>,
    availability: Availability,
    support: Support,
    #[serde(skip_serializing_if = "Option::is_none")]
    zones: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cmek_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Support {
    plan: String,
    timezone: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Availability {
    #[serde(rename(serialize = "type"))]
    #[serde(alias = "type")]
    availability_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceGroup {
    node: Node,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_of_nodes: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    services: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Node {
    compute: Compute,
    disk: Disk,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Compute {
    cpu: i32,
    ram: i32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Disk {
    #[serde(rename(serialize = "type"))]
    #[serde(alias = "type")]
    disk_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iops: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_expansion: Option<bool>,
}

impl JSONCloudsClustersV4ResponseItem {
    pub fn id(&self) -> String {
        self.id.clone()
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn state(&self) -> String {
        self.current_state.clone()
    }
    pub fn configuration_type(&self) -> String {
        self.configuration_type.clone()
    }
    pub fn description(&self) -> String {
        self.description.clone()
    }
    pub fn couchbase_server(&self) -> &CouchbaseServer {
        &self.couchbase_server
    }
    pub fn connection_string(&self) -> String {
        self.connection_string.clone()
    }
    pub fn cloud_provider(&self) -> &CloudProvider {
        &self.cloud_provider
    }
    pub fn service_groups(&self) -> &Vec<ServiceGroup> {
        &self.service_groups
    }
    pub fn availability(&self) -> &Availability {
        &self.availability
    }
    pub fn support(&self) -> &Support {
        &self.support
    }
    pub fn audit_data(&self) -> Option<&AuditData> {
        self.audit_data.as_ref()
    }
    pub fn app_service_id(&self) -> Option<String> {
        self.app_service_id.clone()
    }
    pub fn cmek_id(&self) -> Option<String> {
        self.cmek_id.clone()
    }
}
