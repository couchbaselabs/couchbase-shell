use crate::client::{CapellaClient, Client};
use crate::config::ClusterTlsConfig;
use crate::tutorial::Tutorial;
use nu_plugin::LabeledError;
use nu_protocol::ShellError;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::HashMap, time::Duration};

pub struct State {
    active: Mutex<String>,
    clusters: HashMap<String, RemoteCluster>,
    tutorial: Tutorial,
    config_path: Option<PathBuf>,
    capella_orgs: HashMap<String, RemoteCapellaOrganization>,
    active_capella_org: Mutex<Option<String>>,
}

impl State {
    pub fn new(
        clusters: HashMap<String, RemoteCluster>,
        active: String,
        config_path: Option<PathBuf>,
        capella_orgs: HashMap<String, RemoteCapellaOrganization>,
        active_capella_org: Option<String>,
    ) -> Self {
        let state = Self {
            active: Mutex::new(active.clone()),
            clusters,
            tutorial: Tutorial::new(),
            config_path,
            capella_orgs,
            active_capella_org: Mutex::new(active_capella_org),
        };
        if !active.is_empty() {
            state.set_active(active).unwrap();
        }
        state
    }

    pub fn add_cluster(&mut self, alias: String, cluster: RemoteCluster) -> Result<(), ShellError> {
        if self.clusters.contains_key(alias.as_str()) {
            return Err(ShellError::GenericError(
                format!("Identifier {} is already registered to a cluster", alias),
                "".into(),
                None,
                None,
                Vec::new(),
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
            return Err(LabeledError {
                label: "Cluster not found".into(),
                msg: format!("The cluster named {} is not known", active),
                span: None,
            }
            .into());
        }

        {
            let mut guard = self.active.lock().unwrap();
            *guard = active.clone();
        }

        match self.active_cluster() {
            Some(remote) => {
                let _ = remote.cluster();

                //if remote.active_bucket().is_some() {
                //    let _ = remote.bucket(remote.active_bucket().unwrap().as_str());
                //}

                if let Some(s) = remote.active_scope().clone() {
                    let _ = remote.set_active_scope(s);
                }
                if let Some(c) = remote.active_collection().clone() {
                    let _ = remote.set_active_collection(c);
                }
            }
            None => {}
        }

        for (k, v) in &self.clusters {
            if k != &active {
                v.deactivate()
            }
        }

        Ok(())
    }

    pub fn active_cluster(&self) -> Option<&RemoteCluster> {
        let active = self.active.lock().unwrap();
        self.clusters.get(&*active)
    }

    pub fn tutorial(&self) -> &Tutorial {
        &self.tutorial
    }

    pub fn config_path(&self) -> &Option<PathBuf> {
        &self.config_path
    }

    pub fn capella_orgs(&self) -> &HashMap<String, RemoteCapellaOrganization> {
        &self.capella_orgs
    }

    pub fn active_capella_org(&self) -> Result<&RemoteCapellaOrganization, ShellError> {
        let guard = self.active_capella_org.lock().unwrap();

        let active = match guard.deref() {
            Some(a) => a,
            None => {
                return Err(ShellError::GenericError(
                    "No active Capella organization set".into(),
                    "".into(),
                    None,
                    None,
                    Vec::new(),
                ))
            }
        };

        self.capella_orgs.get(&*active).ok_or_else(|| {
            ShellError::GenericError(
                "Active Capella organization not known".into(),
                "".into(),
                None,
                None,
                Vec::new(),
            )
        })
    }

    pub fn active_capella_org_name(&self) -> Option<String> {
        self.active_capella_org.lock().unwrap().clone()
    }

    pub fn set_active_capella_org(&self, active: String) -> Result<(), ShellError> {
        if !self.capella_orgs.contains_key(&active) {
            return Err(ShellError::GenericError(
                "Capella organization not known".into(),
                format!("Capella organization {} has not been registered", active),
                None,
                None,
                Vec::new(),
            ));
        }

        {
            let mut guard = self.active_capella_org.lock().unwrap();
            *guard = Some(active.clone());
        }

        Ok(())
    }

    pub fn capella_org_for_cluster(
        &self,
        identifier: String,
    ) -> Result<&RemoteCapellaOrganization, ShellError> {
        let org = &self.capella_orgs.get(identifier.as_str());
        if let Some(c) = org {
            Ok(c)
        } else {
            Err(ShellError::GenericError(
                format!(
                    "No cloud organization registered for cluster name {}",
                    identifier,
                ),
                "".into(),
                None,
                None,
                Vec::new(),
            ))
        }
    }
}

pub struct RemoteCapellaOrganization {
    secret_key: String,
    access_key: String,
    client: Mutex<Option<Arc<CapellaClient>>>,
    timeout: Duration,
    active_project: Mutex<Option<String>>,
}

impl RemoteCapellaOrganization {
    pub fn new(
        secret_key: String,
        access_key: String,
        timeout: Duration,
        active_project: Option<String>,
    ) -> Self {
        Self {
            secret_key,
            access_key,
            client: Mutex::new(None),
            timeout,
            active_project: Mutex::new(active_project),
        }
    }

    pub fn secret_key(&self) -> String {
        self.secret_key.clone()
    }

    pub fn access_key(&self) -> String {
        self.access_key.clone()
    }

    pub fn client(&self) -> Arc<CapellaClient> {
        let mut c = self.client.lock().unwrap();
        if c.is_none() {
            *c = Some(Arc::new(CapellaClient::new(
                self.secret_key.clone(),
                self.access_key.clone(),
            )));
        }
        c.as_ref().unwrap().clone()
    }

    pub fn timeout(&self) -> Duration {
        self.timeout.clone()
    }

    pub fn active_project(&self) -> Option<String> {
        self.active_project.lock().unwrap().clone()
    }

    pub fn set_active_project(&self, name: String) {
        let mut active = self.active_project.lock().unwrap();
        *active = Some(name);
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
    timeouts: Mutex<ClusterTimeouts>,
    capella_org: Option<String>,
    kv_batch_size: u32,
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
        capella_org: Option<String>,
        kv_batch_size: u32,
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
            timeouts: Mutex::new(timeouts),
            capella_org,
            kv_batch_size,
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
        let x = active.clone();
        x
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
