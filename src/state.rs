use crate::client::CapellaClient;

use crate::tutorial::Tutorial;
use crate::RemoteCluster;
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
                "".to_string(),
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
                label: "Cluster not found".to_string(),
                msg: format!("The cluster named {} is not known", active),
                span: None,
            }
            .into());
        }

        {
            let mut guard = self.active.lock().unwrap();
            *guard = active.clone();
        }

        if let Some(remote) = self.active_cluster() {
            let _ = remote.cluster();

            //if remote.active_bucket().is_some() {
            //    let _ = remote.bucket(remote.active_bucket().unwrap().as_str());
            //}

            if let Some(s) = remote.active_scope() {
                remote.set_active_scope(s);
            }
            if let Some(c) = remote.active_collection() {
                remote.set_active_collection(c);
            }
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
                    "No active Capella organization set".to_string(),
                    "".to_string(),
                    None,
                    None,
                    Vec::new(),
                ))
            }
        };

        self.capella_orgs.get(active).ok_or_else(|| {
            ShellError::GenericError(
                "Active Capella organization not known".to_string(),
                "".to_string(),
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
                "Capella organization not known".to_string(),
                format!("Capella organization {} has not been registered", active),
                None,
                None,
                Vec::new(),
            ));
        }

        {
            let mut guard = self.active_capella_org.lock().unwrap();
            *guard = Some(active);
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
                "".to_string(),
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
        self.timeout
    }

    pub fn active_project(&self) -> Option<String> {
        self.active_project.lock().unwrap().clone()
    }

    pub fn set_active_project(&self, name: String) {
        let mut active = self.active_project.lock().unwrap();
        *active = Some(name);
    }
}
