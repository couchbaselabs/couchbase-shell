use crate::cli::generic_error;
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
    audit: AuditData,
}

impl Project {
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn created_by(&self) -> &str {
        &self.audit.created_by
    }
    pub fn created_at(&self) -> &str {
        &self.audit.created_at
    }
    pub fn modified_by(&self) -> &str {
        &self.audit.modified_by
    }
    pub fn modified_at(&self) -> &str {
        &self.audit.modified_at
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

impl Cluster {
    pub fn total_nodes(&self) -> i32 {
        let mut total = 0;

        for sg in &self.service_groups {
            total += sg.num_of_nodes.unwrap();
        }
        total
    }
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
    pub fn new(provider: &Provider, region: String, cidr: Option<String>) -> Self {
        match provider {
            Provider::Aws => Self {
                provider: "aws".into(),
                region,
                cidr,
            },
            Provider::Azure => Self {
                provider: "azure".into(),
                region,
                cidr,
            },
            Provider::Gcp => Self {
                provider: "gcp".into(),
                region,
                cidr,
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
            _ => Err(generic_error(
                "Unsupported cloud provider",
                "The supported providers are 'aws', 'gcp' and 'azure'".to_string(),
                None,
            )),
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
    description: String,
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
        description: String,
        cidr: Option<String>,
        region: String,
        provider: Provider,
        version: Option<String>,
        num_of_nodes: i32,
    ) -> Self {
        Self {
            name,
            description,
            configuration_type: None,
            cloud_provider: CloudProvider::new(&provider, region, cidr),
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FreeTierClusterCreateRequest {
    name: String,
    description: String,
    cloud_provider: CloudProvider,
}

impl From<ClusterCreateRequest> for FreeTierClusterCreateRequest {
    fn from(cluster_request: ClusterCreateRequest) -> Self {
        FreeTierClusterCreateRequest {
            name: cluster_request.name,
            description: cluster_request.description,
            cloud_provider: cluster_request.cloud_provider,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ColumnarClusterCreateRequest {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    cloud_provider: String,
    region: String,
    nodes: i32,
    support: Support,
    compute: Compute,
    availability: Availability,
}

impl ColumnarClusterCreateRequest {
    pub fn new(name: String, num_of_nodes: i32) -> Self {
        Self {
            name,
            description: Some("A Columnar analytics cluster created using cbshell".to_string()),
            cloud_provider: "aws".into(),
            region: "us-east-1".into(),
            nodes: num_of_nodes,
            support: Support {
                plan: "developer pro".into(),
                timezone: Some("ET".to_string()),
            },
            compute: Compute { cpu: 4, ram: 32 },
            availability: Availability {
                availability_type: "single".into(),
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ColumnarClustersResponse {
    data: Vec<ColumnarCluster>,
}

impl ColumnarClustersResponse {
    pub fn items(&self) -> Vec<ColumnarCluster> {
        self.data.clone()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ColumnarCluster {
    id: String,
    description: Option<String>,
    name: String,
    cloud_provider: String,
    region: String,
    nodes: i64,
    current_state: String,
    support: Support,
    compute: Compute,
    availability: Availability,
}

impl ColumnarCluster {
    pub fn id(&self) -> String {
        self.id.clone()
    }
    pub fn name(&self) -> String {
        self.name.clone()
    }
    pub fn state(&self) -> String {
        self.current_state.clone()
    }
    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }
    pub fn region(&self) -> String {
        self.region.clone()
    }
    pub fn provider(&self) -> String {
        self.cloud_provider.clone()
    }
    pub fn availability(&self) -> &Availability {
        &self.availability
    }
    pub fn compute(&self) -> Compute {
        self.compute.clone()
    }
    pub fn support(&self) -> &Support {
        &self.support
    }
    pub fn nodes(&self) -> i64 {
        self.nodes
    }
}

#[derive(Debug, Deserialize)]
pub struct CollectionsResponse {
    data: Vec<Collection>,
}

impl CollectionsResponse {
    pub fn items(self) -> Vec<Collection> {
        self.data
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Collection {
    name: String,
    #[serde(rename = "maxTTL")]
    max_expiry: i64,
}

impl Collection {
    pub fn new(name: String, max_expiry: i64) -> Collection {
        Collection { name, max_expiry }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn max_expiry(&self) -> i64 {
        self.max_expiry
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct CredentialsCreateRequest {
    name: String,
    password: String,
    access: Vec<Access>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Access {
    privileges: Vec<String>,
    resources: Resources,
}

impl Access {
    pub fn privileges(&self) -> Vec<String> {
        self.privileges.clone()
    }

    // Although resources holds a list of buckets the list only ever has one bucket in it,
    // hence the hardcoded buckets[0] below
    pub fn bucket(&self) -> String {
        self.resources.buckets[0].name.clone()
    }

    pub fn scopes(&self) -> Vec<String> {
        if let Some(scopes) = self.resources.buckets[0].scopes.clone() {
            scopes.iter().map(|s| s.name.clone()).collect()
        } else {
            // If no scopes are listed in the response the access applies to all the scopes
            vec!["*".to_string()]
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Resources {
    buckets: Vec<Bucket>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Bucket {
    name: String,
    scopes: Option<Vec<Scope>>,
}

impl CredentialsCreateRequest {
    pub fn new(
        name: String,
        password: String,
        read: bool,
        write: bool,
        bucket: Option<String>,
        scopes: Vec<String>,
    ) -> Self {
        let mut privileges = vec![];

        if read {
            privileges.push("read".to_string())
        }

        if write {
            privileges.push("write".to_string())
        }

        Self {
            name,
            password,
            access: vec![Access {
                privileges,
                resources: Resources {
                    buckets: vec![Bucket {
                        name: bucket.unwrap_or("*".to_string()),
                        scopes: Some(scopes.iter().map(|s| Scope { name: s.into() }).collect()),
                    }],
                },
            }],
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct CredentialsResponse {
    data: Vec<Credential>,
}

impl CredentialsResponse {
    pub fn data(&self) -> Vec<Credential> {
        self.data.clone()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct Credential {
    id: String,
    name: String,
    audit: AuditData,
    access: Vec<Access>,
}

impl Credential {
    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn access(&self) -> Vec<Access> {
        self.access.clone()
    }
}

#[derive(Debug, Deserialize)]
pub struct ScopesResponse {
    scopes: Vec<Scope>,
}

impl ScopesResponse {
    pub fn scopes(&self) -> Vec<Scope> {
        self.scopes.clone()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Scope {
    name: String,
}

impl Scope {
    pub fn name(&self) -> String {
        self.name.clone()
    }
}
