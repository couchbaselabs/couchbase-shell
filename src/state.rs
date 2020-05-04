use couchbase::{Bucket, Cluster};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

pub struct State {
    active: Mutex<String>,
    clusters: HashMap<String, RemoteCluster>,
}

impl State {
    pub fn new(clusters: HashMap<String, RemoteCluster>, active: String) -> Self {
        let state = Self {
            active: Mutex::new(active.clone()),
            clusters,
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

        if remote.bucket_on_active().is_some() {
            let _ = remote.bucket(remote.bucket_on_active().unwrap());
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
}

pub struct RemoteCluster {
    connstr: String,
    username: String,
    password: String,
    cluster: Mutex<Option<Arc<Cluster>>>,
    buckets: Mutex<HashMap<String, Arc<Bucket>>>,
    bucket_on_active: Option<String>,
}

impl RemoteCluster {
    pub fn new(
        connstr: String,
        username: String,
        password: String,
        bucket_on_active: Option<String>,
    ) -> Self {
        Self {
            cluster: Mutex::new(None),
            buckets: Mutex::new(HashMap::new()),
            connstr,
            username,
            password,
            bucket_on_active,
        }
    }

    pub fn cluster(&self) -> Arc<Cluster> {
        let mut c = self.cluster.lock().unwrap();
        if c.is_none() {
            *c = Some(Arc::new(Cluster::connect(
                &self.connstr,
                &self.username,
                &self.password,
            )));
        }
        c.as_ref().unwrap().clone()
    }

    pub fn bucket(&self, name: &str) -> Arc<Bucket> {
        let mut buckets = self.buckets.lock().unwrap();
        if !buckets.contains_key(name) {
            let bucket = self.cluster().bucket(name);
            buckets.insert(name.into(), Arc::new(bucket));
        }
        buckets.get(name).unwrap().clone()
    }

    pub fn unique_bucket_name(&self) -> Option<String> {
        let buckets = self.buckets.lock().unwrap();
        if buckets.len() == 1 {
            return buckets.keys().next().map(|s| s.clone());
        }
        None
    }

    pub fn bucket_on_active(&self) -> Option<&String> {
        self.bucket_on_active.as_ref()
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

    pub fn password(&self) -> &str {
        self.password.as_str()
    }

    pub fn connstr(&self) -> &str {
        self.connstr.as_str()
    }
}
