use nu_protocol::ShellError;
use serde_derive::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Debug, Deserialize)]
pub(crate) struct OrganizationsResponse {
    data: Vec<Organization>,
}

impl OrganizationsResponse {
    pub fn items(&self) -> &Vec<Organization> {
        self.data.as_ref()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Organization {
    id: String,
    name: String,
}

impl Organization {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProjectsResponse {
    data: Vec<Project>,
}

impl ProjectsResponse {
    pub fn items(&self) -> &Vec<Project> {
        self.data.as_ref()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Project {
    id: String,
    name: String,
}

impl Project {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClustersResponse {
    data: Vec<Cluster>,
}

impl ClustersResponse {
    pub fn items(&self) -> Vec<Cluster> {
        self.data.clone()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Cluster {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CloudProvider {
    #[serde(rename(serialize = "type"))]
    #[serde(alias = "type")]
    provider: String,
    region: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cidr: Option<String>,
}

impl CloudProvider {
    pub fn new(provider: &Provider) -> Self {
        match provider {
            Provider::Aws => Self {
                provider: "aws".into(),
                region: "us-east-1".into(),
                cidr: None,
            },
            Provider::Azure => Self {
                provider: "azure".into(),
                region: "eastus".into(),
                cidr: None,
            },
            Provider::Gcp => Self {
                provider: "gcp".into(),
                region: "us-east1".into(),
                cidr: None,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Provider {
    Aws,
    Gcp,
    Azure,
}

impl TryFrom<&str> for Provider {
    type Error = ShellError;

    fn try_from(alias: &str) -> Result<Self, Self::Error> {
        match alias {
            "aws" => Ok(Provider::Aws),
            "gcp" => Ok(Provider::Gcp),
            "azure" => Ok(Provider::Azure),
            _ => Err(ShellError::GenericError {
                error: "invalid cloud provider".to_string(),
                msg: "".to_string(),
                span: None,
                help: Some("The supported providers are 'aws', 'gcp' and 'azure'".into()),
                inner: vec![],
            }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CouchbaseServer {
    version: String,
}

impl CouchbaseServer {
    pub fn new(version: String) -> Self {
        Self { version }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuditData {
    created_by: String,
    created_at: String,
    modified_by: String,
    modified_at: String,
    version: i32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClusterCreateRequest {
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

impl ClusterCreateRequest {
    pub fn new(
        name: String,
        provider: Provider,
        version: Option<String>,
        num_of_nodes: i32,
    ) -> Self {
        Self {
            name,
            description: Some("A single node cluster created using cbshell".to_string()),
            configuration_type: None,
            cloud_provider: CloudProvider::new(&provider),
            couchbase_server: version.map(CouchbaseServer::new),
            service_groups: vec![{
                ServiceGroup {
                    node: Node {
                        compute: Compute { cpu: 4, ram: 16 },
                        disk: Disk::single_node_disk_from_provider(&provider),
                    },
                    num_of_nodes: Some(num_of_nodes),
                    services: Some(vec![
                        "index".to_string(),
                        "data".to_string(),
                        "query".to_string(),
                        "search".to_string(),
                    ]),
                }
            }],
            availability: Availability {
                availability_type: "single".into(),
            },
            support: Support {
                plan: "basic".into(),
                timezone: None,
            },
            zones: None,
            cmek_id: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Support {
    plan: String,
    timezone: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Availability {
    #[serde(rename(serialize = "type"))]
    #[serde(alias = "type")]
    availability_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServiceGroup {
    node: Node,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_of_nodes: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    services: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Node {
    compute: Compute,
    disk: Disk,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Compute {
    cpu: i32,
    ram: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

impl Disk {
    pub fn single_node_disk_from_provider(provider: &Provider) -> Self {
        match provider {
            Provider::Aws => Self {
                disk_type: "gp3".into(),
                storage: Some(50),
                iops: Some(3000),
                auto_expansion: None,
            },
            Provider::Azure => Self {
                disk_type: "P6".into(),
                storage: None,
                iops: None,
                auto_expansion: None,
            },
            Provider::Gcp => Self {
                disk_type: "pd-ssd".into(),
                storage: Some(50),
                iops: None,
                auto_expansion: None,
            },
        }
    }
}

impl Cluster {
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

#[derive(Debug, Deserialize)]
pub(crate) struct BucketsResponse {
    data: Vec<Bucket>,
}

impl BucketsResponse {
    pub fn items(self) -> Vec<Bucket> {
        self.data
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Bucket {
    name: String,
    #[serde(alias = "type")]
    bucket_type: String,
    memory_allocation_in_mb: u64,
    durability_level: String,
    replicas: u32,
    flush: bool,
    time_to_live_in_seconds: u64,
}

impl Bucket {
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

#[derive(Debug, Serialize)]
pub(crate) struct CredentialsCreateRequest {
    name: String,
    password: String,
    access: Vec<Access>,
}

#[derive(Debug, Serialize)]
struct Access {
    privileges: Vec<String>,
}

impl CredentialsCreateRequest {
    pub fn new(name: String, password: String) -> Self {
        Self {
            name,
            password,
            access: vec![Access {
                privileges: vec!["write".to_string()],
            }],
        }
    }
}
