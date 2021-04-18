use crate::{client::Client, config::ClusterTlsConfig};

use crate::tutorial::Tutorial;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

pub struct State {
    active: Mutex<String>,
    clusters: HashMap<String, RemoteCluster>,
    default_scope: Option<String>,
    default_collection: Option<String>,
    tutorial: Tutorial,
}

impl State {
    pub fn new(
        clusters: HashMap<String, RemoteCluster>,
        active: String,
        default_scope: Option<String>,
        default_collection: Option<String>,
    ) -> Self {
        let state = Self {
            active: Mutex::new(active.clone()),
            clusters,
            default_scope,
            default_collection,
            tutorial: Tutorial::new(),
        };
        state.set_active(active).unwrap();
        state
    }

    pub fn clusters(&self) -> &HashMap<String, RemoteCluster> {
        &self.clusters
    }

    pub fn active(&self) -> String {
        self.active.lock().unwrap().clone()
    }

    pub fn set_active(&self, active: String) -> Result<(), u32> {
        if !self.clusters.contains_key(&active) {
            return Err(1); // make me proper!
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
        self.active_bucket
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.clone())
    }

    pub fn set_active_bucket(&self, name: String) {
        let mut active = self.active_bucket.lock().unwrap();
        *active = Some(name);
    }

    pub fn active_scope(&self) -> Option<String> {
        self.active_scope
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.clone())
    }

    pub fn set_active_scope(&self, name: String) {
        let mut active = self.active_scope.lock().unwrap();
        *active = Some(name);
    }

    pub fn active_collection(&self) -> Option<String> {
        self.active_collection
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.clone())
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

    pub fn username(&self) -> &str {
        self.username.as_str()
    }

    pub fn tls_config(&self) -> &ClusterTlsConfig {
        &self.tls_config
    }
}
