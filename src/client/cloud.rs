use crate::cli::CtrlcFuture;
use crate::client::cloud_json::{
    Bucket, BucketsResponse, Cluster, ClustersResponse, OrganizationsResponse, ProjectsResponse,
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

    pub fn list_organizations(
        &self,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<OrganizationsResponse, ClientError> {
        let request = CapellaRequest::OrganizationList {};
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<ProjectsResponse, ClientError> {
        let request = CapellaRequest::ProjectList { org_id };
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ProjectCreate {
            org_id,
            payload: format!("{{\"name\": \"{}\"}}", name),
        };
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
        let request = CapellaRequest::ProjectDelete { org_id, project_id };
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
    ) -> Result<Cluster, ClientError> {
        let request = CapellaRequest::ClusterList { org_id, project_id };
        let response = self.capella_request(request, deadline, ctrl_c)?;

        if response.status() != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            });
        }

        let resp: ClustersResponse = serde_json::from_str(response.content())?;

        for c in resp.items() {
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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<ClustersResponse, ClientError> {
        let request = CapellaRequest::ClusterList { org_id, project_id };
        let response = self.capella_request(request, deadline, ctrl_c)?;

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
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::ClusterCreate {
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
        let request = CapellaRequest::ClusterDelete {
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

    pub fn create_credentials(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        payload: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::CredentialsCreate {
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

    pub fn get_bucket(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        bucket: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<Bucket, ClientError> {
        let request = CapellaRequest::BucketGet {
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

        let resp: Bucket = serde_json::from_str(response.content())?;
        Ok(resp)
    }

    pub fn list_buckets(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<BucketsResponse, ClientError> {
        let request = CapellaRequest::BucketList {
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

        let resp: BucketsResponse = serde_json::from_str(response.content())?;
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
        let request = CapellaRequest::BucketCreate {
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
        let request = CapellaRequest::BucketDelete {
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
        let request = CapellaRequest::BucketUpdate {
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
        let request = CapellaRequest::BucketLoadSample {
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

    pub fn allow_ip_address(
        &self,
        org_id: String,
        project_id: String,
        cluster_id: String,
        address: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<(), ClientError> {
        let request = CapellaRequest::AllowIPAddress {
            org_id,
            project_id,
            cluster_id,
            payload: format!("{{\"cidr\": \"{}\"}}", address.clone()),
        };
        let response = self.capella_request(request, deadline, ctrl_c)?;

        match response.status() {
            201 => Ok(()),
            _ => Err(ClientError::RequestFailed {
                reason: Some(response.content().into()),
                key: None,
            }),
        }
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
            Self::BucketCreate { .. } => HttpVerb::Post,
            Self::BucketDelete { .. } => HttpVerb::Delete,
            Self::BucketGet { .. } => HttpVerb::Get,
            Self::BucketLoadSample { .. } => HttpVerb::Post,
            Self::BucketList { .. } => HttpVerb::Get,
            Self::BucketUpdate { .. } => HttpVerb::Put,
            Self::AllowIPAddress { .. } => HttpVerb::Post,
            Self::CredentialsCreate { .. } => HttpVerb::Post,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::ProjectCreate { payload, .. } => Some(payload.as_bytes().into()),
            Self::ClusterCreate { payload, .. } => Some(payload.as_bytes().into()),
            Self::BucketCreate { payload, .. } => Some(payload.as_bytes().into()),
            Self::BucketLoadSample { payload, .. } => Some(payload.as_bytes().into()),
            Self::BucketUpdate { payload, .. } => Some(payload.as_bytes().into()),
            Self::AllowIPAddress { payload, .. } => Some(payload.as_bytes().into()),
            Self::CredentialsCreate { payload, .. } => Some(payload.as_bytes().into()),
            _ => None,
        }
    }
}
