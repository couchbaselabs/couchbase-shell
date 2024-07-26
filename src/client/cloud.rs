use crate::cli::CtrlcFuture;
use crate::client::cloud_json::{
    JSONCloudBucketsV4Response, JSONCloudClustersV4Response, JSONCloudsBucketsV4ResponseItem,
    JSONCloudsClustersV4ResponseItem, JSONCloudsOrganizationsResponse, JSONCloudsProjectsResponse,
};
use crate::client::error::ClientError;
use crate::client::http_handler::{HttpResponse, HttpVerb};
use crate::client::Endpoint;
use base64::prelude::BASE64_STANDARD;
use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use log::debug;
use reqwest::Client;
use sha2::Sha256;
use std::ops::Sub;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use tokio::{select, time::Instant};

const CLOUD_URL: &str = "https://cloudapi.cloud.couchbase.com";
pub const CAPELLA_SRV_SUFFIX: &str = "cloud.couchbase.com";

pub struct CapellaClient {
    secret_key: String,
    access_key: String,
}

impl CapellaClient {
    pub fn new(secret_key: String, access_key: String) -> Self {
        Self {
            secret_key,
            access_key,
        }
    }

    fn http_do(
        &self,
        verb: HttpVerb,
        path: &str,
        payload: Option<Vec<u8>>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        let now = Instant::now();
        if now >= deadline {
            return Err(ClientError::Timeout { key: None });
        }
        let timeout = deadline.sub(now);
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);

        let uri = format!("{}{}", CLOUD_URL, path);

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
                () = ctrl_c_fut => Err(ClientError::Cancelled{key: None}),
            }
        })
    }

    fn http_get(
        &self,
        path: &str,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Get, path, None, deadline, ctrl_c)
    }

    fn http_delete(
        &self,
        path: &str,
        payload: Option<Vec<u8>>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Delete, path, payload, deadline, ctrl_c)
    }

    fn http_post(
        &self,
        path: &str,
        payload: Option<Vec<u8>>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Post, path, payload, deadline, ctrl_c)
    }

    fn http_put(
        &self,
        path: &str,
        payload: Option<Vec<u8>>,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(String, u16), ClientError> {
        self.http_do(HttpVerb::Put, path, payload, deadline, ctrl_c)
    }

    pub fn capella_request(
        &self,
        request: CapellaRequest,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<HttpResponse, ClientError> {
        let (content, status) = match request.verb() {
            HttpVerb::Get => self.http_get(request.path().as_str(), deadline, ctrl_c)?,
            HttpVerb::Post => {
                self.http_post(request.path().as_str(), request.payload(), deadline, ctrl_c)?
            }
            HttpVerb::Delete => {
                self.http_delete(request.path().as_str(), request.payload(), deadline, ctrl_c)?
            }
            HttpVerb::Put => {
                self.http_put(request.path().as_str(), request.payload(), deadline, ctrl_c)?
            }
        };
        // This endpoint is pretty undenyably a hack, but doesn't really matter for now.
        Ok(HttpResponse::new(
            content,
            status,
            Endpoint::new(CLOUD_URL.to_string(), 443),
        ))
    }

    pub fn get_organizations(
        &self,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<JSONCloudsOrganizationsResponse, ClientError> {
        let request = CapellaRequest::GetOrganizations {};
        let response = self.capella_request(request, deadline, ctrl_c)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        };

        let resp: JSONCloudsOrganizationsResponse = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn get_projects(
        &self,
        org_id: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<JSONCloudsProjectsResponse, ClientError> {
        let request = CapellaRequest::GetProjects { org_id };
        let response = self.capella_request(request, deadline, ctrl_c)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        };

        let resp: JSONCloudsProjectsResponse = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn create_project(
        &self,
        org_id: String,
        payload: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::CreateProject { org_id, payload };
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::DeleteProject { org_id, project_id };
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<JSONCloudsClustersV4ResponseItem, ClientError> {
        let request = CapellaRequest::GetClustersV4 { org_id, project_id };
        let response = self.capella_request(request, deadline, ctrl_c)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        let resp: JSONCloudClustersV4Response = serde_json::from_str(response.content())?;

        for c in resp.items() {
            if c.name() == cluster_name {
                return Ok(c);
            }
        }

        Err(ClientError::CapellaClusterNotFound { name: cluster_name })
    }

    pub fn get_clusters(
        &self,
        org_id: String,
        project_id: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<JSONCloudClustersV4Response, ClientError> {
        let request = CapellaRequest::GetClustersV4 { org_id, project_id };
        let response = self.capella_request(request, deadline, ctrl_c)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        let resp: JSONCloudClustersV4Response = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn create_cluster(
        &self,
        org_id: String,
        project_id: String,
        payload: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::CreateClusterV4 {
            org_id,
            project_id,
            payload,
        };
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::DeleteClusterV4 {
            org_id,
            project_id,
            cluster_id,
        };
        let response = self.capella_request(request, deadline, ctrl_c)?;

        if response.status() != 202 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }
        Ok(())
    }

    pub fn get_bucket(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<JSONCloudsBucketsV4ResponseItem, ClientError> {
        let request = CapellaRequest::GetBucketV4 {
            org_id,
            project_id,
            cluster_id,
            bucket_id: BASE64_STANDARD.encode(bucket),
        };
        let response = self.capella_request(request, deadline, ctrl_c)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        let resp: JSONCloudsBucketsV4ResponseItem = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn get_buckets(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<JSONCloudBucketsV4Response, ClientError> {
        let request = CapellaRequest::GetBucketsV4 {
            org_id,
            project_id,
            cluster_id,
        };
        let response = self.capella_request(request, deadline, ctrl_c)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        let resp: JSONCloudBucketsV4Response = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn create_bucket(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::CreateBucketV4 {
            org_id,
            project_id,
            cluster_id,
            payload,
        };
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::DropBucketV4 {
            org_id,
            project_id,
            cluster_id,
            bucket_id: BASE64_STANDARD.encode(bucket),
        };
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::UpdateBucketV4 {
            org_id,
            project_id,
            cluster_id,
            bucket_id: BASE64_STANDARD.encode(bucket),
            payload,
        };
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::LoadSampleBucketV4 {
            org_id,
            project_id,
            cluster_id,
            payload: format!("{{\"name\": \"{}\"}}", sample.clone()),
        };
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
}

#[allow(dead_code)]
pub enum CapellaRequest {
    CreateAllowListEntry {
        cluster_id: String,
        payload: String,
    },
    CreateBucket {
        cluster_id: String,
        payload: String,
    },
    CreateBucketV4 {
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
    },
    CreateCluster {
        payload: String,
    },
    CreateClusterV4 {
        org_id: String,
        project_id: String,
        payload: String,
    },
    CreateProject {
        org_id: String,
        payload: String,
    },
    CreateUser {
        cluster_id: String,
        payload: String,
    },
    DeleteAllowListEntry {
        cluster_id: String,
        payload: String,
    },
    DeleteBucket {
        cluster_id: String,
        payload: String,
    },
    DeleteClusterV4 {
        org_id: String,
        project_id: String,
        cluster_id: String,
    },
    DeleteProject {
        org_id: String,
        project_id: String,
    },
    DeleteUser {
        cluster_id: String,
        username: String,
    },
    DropBucketV4 {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
    },
    // GetAPIStatus,
    GetAllowList {
        cluster_id: String,
    },
    GetBuckets {
        cluster_id: String,
    },
    GetBucketV4 {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
    },
    GetBucketsV4 {
        org_id: String,
        project_id: String,
        cluster_id: String,
    },
    GetClouds,
    GetCluster {
        cluster_id: String,
    },
    GetClusterHealth {
        cluster_id: String,
    },
    GetClustersV4 {
        org_id: String,
        project_id: String,
    },
    GetClusterV4 {
        org_id: String,
        project_id: String,
        cluster_id: String,
    },
    GetOrganizations,
    GetProjects {
        org_id: String,
    },
    GetUsers {
        cluster_id: String,
    },
    LoadSampleBucketV4 {
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
    },
    UpdateBucket {
        cluster_id: String,
        payload: String,
    },
    UpdateBucketV4 {
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket_id: String,
        payload: String,
    },
    UpdateUser {
        cluster_id: String,
        username: String,
        payload: String,
    },
}

impl CapellaRequest {
    pub fn path(&self) -> String {
        match self {
            Self::CreateAllowListEntry { cluster_id, .. } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::CreateBucket { cluster_id, .. } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::CreateBucketV4 {
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
            Self::CreateCluster { .. } => "/v2/clusters".into(),
            Self::CreateClusterV4 {
                org_id, project_id, ..
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters",
                    org_id, project_id
                )
            }
            Self::CreateProject { org_id, .. } => {
                format!("/v4/organizations/{}/projects", org_id)
            }
            Self::CreateUser { cluster_id, .. } => {
                format!("/v2/clusters/{}/users", cluster_id)
            }
            Self::DeleteAllowListEntry { cluster_id, .. } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::DeleteBucket { cluster_id, .. } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::DeleteClusterV4 {
                org_id,
                project_id,
                cluster_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}",
                    org_id, project_id, cluster_id
                )
            }
            Self::DeleteProject { org_id, project_id } => {
                format!("/v4/organizations/{}/projects/{}", org_id, project_id)
            }
            Self::DeleteUser {
                cluster_id,
                username,
            } => {
                format!("/v2/clusters/{}/users/{}", cluster_id, username)
            }
            Self::DropBucketV4 {
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
            Self::GetAllowList { cluster_id } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::GetBuckets { cluster_id } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::GetBucketV4 {
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
            Self::GetBucketsV4 {
                org_id,
                project_id,
                cluster_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}/buckets",
                    org_id, project_id, cluster_id
                )
            }
            Self::GetClouds => "/v2/clouds".into(),
            Self::GetClusterHealth { cluster_id } => {
                format!("/v2/clusters/{}/health", cluster_id)
            }
            Self::GetCluster { cluster_id } => {
                format!("/v2/clusters/{}", cluster_id)
            }
            Self::GetClustersV4 { org_id, project_id } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters",
                    org_id, project_id
                )
            }
            Self::GetClusterV4 {
                org_id,
                project_id,
                cluster_id,
            } => {
                format!(
                    "/v4/organizations/{}/projects/{}/clusters/{}",
                    org_id, project_id, cluster_id
                )
            }
            Self::GetOrganizations => "/v4/organizations".into(),
            Self::GetProjects { org_id } => {
                format!("/v4/organizations/{}/projects?perPage=100", org_id)
            }
            Self::GetUsers { cluster_id } => {
                format!("/v2/clusters/{}/users", cluster_id)
            }
            Self::LoadSampleBucketV4 {
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
            Self::UpdateBucket { cluster_id, .. } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::UpdateBucketV4 {
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
            Self::UpdateUser {
                cluster_id,
                username,
                ..
            } => {
                format!("/v2/clusters/{}/users/{}", cluster_id, username)
            }
        }
    }

    pub fn verb(&self) -> HttpVerb {
        match self {
            Self::CreateAllowListEntry { .. } => HttpVerb::Post,
            Self::CreateBucket { .. } => HttpVerb::Post,
            Self::CreateBucketV4 { .. } => HttpVerb::Post,
            Self::CreateCluster { .. } => HttpVerb::Post,
            Self::CreateClusterV4 { .. } => HttpVerb::Post,
            Self::CreateProject { .. } => HttpVerb::Post,
            Self::CreateUser { .. } => HttpVerb::Post,
            Self::DeleteAllowListEntry { .. } => HttpVerb::Delete,
            Self::DeleteBucket { .. } => HttpVerb::Delete,
            Self::DeleteClusterV4 { .. } => HttpVerb::Delete,
            Self::DeleteProject { .. } => HttpVerb::Delete,
            Self::DeleteUser { .. } => HttpVerb::Delete,
            Self::DropBucketV4 { .. } => HttpVerb::Delete,
            Self::GetAllowList { .. } => HttpVerb::Get,
            Self::GetBuckets { .. } => HttpVerb::Get,
            Self::GetBucketV4 { .. } => HttpVerb::Get,
            Self::GetBucketsV4 { .. } => HttpVerb::Get,
            Self::GetClouds => HttpVerb::Get,
            Self::GetClusterHealth { .. } => HttpVerb::Get,
            Self::GetCluster { .. } => HttpVerb::Get,
            Self::GetClustersV4 { .. } => HttpVerb::Get,
            Self::GetClusterV4 { .. } => HttpVerb::Get,
            Self::GetOrganizations => HttpVerb::Get,
            Self::GetProjects { .. } => HttpVerb::Get,
            Self::GetUsers { .. } => HttpVerb::Get,
            Self::LoadSampleBucketV4 { .. } => HttpVerb::Post,
            Self::UpdateBucket { .. } => HttpVerb::Put,
            Self::UpdateBucketV4 { .. } => HttpVerb::Put,
            Self::UpdateUser { .. } => HttpVerb::Put,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::CreateAllowListEntry { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateBucket { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateBucketV4 { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateCluster { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateClusterV4 { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateProject { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateUser { payload, .. } => Some(payload.as_bytes().into()),
            Self::DeleteAllowListEntry { payload, .. } => Some(payload.as_bytes().into()),
            Self::DeleteBucket { payload, .. } => Some(payload.as_bytes().into()),
            Self::LoadSampleBucketV4 { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpdateBucket { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpdateBucketV4 { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpdateUser { payload, .. } => Some(payload.as_bytes().into()),
            _ => None,
        }
    }
}
