use crate::client::{CapellaClient, Endpoint};

use crate::tutorial::Tutorial;
use crate::RemoteCluster;
use nu_protocol::LabeledError;
use nu_protocol::ShellError;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::{collections::HashMap, time::Duration};

#[derive(Debug, Clone)]
pub struct TransactionState {
    id: String,
    endpoint: Endpoint,
}

impl TransactionState {
    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn endpoint(&self) -> Endpoint {
        self.endpoint.clone()
    }
}

#[derive(Debug)]
pub struct LLM {
    api_key: String,
}

impl LLM {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub fn api_key(&self) -> String {
        self.api_key.clone()
    }
}

pub struct State {
    active: Mutex<String>,
    clusters: HashMap<String, RemoteCluster>,
    tutorial: Tutorial,
    config_path: Option<PathBuf>,
    capella_orgs: HashMap<String, RemoteCapellaOrganization>,
    active_capella_org: Mutex<Option<String>>,
    active_transaction: Mutex<Option<TransactionState>>,
    llm: Option<LLM>,
}

impl State {
    pub fn new(
        clusters: HashMap<String, RemoteCluster>,
        active: String,
        config_path: Option<PathBuf>,
        capella_orgs: HashMap<String, RemoteCapellaOrganization>,
        active_capella_org: Option<String>,
        llm: Option<LLM>,
    ) -> Self {
        let state = Self {
            active: Mutex::new(active.clone()),
            clusters,
            tutorial: Tutorial::new(),
            config_path,
            capella_orgs,
            active_capella_org: Mutex::new(active_capella_org),
            active_transaction: Mutex::new(None),
            llm,
        };
        if !active.is_empty() {
            state.set_active(active).unwrap();
        }
        state
    }

    pub fn add_cluster(&mut self, alias: String, cluster: RemoteCluster) -> Result<(), ShellError> {
        if self.clusters.contains_key(alias.as_str()) {
            return Err(ShellError::GenericError {
                error: format!("Identifier {} is already registered to a cluster", alias),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: Vec::new(),
            });
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
            return Err(
                LabeledError::new(format!("The cluster named {} is not known", active)).into(),
            );
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
                remote.set_active_scope(Some(s));
            }
            if let Some(c) = remote.active_collection() {
                remote.set_active_collection(Some(c));
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
        let active = match self.active_capella_org_name() {
            Some(a) => a,
            None => {
                return Err(ShellError::GenericError {
                    error: "No active Capella organization set".to_string(),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: Vec::new(),
                })
            }
        };

        self.capella_orgs
            .get(&active)
            .ok_or_else(|| ShellError::GenericError {
                error: "Active Capella organization not known".to_string(),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: Vec::new(),
            })
    }

    pub fn active_capella_org_name(&self) -> Option<String> {
        self.active_capella_org.lock().unwrap().clone()
    }

    pub fn set_active_capella_org(&self, active: String) -> Result<(), ShellError> {
        if !self.capella_orgs.contains_key(&active) {
            return Err(ShellError::GenericError {
                error: "Capella organization not known".to_string(),
                msg: format!("Capella organization {} has not been registered", active),
                span: None,
                help: None,
                inner: Vec::new(),
            });
        }

        {
            let mut guard = self.active_capella_org.lock().unwrap();
            *guard = Some(active);
        }

        Ok(())
    }

    pub fn set_active_capella_org_id(&mut self, id: String) -> Result<(), ShellError> {
        let active = match self.active_capella_org_name() {
            Some(a) => a,
            None => {
                return Err(ShellError::GenericError {
                    error: "No active Capella organization set".to_string(),
                    msg: "".to_string(),
                    span: None,
                    help: None,
                    inner: Vec::new(),
                })
            }
        };

        let orgs = &mut self.capella_orgs;
        let org = match orgs.get_mut(&active) {
            Some(org) => org,
            None => {
                return Err(ShellError::GenericError {
                    error: "Capella organization not known".to_string(),
                    msg: format!("Capella organization {} has not been registered", active),
                    span: None,
                    help: None,
                    inner: Vec::new(),
                })
            }
        };
        org.set_id(id);
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
            Err(ShellError::GenericError {
                error: format!(
                    "No cloud organization registered for cluster name {}",
                    identifier,
                ),
                msg: "".to_string(),
                span: None,
                help: None,
                inner: Vec::new(),
            })
        }
    }

    pub fn active_transaction(&self) -> Option<TransactionState> {
        self.active_transaction.lock().unwrap().clone()
    }

    pub fn start_transaction(&mut self, id: String, endpoint: Endpoint) -> Result<(), ShellError> {
        {
            let mut guard = self.active_transaction.lock().unwrap();
            *guard = Some(TransactionState { id, endpoint });
        }

        Ok(())
    }

    pub fn end_transaction(&mut self) {
        {
            let mut guard = self.active_transaction.lock().unwrap();
            *guard = None;
        }
    }

    pub fn llm(&self) -> &Option<LLM> {
        &self.llm
    }
}

pub struct RemoteCapellaOrganization {
    id: Option<String>,
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
            id: None,
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

    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    pub fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}
