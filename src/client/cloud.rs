use crate::cli::CtrlcFuture;
use crate::client::error::ClientError;
use crate::client::http_handler::{HttpResponse, HttpVerb};
use hmac::{Hmac, Mac, NewMac};
use isahc::prelude::*;
use isahc::ResponseFuture;
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;
use std::collections::HashMap;
use std::ops::Sub;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use tokio::{select, time::Instant};

const CLOUD_URL: &str = "https://cloudapi.cloud.couchbase.com";

#[derive(Debug, Deserialize)]
struct LimitedClusterSummary {
    id: String,
    name: String,
}

pub struct CloudClient {
    secret_key: String,
    access_key: String,
}

impl CloudClient {
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
            return Err(ClientError::Timeout);
        }
        let timeout = deadline.sub(now);
        let ctrl_c_fut = CtrlcFuture::new(ctrl_c);

        let uri = format!("{}{}", CLOUD_URL, path);

        let mut res_builder = match verb {
            HttpVerb::Get => isahc::Request::get(uri),
            HttpVerb::Delete => isahc::Request::delete(uri),
            HttpVerb::Put => isahc::Request::put(uri),
            HttpVerb::Post => isahc::Request::post(uri),
        };

        let now_millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let bearer_payload = format!(
            "{}\n{}\n{}",
            res_builder.method_ref().unwrap(),
            path,
            now_millis.to_string()
        );

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.secret_key.clone().as_bytes()).unwrap();
        mac.update(bearer_payload.as_bytes());
        let mac_result = mac.finalize();

        let bearer = format!(
            "Bearer {}:{}",
            self.access_key.clone(),
            base64::encode(mac_result.into_bytes())
        );

        res_builder = res_builder
            .timeout(timeout)
            .header("content-type", "application/json")
            .header("Authorization", bearer)
            .header("Couchbase-Timestamp", now_millis.to_string());

        let res_fut: ResponseFuture;
        if let Some(p) = payload {
            res_fut = res_builder.body(p)?.send_async();
        } else {
            res_fut = res_builder.body(())?.send_async();
        }

        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            select! {
                result = res_fut => {
                    let mut response = result.map_err(ClientError::from)?;
                    let content = response.text().await?;
                    let status = response.status().into();
                    Ok((content, status))
                },
                () = ctrl_c_fut => Err(ClientError::Cancelled),
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

    pub fn find_cluster_id(
        &self,
        cluster_name: String,
        deadline: Instant,
        ctrl_c: Arc<AtomicBool>,
    ) -> Result<String, ClientError> {
        let request = CloudRequest::GetClusters {};
        let (content, status) = self.http_get(request.path().as_str(), deadline, ctrl_c)?;

        if status != 200 {
            return Err(ClientError::RequestFailed {
                reason: Some(content),
            });
        }

        let data: Value = serde_json::from_str(content.as_str())?;
        let v = data.get("data").unwrap().to_string();
        let clusters: Vec<LimitedClusterSummary> = serde_json::from_str(v.as_str())?;

        for c in clusters {
            if c.name == cluster_name {
                return Ok(c.id);
            }
        }

        Err(ClientError::ClusterNotFound { name: cluster_name })
    }

    pub fn cloud_request(
        &self,
        request: CloudRequest,
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
        Ok(HttpResponse::new(content, status))
    }
}

pub enum CloudRequest {
    CreateAllowListEntry {
        cluster_id: String,
        payload: String,
    },
    CreateBucket {
        cluster_id: String,
        payload: String,
    },
    CreateCluster {
        payload: String,
    },
    CreateProject {
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
    DeleteCluster {
        cluster_id: String,
    },
    DeleteProject {
        project_id: String,
    },
    DeleteUser {
        cluster_id: String,
        username: String,
    },
    GetAPIStatus,
    GetAllowList {
        cluster_id: String,
    },
    GetBuckets {
        cluster_id: String,
    },
    GetCertificate {
        cluster_id: String,
    },
    GetCloud {
        cloud_id: String,
    },
    GetClouds,
    GetCluster {
        cluster_id: String,
    },
    GetClusterHealth {
        cluster_id: String,
    },
    GetClusters,
    GetClusterStatus {
        cluster_id: String,
    },
    GetProject {
        project_id: String,
    },
    GetProjects,
    GetUsers {
        cluster_id: String,
    },
    GetOrgUsers,
    UpdateAllowList {
        cluster_id: String,
        payload: String,
    },
    UpdateBucket {
        cluster_id: String,
        payload: String,
    },
    UpdateUser {
        cluster_id: String,
        username: String,
        payload: String,
    },
}

impl CloudRequest {
    pub fn path(&self) -> String {
        match self {
            Self::CreateAllowListEntry { cluster_id, .. } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::CreateBucket { cluster_id, .. } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::CreateCluster { .. } => "/v2/clusters".into(),
            Self::CreateProject { .. } => "/v2/projects".into(),
            Self::CreateUser { cluster_id, .. } => {
                format!("/v2/clusters/{}/users", cluster_id)
            }
            Self::DeleteAllowListEntry { cluster_id, .. } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::DeleteBucket { cluster_id, .. } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::DeleteCluster { cluster_id, .. } => {
                format!("/v2/clusters/{}", cluster_id)
            }
            Self::DeleteProject { project_id } => {
                format!("/v2/projects/{}", project_id)
            }
            Self::DeleteUser {
                cluster_id,
                username,
            } => {
                format!("/v2/clusters/{}/users/{}", cluster_id, username)
            }
            Self::GetAPIStatus => "/v2/status".into(),
            Self::GetAllowList { cluster_id } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::GetBuckets { cluster_id } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
            }
            Self::GetCertificate { cluster_id } => {
                format!("/v2/clusters/{}/certificate", cluster_id)
            }
            Self::GetCloud { cloud_id } => {
                format!("/v2/clouds/{}", cloud_id)
            }
            Self::GetClouds => "/v2/clouds".into(),
            Self::GetClusterHealth { cluster_id } => {
                format!("/v2/clusters/{}/health", cluster_id)
            }
            Self::GetCluster { cluster_id } => {
                format!("/v2/clusters/{}", cluster_id)
            }
            Self::GetClusters => "/v2/clusters".into(),
            Self::GetClusterStatus { cluster_id } => {
                format!("/v2/clusters/{}/status", cluster_id)
            }
            Self::GetOrgUsers => "/v2/users".into(),
            Self::GetProject { project_id } => {
                format!("/v2/projects/{}", project_id)
            }
            Self::GetProjects => "/v2/projects".into(),
            Self::GetUsers { cluster_id } => {
                format!("/v2/clusters/{}/users", cluster_id)
            }
            Self::UpdateAllowList { cluster_id, .. } => {
                format!("/v2/clusters/{}/allowlist", cluster_id)
            }
            Self::UpdateBucket { cluster_id, .. } => {
                format!("/v2/clusters/{}/buckets", cluster_id)
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
            Self::CreateCluster { .. } => HttpVerb::Post,
            Self::CreateProject { .. } => HttpVerb::Post,
            Self::CreateUser { .. } => HttpVerb::Post,
            Self::DeleteAllowListEntry { .. } => HttpVerb::Delete,
            Self::DeleteBucket { .. } => HttpVerb::Delete,
            Self::DeleteCluster { .. } => HttpVerb::Delete,
            Self::DeleteProject { .. } => HttpVerb::Delete,
            Self::DeleteUser { .. } => HttpVerb::Delete,
            Self::GetAPIStatus => HttpVerb::Get,
            Self::GetAllowList { .. } => HttpVerb::Get,
            Self::GetBuckets { .. } => HttpVerb::Get,
            Self::GetCertificate { .. } => HttpVerb::Get,
            Self::GetCloud { .. } => HttpVerb::Get,
            Self::GetClouds => HttpVerb::Get,
            Self::GetClusterHealth { .. } => HttpVerb::Get,
            Self::GetCluster { .. } => HttpVerb::Get,
            Self::GetClusters => HttpVerb::Get,
            Self::GetClusterStatus { .. } => HttpVerb::Get,
            Self::GetOrgUsers => HttpVerb::Get,
            Self::GetProject { .. } => HttpVerb::Get,
            Self::GetProjects => HttpVerb::Get,
            Self::GetUsers { .. } => HttpVerb::Get,
            Self::UpdateAllowList { .. } => HttpVerb::Put,
            Self::UpdateBucket { .. } => HttpVerb::Put,
            Self::UpdateUser { .. } => HttpVerb::Put,
        }
    }

    pub fn payload(&self) -> Option<Vec<u8>> {
        match self {
            Self::CreateAllowListEntry { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateBucket { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateCluster { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateProject { payload, .. } => Some(payload.as_bytes().into()),
            Self::CreateUser { payload, .. } => Some(payload.as_bytes().into()),
            Self::DeleteAllowListEntry { payload, .. } => Some(payload.as_bytes().into()),
            Self::DeleteBucket { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpdateAllowList { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpdateBucket { payload, .. } => Some(payload.as_bytes().into()),
            Self::UpdateUser { payload, .. } => Some(payload.as_bytes().into()),
            _ => None,
        }
    }

    pub fn headers(&self) -> HashMap<&str, &str> {
        match self {
            Self::CreateAllowListEntry { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::CreateBucket { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::CreateCluster { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::CreateProject { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::CreateUser { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::DeleteAllowListEntry { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::DeleteBucket { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::UpdateAllowList { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::UpdateBucket { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            Self::UpdateUser { .. } => {
                let mut h = HashMap::new();
                h.insert("Content-Type", "application/json");
                h
            }
            _ => HashMap::new(),
        }
    }
}
