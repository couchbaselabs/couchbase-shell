use crate::client::{Client, CloudClient};
use crate::config::ClusterTlsConfig;
use crate::tutorial::Tutorial;
use nu_errors::ShellError;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::HashMap, time::Duration};

pub struct State {
    active: Mutex<String>,
    clusters: HashMap<String, RemoteCluster>,
    default_scope: Option<String>,
    default_collection: Option<String>,
    tutorial: Tutorial,
    config_path: Option<PathBuf>,
    clouds: HashMap<String, RemoteCloud>,
    control_pane: Option<RemoteCloudControlPane>,
    active_cloud: Mutex<Option<String>>,
    default_project: Option<String>,
}

impl State {
    pub fn new(
        clusters: HashMap<String, RemoteCluster>,
        active: String,
        default_scope: Option<String>,
        default_collection: Option<String>,
        config_path: Option<PathBuf>,
        clouds: HashMap<String, RemoteCloud>,
        control_pane: Option<RemoteCloudControlPane>,
        active_cloud: Option<String>,
        default_project: Option<String>,
    ) -> Self {
        let state = Self {
            active: Mutex::new(active.clone()),
            clusters,
            default_scope,
            default_collection,
            tutorial: Tutorial::new(),
            config_path,
            clouds,
            control_pane,
            active_cloud: Mutex::new(active_cloud),
            default_project,
        };
        state.set_active(active).unwrap();
        state
    }

    pub fn add_cluster(&mut self, alias: String, cluster: RemoteCluster) -> Result<(), ShellError> {
        if self.clusters.contains_key(alias.as_str()) {
            return Err(ShellError::unexpected(
                "identifier is already registered for a cluster",
            ));
        }
        self.clusters.insert(alias, cluster);
        Ok(())
    }

    pub fn remove_cluster(&mut self, alias: String) -> Option<RemoteCluster> {
        self.clusters.remove(alias.as_str())
    }

    pub fn clusters(&self) -> &HashMap<String, RemoteCluster> {
        &self.clusters
    }

    pub fn active(&self) -> String {
        self.active.lock().unwrap().clone()
    }

    pub fn set_active(&self, active: String) -> Result<(), ShellError> {
        if !self.clusters.contains_key(&active) {
            return Err(ShellError::unexpected(format!(
                "Cluster not known: {}",
                active
            )));
        }

        {
            let mut guard = self.active.lock().unwrap();
            *guard = active.clone();
        }

        let remote = self.active_cluster();
        let _ = remote.cluster();

        //if remote.active_bucket().is_some() {
        //    let _ = remote.bucket(remote.active_bucket().unwrap().as_str());
        //}
        if let Some(s) = self.default_scope.clone() {
            let _ = remote.set_active_scope(s);
        }
        if let Some(c) = self.default_collection.clone() {
            let _ = remote.set_active_collection(c);
        }

        for (k, v) in &self.clusters {
            if k != &active {
                v.deactivate()
            }
        }

        Ok(())
    }

    pub fn active_cluster(&self) -> &RemoteCluster {
        let active = self.active.lock().unwrap();
        &self
            .clusters
            .get(&*active)
            .expect("No active cluster, this is a bug :(")
    }

    pub fn tutorial(&self) -> &Tutorial {
        &self.tutorial
    }

    pub fn config_path(&self) -> &Option<PathBuf> {
        &self.config_path
    }

    pub fn clouds(&self) -> &HashMap<String, RemoteCloud> {
        &self.clouds
    }

    pub fn cloud_control_pane(&self) -> Result<&RemoteCloudControlPane, ShellError> {
        if let Some(c) = &self.control_pane {
            Ok(c)
        } else {
            Err(ShellError::unexpected("No cloud control pane registered"))
        }
    }

    pub fn set_active_cloud(&self, active: String) -> Result<(), ShellError> {
        if !self.clouds.contains_key(&active) {
            return Err(ShellError::unexpected(format!(
                "Cloud not known: {}",
                active
            )));
        }

        {
            let mut guard = self.active.lock().unwrap();
            *guard = active.clone();
        }

        match self.active_cloud() {
            Ok(c) => {
                if let Some(p) = self.default_project.clone() {
                    let _ = c.set_active_project(p);
                }
            }
            Err(_e) => {}
        }

        Ok(())
    }

    pub fn active_cloud_name(&self) -> Option<String> {
        self.active_cloud.lock().unwrap().clone()
    }

    pub fn active_cloud(&self) -> Result<&RemoteCloud, ShellError> {
        let guard = self.active_cloud.lock().unwrap();

        let active = match guard.deref() {
            Some(a) => a,
            None => return Err(ShellError::unexpected("No active cloud set")),
        };

        self.clouds
            .get(&*active)
            .ok_or_else(|| ShellError::unexpected("Active cloud name not known"))
    }
}

pub struct RemoteCloud {
    active_project: Mutex<Option<String>>,
}

impl RemoteCloud {
    pub fn new(active_project: Option<String>) -> Self {
        Self {
            active_project: Mutex::new(active_project),
        }
    }
    pub fn active_project(&self) -> Option<String> {
        self.active_project.lock().unwrap().clone()
    }

    pub fn set_active_project(&self, name: String) {
        let mut active = self.active_project.lock().unwrap();
        *active = Some(name);
    }
}

pub struct RemoteCloudControlPane {
    secret_key: String,
    access_key: String,
    client: Mutex<Option<Arc<CloudClient>>>,
    timeout: Duration,
}

impl RemoteCloudControlPane {
    pub fn new(secret_key: String, access_key: String, timeout: Duration) -> Self {
        Self {
            secret_key,
            access_key,
            client: Mutex::new(None),
            timeout,
        }
    }

    pub fn secret_key(&self) -> String {
        self.secret_key.clone()
    }

    pub fn access_key(&self) -> String {
        self.access_key.clone()
    }

    pub fn client(&self) -> Arc<CloudClient> {
        let mut c = self.client.lock().unwrap();
        if c.is_none() {
            *c = Some(Arc::new(CloudClient::new(
                self.secret_key.clone(),
                self.access_key.clone(),
            )));
        }
        c.as_ref().unwrap().clone()
    }

    pub fn timeout(&self) -> Duration {
        self.timeout.clone()
    }
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
    timeouts: ClusterTimeouts,
    cloud: bool,
}

impl RemoteCluster {
    pub fn new(
        hostnames: Vec<String>,
        username: String,
        password: String,
        active_bucket: Option<String>,
        active_scope: Option<String>,
        active_collection: Option<String>,
        tls_config: ClusterTlsConfig,
        timeouts: ClusterTimeouts,
        cloud: bool,
    ) -> Self {
        Self {
            cluster: Mutex::new(None),
            hostnames,
            username,
            password,
            active_bucket: Mutex::new(active_bucket),
            active_scope: Mutex::new(active_scope),
            active_collection: Mutex::new(active_collection),
            tls_config,
            timeouts,
            cloud,
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

    pub fn timeouts(&self) -> &ClusterTimeouts {
        &self.timeouts
    }

    pub fn cloud(&self) -> bool {
        self.cloud
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
}
