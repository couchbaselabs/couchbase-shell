use crate::client::{CapellaClient, Endpoint};

use crate::cli::{
    embed_model_missing, generic_error, no_active_project_error, no_llm_configured,
    organization_not_registered,
};
use crate::tutorial::Tutorial;
use crate::RemoteCluster;
use lazy_static::__Deref;
use nu_protocol::LabeledError;
use nu_protocol::ShellError;
use serde::{Deserialize, Serialize};
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
pub struct Llm {
    api_key: Option<String>,
    provider: Provider,
    embed_model: Option<String>,
    chat_model: Option<String>,
    api_base: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Provider {
    Gemini,
    OpenAI,
    Bedrock,
}

impl Llm {
    pub fn new(
        api_key: Option<String>,
        provider: Provider,
        embed_model: Option<String>,
        chat_model: Option<String>,
        api_base: Option<String>,
    ) -> Self {
        Self {
            api_key,
            provider,
            embed_model,
            chat_model,
            api_base,
        }
    }

    pub fn api_key(&self) -> Option<String> {
        self.api_key.clone()
    }

    pub fn provider(&self) -> Provider {
        self.provider.clone()
    }

    pub fn embed_model(&self) -> Option<String> {
        self.embed_model.clone()
    }

    pub fn chat_model(&self) -> Option<String> {
        self.chat_model.clone()
    }

    pub fn api_base(&self) -> Option<String> {
        self.api_base.clone()
    }
}

pub struct State {
    active: Mutex<String>,
    clusters: HashMap<String, RemoteCluster>,
    tutorial: Tutorial,
    config_path: Option<PathBuf>,
    capella_orgs: HashMap<String, RemoteCapellaOrganization>,
    active_capella_org: Mutex<Option<String>>,
    active_project: Mutex<Option<String>>,
    active_transaction: Mutex<Option<TransactionState>>,
    llms: HashMap<String, Llm>,
    active_llm: Mutex<Option<String>>,
}

impl State {
    pub fn new(
        clusters: HashMap<String, RemoteCluster>,
        active: String,
        config_path: Option<PathBuf>,
        capella_orgs: HashMap<String, RemoteCapellaOrganization>,
        active_capella_org: Option<String>,
        active_project: Option<String>,
        llms: HashMap<String, Llm>,
        active_llm: Option<String>,
    ) -> Self {
        let state = Self {
            active: Mutex::new(active.clone()),
            clusters,
            tutorial: Tutorial::new(),
            config_path,
            capella_orgs,
            active_capella_org: Mutex::new(active_capella_org),
            active_project: Mutex::new(active_project),
            active_transaction: Mutex::new(None),
            llms,
            active_llm: Mutex::new(active_llm),
        };
        if !active.is_empty() {
            state.set_active(active).unwrap();
        }
        state
    }

    pub fn add_cluster(&mut self, alias: String, cluster: RemoteCluster) -> Result<(), ShellError> {
        if self.clusters.contains_key(alias.as_str()) {
            return Err(generic_error(
                format!("Identifier {} is already registered to a cluster", alias),
                "Cluster identifiers must be unique, use 'cb-env managed' to check which ones are already in use".to_string(),
                None
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
            return Err(
                LabeledError::new(format!("The cluster named {} is not known", active)).into(),
            );
        }

        {
            let mut guard = self.active.lock().unwrap();
            guard.clone_from(&active)
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

    pub fn named_or_active_org(
        &self,
        organization: Option<String>,
    ) -> Result<&RemoteCapellaOrganization, ShellError> {
        if let Some(org) = organization {
            self.get_capella_org(org)
        } else {
            self.active_capella_org()
        }
    }

    pub fn active_capella_org(&self) -> Result<&RemoteCapellaOrganization, ShellError> {
        let active = match self.active_capella_org_name() {
            Some(a) => a,
            None => {
                return Err(generic_error(
                    "No active Capella org",
                    "Check the docs in couchbase.sh for examples of registering an organization in the config file".to_string(),
                    None
                ));
            }
        };

        self.capella_orgs
            .get(&active)
            .ok_or_else(|| organization_not_registered(active))
    }

    pub fn active_capella_org_name(&self) -> Option<String> {
        self.active_capella_org.lock().unwrap().clone()
    }

    pub fn set_active_capella_org(&self, active: String) -> Result<(), ShellError> {
        if let Some(org) = self.capella_orgs.get(&active) {
            {
                let mut guard = self.active_capella_org.lock().unwrap();
                *guard = Some(active);
            }
            self.set_active_project(org.default_project());
        } else {
            return Err(organization_not_registered(active));
        }

        Ok(())
    }

    pub fn get_capella_org(
        &self,
        identifier: String,
    ) -> Result<&RemoteCapellaOrganization, ShellError> {
        let org = self.capella_orgs.get(identifier.as_str());
        if let Some(c) = org {
            Ok(c)
        } else {
            Err(organization_not_registered(identifier))
        }
    }

    pub fn named_or_active_project(&self, project: Option<String>) -> Result<String, ShellError> {
        if let Some(proj) = project {
            Ok(proj)
        } else {
            self.active_project()
        }
    }

    pub fn active_project(&self) -> Result<String, ShellError> {
        if let Some(active) = self.active_project.lock().unwrap().clone() {
            return Ok(active);
        }
        Err(no_active_project_error(None))
    }

    pub fn set_active_project(&self, active_project: impl Into<Option<String>>) {
        let mut active = self.active_project.lock().unwrap();
        *active = active_project.into();
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

    pub fn active_llm_id(&self) -> Option<String> {
        self.active_llm.lock().unwrap().clone()
    }

    pub fn active_llm(&self) -> Option<&Llm> {
        let active_llm = match self.active_llm.lock().unwrap().deref() {
            Some(active) => self.llms.get(active),
            None => None,
        };
        active_llm
    }

    pub fn active_embed_model(&self) -> Result<String, ShellError> {
        match self.active_llm() {
            Some(m) => match m.embed_model() {
                Some(m) => Ok(m),
                None => Err(embed_model_missing()),
            },
            None => Err(no_llm_configured()),
        }
    }

    pub fn set_active_llm(&self, active: String) -> Result<(), ShellError> {
        if !self.llms.contains_key(&active) {
            return Err(LabeledError::new(format!("The llm named {} is not known", active)).into());
        }

        let mut guard = self.active_llm.lock().unwrap();
        *guard = Some(active);

        Ok(())
    }
}

pub struct RemoteCapellaOrganization {
    secret_key: String,
    access_key: String,
    client: Mutex<Option<Arc<CapellaClient>>>,
    timeout: Duration,
    default_project: Option<String>,
    api_endpoint: String,
}

impl RemoteCapellaOrganization {
    pub fn new(
        secret_key: String,
        access_key: String,
        timeout: Duration,
        default_project: Option<String>,
        api_endpoint: String,
    ) -> Self {
        Self {
            secret_key,
            access_key,
            client: Mutex::new(None),
            timeout,
            default_project,
            api_endpoint,
        }
    }

    pub fn client(&self) -> Arc<CapellaClient> {
        let mut c = self.client.lock().unwrap();
        if c.is_none() {
            *c = Some(Arc::new(CapellaClient::new(
                self.secret_key.clone(),
                self.access_key.clone(),
                self.api_endpoint.clone(),
                self.timeout,
            )));
        }
        c.as_ref().unwrap().clone()
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn default_project(&self) -> Option<String> {
        self.default_project.clone()
    }
}
