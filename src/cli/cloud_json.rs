use serde_derive::Deserialize;

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
    pub fn tenant_id(&self) -> String {
        self.tenant_id.clone()
    }
    pub fn cloud_id(&self) -> String {
        self.cloud_id.clone()
    }
    pub fn project_id(&self) -> String {
        self.project_id.clone()
    }
    pub fn services(&self) -> &Vec<String> {
        &self.services
    }
    pub fn nodes(&self) -> i64 {
        self.nodes
    }
}
