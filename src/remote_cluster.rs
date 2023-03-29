use crate::client::{Client, CAPELLA_SRV_SUFFIX};
use crate::ClusterTlsConfig;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RemoteClusterType {
    Provisioned,
    Other,
}

impl From<Vec<String>> for RemoteClusterType {
    fn from(hostnames: Vec<String>) -> Self {
        if hostnames.len() == 1 && hostnames[0].contains(&CAPELLA_SRV_SUFFIX.to_string()) {
            // This means that this is a Capella host.
            RemoteClusterType::Provisioned
        } else {
            RemoteClusterType::Other
        }
    }
}

impl From<String> for RemoteClusterType {
    fn from(cluster_type: String) -> Self {
        if cluster_type == "provisioned".to_string() {
            RemoteClusterType::Provisioned
        } else {
            RemoteClusterType::Other
        }
    }
}

impl Into<String> for RemoteClusterType {
    fn into(self) -> String {
        match self {
            RemoteClusterType::Provisioned => "provisioned",
            RemoteClusterType::Other => "other",
        }
        .to_string()
    }
}

pub struct RemoteClusterResources {
    pub hostnames: Vec<String>,
    pub username: String,
    pub password: String,
    pub active_bucket: Option<String>,
    pub active_scope: Option<String>,
    pub active_collection: Option<String>,
    pub display_name: Option<String>,
}

pub struct RemoteCluster {
    hostnames: Vec<String>,
    username: String,
    password: String,
    cluster: Mutex<Option<Arc<Client>>>,
    active_bucket: Mutex<Option<String>>,
    active_scope: Mutex<Option<String>>,
    active_collection: Mutex<Option<String>>,
    tls_config: ClusterTlsConfig,
    timeouts: Mutex<ClusterTimeouts>,
    capella_org: Option<String>,
    kv_batch_size: u32,
    cluster_type: RemoteClusterType,
    display_name: Option<String>,
}

impl RemoteCluster {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resources: RemoteClusterResources,
        tls_config: ClusterTlsConfig,
        timeouts: ClusterTimeouts,
        capella_org: Option<String>,
        kv_batch_size: u32,
        cluster_type: RemoteClusterType,
    ) -> Self {
        Self {
            cluster: Mutex::new(None),
            hostnames: resources.hostnames,
            username: resources.username,
            password: resources.password,
            active_bucket: Mutex::new(resources.active_bucket),
            active_scope: Mutex::new(resources.active_scope),
            active_collection: Mutex::new(resources.active_collection),
            tls_config,
            timeouts: Mutex::new(timeouts),
            capella_org,
            kv_batch_size,
            cluster_type,
            display_name: resources.display_name,
        }
    }

    pub fn cluster(&self) -> Arc<Client> {
        let mut c = self.cluster.lock().unwrap();
        if c.is_none() {
            *c = Some(Arc::new(Client::new(
                self.hostnames.clone(),
                self.username.clone(),
                self.password.clone(),
                self.tls_config.clone(),
            )));
        }
        c.as_ref().unwrap().clone()
    }

    pub fn active_bucket(&self) -> Option<String> {
        self.active_bucket.lock().unwrap().as_ref().cloned()
    }

    pub fn set_active_bucket(&self, name: String) {
        let mut active = self.active_bucket.lock().unwrap();
        *active = Some(name);
    }

    pub fn active_scope(&self) -> Option<String> {
        self.active_scope.lock().unwrap().as_ref().cloned()
    }

    pub fn set_active_scope(&self, name: String) {
        let mut active = self.active_scope.lock().unwrap();
        *active = Some(name);
    }

    pub fn active_collection(&self) -> Option<String> {
        self.active_collection.lock().unwrap().as_ref().cloned()
    }

    pub fn set_active_collection(&self, name: String) {
        let mut active = self.active_collection.lock().unwrap();
        *active = Some(name);
    }

    pub fn deactivate(&self) {
        let mut c = self.cluster.lock().unwrap();
        if c.is_some() {
            *c = None;
        }
    }

    pub fn hostnames(&self) -> &Vec<String> {
        &self.hostnames
    }

    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    pub fn password(&self) -> &str {
        self.password.as_str()
    }

    pub fn tls_config(&self) -> &ClusterTlsConfig {
        &self.tls_config
    }

    pub fn timeouts(&self) -> ClusterTimeouts {
        let active = self.timeouts.lock().unwrap();
        active.clone()
    }

    pub fn set_timeouts(&self, timeouts: ClusterTimeouts) {
        let mut active = self.timeouts.lock().unwrap();
        *active = timeouts
    }

    pub fn capella_org(&self) -> Option<String> {
        self.capella_org.clone()
    }

    pub fn kv_batch_size(&self) -> u32 {
        self.kv_batch_size
    }

    #[allow(dead_code)]
    pub fn cluster_type(&self) -> RemoteClusterType {
        self.cluster_type
    }

    pub fn display_name(&self) -> Option<String> {
        self.display_name.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ClusterTimeouts {
    data_timeout: Duration,
    query_timeout: Duration,
    analytics_timeout: Duration,
    search_timeout: Duration,
    management_timeout: Duration,
}

impl Default for ClusterTimeouts {
    fn default() -> Self {
        ClusterTimeouts {
            data_timeout: Duration::from_millis(30000),
            query_timeout: Duration::from_millis(75000),
            analytics_timeout: Duration::from_millis(75000),
            search_timeout: Duration::from_millis(75000),
            management_timeout: Duration::from_millis(75000),
        }
    }
}

impl ClusterTimeouts {
    pub fn new(
        data_timeout: Duration,
        query_timeout: Duration,
        analytics_timeout: Duration,
        search_timeout: Duration,
        management_timeout: Duration,
    ) -> Self {
        ClusterTimeouts {
            data_timeout,
            query_timeout,
            analytics_timeout,
            search_timeout,
            management_timeout,
        }
    }

    pub fn data_timeout(&self) -> Duration {
        self.data_timeout
    }

    pub fn query_timeout(&self) -> Duration {
        self.query_timeout
    }

    pub fn analytics_timeout(&self) -> Duration {
        self.analytics_timeout
    }

    pub fn search_timeout(&self) -> Duration {
        self.search_timeout
    }

    pub fn management_timeout(&self) -> Duration {
        self.management_timeout
    }

    pub fn set_analytics_timeout(&mut self, duration: Duration) {
        self.analytics_timeout = duration
    }

    pub fn set_search_timeout(&mut self, duration: Duration) {
        self.search_timeout = duration
    }

    pub fn set_query_timeout(&mut self, duration: Duration) {
        self.query_timeout = duration
    }

    pub fn set_data_timeout(&mut self, duration: Duration) {
        self.data_timeout = duration
    }

    pub fn set_management_timeout(&mut self, duration: Duration) {
        self.management_timeout = duration
    }
}
