use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClusterSummaryV3 {
    id: String,
    name: String,
    #[serde(rename = "projectId")]
    project_id: String,
}

impl JSONCloudClusterSummaryV3 {
    pub fn id(&self) -> String {
        self.id.clone()
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn project_id(&self) -> String {
        self.project_id.clone()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct JSONCloudClustersSummariesItemsV3 {
    #[serde(rename = "tenantId", default)]
    tenant_id: String,
    #[serde(default)]
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

    pub fn tenant_id(&self) -> String {
        self.data.tenant_id.clone()
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
    #[serde(rename = "createdBy")]
    created_by: String,
    #[serde(rename = "modifiedAt")]
    modified_at: String,
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
    pub fn created_by(&self) -> String {
        self.created_by.clone()
    }
    pub fn modified_at(&self) -> String {
        self.modified_at.clone()
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
}
