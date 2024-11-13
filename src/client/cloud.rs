use crate::cli::CtrlcFuture;
use crate::client::cloud_json::{
    Cluster, ClustersResponse, Collection, CollectionsResponse, ColumnarCluster,
    ColumnarClustersResponse, OrganizationsResponse, ProjectsResponse, ScopesResponse,
};
use crate::client::error::ClientError;
use crate::client::http_handler::{HttpResponse, HttpVerb};
use base64::prelude::BASE64_STANDARD;
use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use log::debug;
use nu_protocol::Signals;
use reqwest::Client;
use sha2::Sha256;
use std::ops::{Add, Sub};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use tokio::{select, time::Instant};

pub const CLOUD_URL: &str = "https://cloudapi.cloud.couchbase.com";
pub const CAPELLA_SRV_SUFFIX: &str = "cloud.couchbase.com";

pub struct CapellaClient {
    secret_key: String,
    access_key: String,
    api_endpoint: String,
    timeout: Duration,
}

impl CapellaClient {
    pub fn new(
        secret_key: String,
        access_key: String,
        api_endpoint: String,
        timeout: Duration,
    ) -> Self {
        Self {
            secret_key,
            access_key,
            api_endpoint,
            timeout,
        }
    }

    fn deadline(&self) -> Instant {
        Instant::now().add(self.timeout)
    }

    fn http_do(
        &self,
        verb: HttpVerb,
        path: &str,
        payload: Option<Vec<u8>>,
        signals: Signals,
    ) -> Result<(String, u16), ClientError> {
        let now = Instant::now();
        if now >= self.deadline() {
            return Err(ClientError::Timeout { key: None });
        }
        let timeout = self.deadline().sub(now);
        let signals_fut = CtrlcFuture::new(signals);

        let uri = format!("{}{}", self.api_endpoint, path);

        let client = Client::new();
        let mut res_builder = match verb {
            HttpVerb::Get => client.get(uri),
            HttpVerb::Delete => client.delete(uri),
            HttpVerb::Put => client.put(uri),
            HttpVerb::Post => client.post(uri),
        };

        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let bearer_payload = format!("{}\n{}\n{}", verb.as_str(), path, now_millis);

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.secret_key.clone().as_bytes()).unwrap();
        mac.update(bearer_payload.as_bytes());
        let mac_result = mac.finalize();

        let bearer = if path.contains("/v4/") {
            format!("Bearer {}", &self.secret_key)
        } else {
            format!(
                "Bearer {}:{}",
                self.access_key.clone(),
                general_purpose::STANDARD.encode(mac_result.into_bytes())
            )
        };

        res_builder = res_builder
            .timeout(timeout)
            .header("content-type", "application/json")
            .header("Authorization", bearer)
            .header("Couchbase-Timestamp", now_millis.to_string());

        if let Some(p) = payload {
            res_builder = res_builder.body(p);
        }

        debug!("Performing Capella management request {:?}", &res_builder);

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let res_fut = res_builder.send();
            select! {
                result = res_fut => {
                    let response = result.map_err(ClientError::from)?;
                    let status = response.status().into();
                    let content = response.text().await.map_err(ClientError::from)?;
                    Ok((content, status))
                },
                () = signals_fut => Err(ClientError::Cancelled{key: None}),
            }
        })
    }

    fn http_get(&self, path: &str, signals: Signals) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Get, path, None, signals)
    }

    fn http_delete(
        &self,
        path: &str,
        payload: Option<Vec<u8>>,
        signals: Signals,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Delete, path, payload, signals)
    }

    fn http_post(
        &self,
        path: &str,
        payload: Option<Vec<u8>>,
        signals: Signals,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Post, path, payload, signals)
    }

    fn http_put(
        &self,
        path: &str,
        payload: Option<Vec<u8>>,
        signals: Signals,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Put, path, payload, signals)
    }

    pub fn capella_request(
        &self,
        request: CapellaRequest,
        signals: Signals,
    ) -> Result<HttpResponse, ClientError> {
        let (content, status) = match request.verb() {
            HttpVerb::Get => self.http_get(request.path().as_str(), signals)?,
            HttpVerb::Post => {
                self.http_post(request.path().as_str(), request.payload(), signals)?
            }
            HttpVerb::Delete => {
                self.http_delete(request.path().as_str(), request.payload(), signals)?
            }
            HttpVerb::Put => self.http_put(request.path().as_str(), request.payload(), signals)?,
        };
        Ok(HttpResponse::new(content, status))
    }

    pub fn list_organizations(
        &self,
        signals: Signals,
    ) -> Result<OrganizationsResponse, ClientError> {
        let request = CapellaRequest::OrganizationList {};
        let response = self.capella_request(request, signals)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        };

        let resp: OrganizationsResponse = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn list_projects(
        &self,
        org_id: String,
        signals: Signals,
    ) -> Result<ProjectsResponse, ClientError> {
        let request = CapellaRequest::ProjectList { org_id };
        let response = self.capella_request(request, signals)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        };

        let resp: ProjectsResponse = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn create_project(
        &self,
        org_id: String,
        name: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ProjectCreate {
            org_id,
            payload: format!("{{\"name\": \"{}\"}}", name),
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 201 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        };

        Ok(())
    }

    pub fn delete_project(
        &self,
        org_id: String,
        project_id: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ProjectDelete { org_id, project_id };
        let response = self.capella_request(request, signals)?;

        if response.status() != 204 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        };

        Ok(())
    }

    pub fn get_cluster(
        &self,
        cluster_name: String,
        org_id: String,
        project_id: String,
        signals: Signals,
    ) -> Result<Cluster, ClientError> {
        let cluster_list = self.list_clusters(org_id, project_id, signals)?;

        for c in cluster_list.items() {
            if c.name() == cluster_name {
                return Ok(c);
            }
        }

        Err(ClientError::CapellaClusterNotFound { name: cluster_name })
    }

    pub fn list_clusters(
        &self,
        org_id: String,
        project_id: String,
        signals: Signals,
    ) -> Result<ClustersResponse, ClientError> {
        let request = CapellaRequest::ClusterList { org_id, project_id };
        let response = self.capella_request(request, signals)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        let resp: ClustersResponse = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn create_cluster(
        &self,
        org_id: String,
        project_id: String,
        payload: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ClusterCreate {
            org_id,
            project_id,
            payload,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 202 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }
        Ok(())
    }

    pub fn delete_cluster(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ClusterDelete {
            org_id,
            project_id,
            cluster_id,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 202 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }
        Ok(())
    }

    pub fn create_columnar_cluster(
        &self,
        org_id: String,
        project_id: String,
        payload: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ColumnarClusterCreate {
            org_id,
            project_id,
            payload,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 202 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }
        Ok(())
    }

    pub fn list_columnar_clusters(
        &self,
        org_id: String,
        project_id: String,
        signals: Signals,
    ) -> Result<ColumnarClustersResponse, ClientError> {
        let request = CapellaRequest::ColumnarClusterList { org_id, project_id };
        let response = self.capella_request(request, signals)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        let resp: ColumnarClustersResponse = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn get_columnar_cluster(
        &self,
        name: String,
        org_id: String,
        project_id: String,
        signals: Signals,
    ) -> Result<ColumnarCluster, ClientError> {
        let resp = self.list_columnar_clusters(org_id, project_id, signals)?;

        for cluster in resp.items() {
            if cluster.name() == name {
                return Ok(cluster);
            }
        }

        Err(ClientError::CapellaClusterNotFound { name })
    }

    pub fn delete_columnar_cluster(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ColumnarClusterDelete {
            org_id,
            project_id,
            cluster_id,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 202 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }
        Ok(())
    }

    pub fn create_credentials(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::CredentialsCreate {
            org_id,
            project_id,
            cluster_id,
            payload,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 201 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }
        Ok(())
    }

    pub fn create_bucket(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::BucketCreate {
            org_id,
            project_id,
            cluster_id,
            payload,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 201 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }
        Ok(())
    }

    pub fn delete_bucket(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::BucketDelete {
            org_id,
            project_id,
            cluster_id,
            bucket_id: BASE64_STANDARD.encode(bucket),
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 204 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }
        Ok(())
    }

    pub fn update_bucket(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket: String,
        payload: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::BucketUpdate {
            org_id,
            project_id,
            cluster_id,
            bucket_id: BASE64_STANDARD.encode(bucket),
            payload,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 204 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }
        Ok(())
    }

    pub fn load_sample_bucket(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        sample: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::BucketLoadSample {
            org_id,
            project_id,
            cluster_id,
            payload: format!("{{\"name\": \"{}\"}}", sample.clone()),
        };
        let response = self.capella_request(request, signals)?;

        // TODO - need to add handling for sample already loaded once AV-82577 is complete
        match response.status() {
            201 => Ok(()),
            422 => Err(ClientError::InvalidSample { sample }),
            _ => Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            }),
        }
    }

    pub fn allow_ip_address(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        address: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::AllowIPAddress {
            org_id,
            project_id,
            cluster_id,
            payload: format!("{{\"cidr\": \"{}\"}}", address.clone()),
        };
        let response = self.capella_request(request, signals)?;

        match response.status() {
            201 => Ok(()),
            _ => Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            }),
        }
    }

    pub fn list_scopes(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket: String,
        signals: Signals,
    ) -> Result<ScopesResponse, ClientError> {
        let request = CapellaRequest::ScopeList {
            org_id,
            project_id,
            cluster_id,
            bucket_id: BASE64_STANDARD.encode(bucket),
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        let resp: ScopesResponse = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_collection(
        &self,
        collection: String,
        expiry: i64,
        namespace: CollectionNamespace,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::CollectionCreate {
            org_id: namespace.org_id,
            project_id: namespace.project_id,
            cluster_id: namespace.cluster_id,
            bucket_id: namespace.bucket_id,
            scope: namespace.scope,
            payload: serde_json::to_string(&Collection::new(collection.clone(), expiry)).unwrap(),
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 201 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn delete_collection(
        &self,
        namespace: CollectionNamespace,
        collection: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::CollectionDelete {
            org_id: namespace.org_id,
            project_id: namespace.project_id,
            cluster_id: namespace.cluster_id,
            bucket_id: namespace.bucket_id,
            scope: namespace.scope,
            collection,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 204 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        Ok(())
    }

    pub fn list_collections(
        &self,
        namespace: CollectionNamespace,
        signals: Signals,
    ) -> Result<CollectionsResponse, ClientError> {
        let request = CapellaRequest::CollectionList {
            org_id: namespace.org_id,
            project_id: namespace.project_id,
            cluster_id: namespace.cluster_id,
            bucket_id: namespace.bucket_id,
            scope: namespace.scope,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        let resp: CollectionsResponse = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn create_scope(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket: String,
        scope: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ScopeCreate {
            org_id,
            project_id,
            cluster_id,
            bucket_id: BASE64_STANDARD.encode(bucket),
            payload: format!("{{\"name\": \"{}\"}}", scope),
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 201 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        };

        Ok(())
    }

    pub fn delete_scope(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket: String,
        scope: String,
        signals: Signals,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ScopeDelete {
            org_id,
            project_id,
            cluster_id,
            bucket_id: BASE64_STANDARD.encode(bucket),
            scope,
        };
        let response = self.capella_request(request, signals)?;

        if response.status() != 204 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        };

        Ok(())
    }
}

#[allow(dead_code)]
pub enum CapellaRequest {
    OrganizationList,
    ProjectCreate {
        org_id: String,
        payload: String,
    },
    AllowIPAddress {
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
    },
    ProjectDelete {
        org_id: String,
        project_id: String,
    },
    ProjectList {
        org_id: String,
    },
    ClusterCreate {
        org_id: String,
        project_id: String,
        payload: String,
    },
    ClusterDelete {
        org_id: String,
        project_id: String,
        cluster_id: String,
    },
    ClusterGet {
        org_id: String,
        project_id: String,
        cluster_id: String,
    },
    ClusterList {
        org_id: String,
        project_id: String,
    },
    ColumnarClusterCreate {
        org_id: String,
        project_id: String,
        payload: String,
    },
    ColumnarClusterDelete {
        org_id: String,
        project_id: String,
        cluster_id: String,
    },
    ColumnarClusterList {
        org_id: String,
        project_id: String,
    },
    BucketCreate {
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
    },
    BucketDelete {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
    },
    BucketGet {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
    },
    BucketList {
        org_id: String,
        project_id: String,
        cluster_id: String,
    },
    BucketLoadSample {
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
    },
    BucketUpdate {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
        payload: String,
    },
    ScopeCreate {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
        payload: String,
    },
    ScopeDelete {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
        scope: String,
    },
    ScopeList {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
    },
    CollectionCreate {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
        scope: String,
        payload: String,
    },
    CollectionDelete {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
        scope: String,
        collection: String,
    },
    CollectionList {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
        scope: String,
    },
    CredentialsCreate {
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
    },
}

impl CapellaRequest {
    pub fn path(&self) -> String {
        match self {
            Self::OrganizationList => "/v4/organizations".into(),
            Self::ProjectDelete { org_id, project_id } => {
                format!("/v4/organizations/{}/projects/{}", org_id, project_id)
            }
            Self::AllowIPAddress {
                org_id,
                project_id,
                cluster_id,
                ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/allowedcidrs",
                    org_id, project_id, cluster_id
                )
            }
            Self::ProjectCreate { org_id, .. } => {
                format!("/v4/organizations/{}/projects", org_id)
            }
            Self::ProjectList { org_id } => {
                format!("/v4/organizations/{}/projects?perPage=100", org_id)
            }
            Self::ClusterCreate {
                org_id, project_id, ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters",
                    org_id, project_id
                )
            }
            Self::ClusterDelete {
                org_id,
                project_id,
                cluster_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}",
                    org_id, project_id, cluster_id
                )
            }
            Self::ClusterGet {
                org_id,
                project_id,
                cluster_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}",
                    org_id, project_id, cluster_id
                )
            }
            Self::ClusterList { org_id, project_id } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters",
                    org_id, project_id
                )
            }
            Self::ColumnarClusterCreate {
                org_id, project_id, ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/analyticsClusters",
                    org_id, project_id
                )
            }
            Self::ColumnarClusterDelete {
                org_id,
                project_id,
                cluster_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/analyticsClusters/{}",
                    org_id, project_id, cluster_id
                )
            }
            Self::ColumnarClusterList { org_id, project_id } => {
                format!(
                    "/v4/organizations/{}/projects/{}/analyticsClusters",
                    org_id, project_id
                )
            }
            Self::BucketCreate {
                org_id,
                project_id,
                cluster_id,
                ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets",
                    org_id, project_id, cluster_id
                )
            }
            Self::BucketDelete {
                org_id,
                project_id,
                cluster_id,
                bucket_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets/{}",
                    org_id, project_id, cluster_id, bucket_id
                )
            }
            Self::BucketGet {
                org_id,
                project_id,
                cluster_id,
                bucket_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets/{}",
                    org_id, project_id, cluster_id, bucket_id
                )
            }
            Self::BucketList {
                org_id,
                project_id,
                cluster_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets",
                    org_id, project_id, cluster_id
                )
            }
            Self::BucketLoadSample {
                org_id,
                project_id,
                cluster_id,
                ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/sampleBuckets",
                    org_id, project_id, cluster_id
                )
            }
            Self::BucketUpdate {
                org_id,
                project_id,
                cluster_id,
                bucket_id,
                ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets/{}",
                    org_id, project_id, cluster_id, bucket_id
                )
            }
            Self::ScopeCreate {
                org_id,
                project_id,
                cluster_id,
                bucket_id,
                ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets/{}/scopes",
                    org_id, project_id, cluster_id, bucket_id
                )
            }
            Self::ScopeDelete {
                org_id,
                project_id,
                cluster_id,
                bucket_id,
                scope,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets/{}/scopes/{}",
                    org_id, project_id, cluster_id, bucket_id, scope
                )
            }
            Self::ScopeList {
                org_id,
                project_id,
                cluster_id,
                bucket_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets/{}/scopes",
                    org_id, project_id, cluster_id, bucket_id,
                )
            }
            Self::CollectionCreate {
                org_id,
                project_id,
                cluster_id,
                bucket_id,
                scope,
                ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets/{}/scopes/{}/collections",
                    org_id, project_id, cluster_id, bucket_id, scope
                )
            }
            Self::CollectionDelete {
                org_id,
                project_id,
                cluster_id,
                bucket_id,
                scope,
                collection,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets/{}/scopes/{}/collections/{}",
                    org_id, project_id, cluster_id, bucket_id, scope, collection
                )
            }
            Self::CollectionList {
                org_id,
                project_id,
                cluster_id,
                bucket_id,
                scope,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets/{}/scopes/{}/collections",
                    org_id, project_id, cluster_id, bucket_id, scope
                )
            }
            Self::CredentialsCreate {
                org_id,
                project_id,
                cluster_id,
                ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/users",
                    org_id, project_id, cluster_id
                )
            }
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::OrganizationList => HttpVerb::Get,
            Self::ProjectCreate { .. } => HttpVerb::Post,
            Self::ProjectDelete { .. } => HttpVerb::Delete,
            Self::ProjectList { .. } => HttpVerb::Get,
            Self::ClusterCreate { .. } => HttpVerb::Post,
            Self::ClusterDelete { .. } => HttpVerb::Delete,
            Self::ClusterGet { .. } => HttpVerb::Get,
            Self::ClusterList { .. } => HttpVerb::Get,
            Self::ColumnarClusterCreate { .. } => HttpVerb::Post,
            Self::ColumnarClusterDelete { .. } => HttpVerb::Delete,
            Self::ColumnarClusterList { .. } => HttpVerb::Get,
            Self::BucketCreate { .. } => HttpVerb::Post,
            Self::BucketDelete { .. } => HttpVerb::Delete,
            Self::BucketGet { .. } => HttpVerb::Get,
            Self::BucketLoadSample { .. } => HttpVerb::Post,
            Self::BucketList { .. } => HttpVerb::Get,
            Self::BucketUpdate { .. } => HttpVerb::Put,
            Self::AllowIPAddress { .. } => HttpVerb::Post,
            Self::ScopeCreate { .. } => HttpVerb::Post,
            Self::ScopeDelete { .. } => HttpVerb::Delete,
            Self::ScopeList { .. } => HttpVerb::Get,
            Self::CollectionCreate { .. } => HttpVerb::Post,
            Self::CollectionDelete { .. } => HttpVerb::Delete,
            Self::CollectionList { .. } => HttpVerb::Get,
            Self::CredentialsCreate { .. } => HttpVerb::Post,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::ProjectCreate { payload, .. } => Some(payload.as_bytes().into()),
            Self::ClusterCreate { payload, .. } => Some(payload.as_bytes().into()),
            Self::ColumnarClusterCreate { payload, .. } => Some(payload.as_bytes().into()),
            Self::BucketCreate { payload, .. } => Some(payload.as_bytes().into()),
            Self::BucketLoadSample { payload, .. } => Some(payload.as_bytes().into()),
            Self::BucketUpdate { payload, .. } => Some(payload.as_bytes().into()),
            Self::AllowIPAddress { payload, .. } => Some(payload.as_bytes().into()),
            Self::ScopeCreate { payload, .. } => Some(payload.as_bytes().into()),
            Self::CollectionCreate { payload, .. } => Some(payload.as_bytes().into()),
            Self::CredentialsCreate { payload, .. } => Some(payload.as_bytes().into()),
            _ => None,
        }
    }
}

pub struct CollectionNamespace {
    org_id: String,
    project_id: String,
    cluster_id: String,
    bucket_id: String,
    scope: String,
}

impl CollectionNamespace {
    pub fn new(
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket: String,
        scope: String,
    ) -> Self {
        Self {
            org_id,
            project_id,
            cluster_id,
            bucket_id: BASE64_STANDARD.encode(bucket),
            scope,
        }
    }
}
