use couchbase::Cluster;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct State {
    active: Mutex<String>,
    clusters: HashMap<String, RemoteCluster>,
}

impl State {
    pub fn new(clusters: HashMap<String, RemoteCluster>, active: String) -> Self {
        Self {
            active: Mutex::new(active),
            clusters,
        }
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
        let mut guard = self.active.lock().unwrap();
        *guard = active;
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
    cluster: Cluster,
}

impl RemoteCluster {
    pub fn new(connstr: String, username: String, password: String) -> Self {
        let cluster = Cluster::connect(&connstr, &username, &password);
        Self {
            cluster,
            connstr,
            username,
            password,
        }
    }

    pub fn cluster(&self) -> &Cluster {
        &self.cluster
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
