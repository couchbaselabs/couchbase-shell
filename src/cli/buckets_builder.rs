use crate::client::cloud_json::Bucket;
use serde_derive::Deserialize;
use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::time::Duration;

// We use a builder error here to force any errors to have to be translated into shell error.
#[derive(Debug, Clone)]
pub struct BuilderError {
    message: String,
}

impl Display for BuilderError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Copy, Clone, Default)]
pub enum DurabilityLevel {
    #[default]
    None = 0x00,
    Majority = 0x01,
    MajorityAndPersistOnMaster = 0x02,
    PersistToMajority = 0x03,
}

impl Display for DurabilityLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let alias = match *self {
            DurabilityLevel::None => "none",
            DurabilityLevel::Majority => "majority",
            DurabilityLevel::MajorityAndPersistOnMaster => "majorityAndPersistActive",
            DurabilityLevel::PersistToMajority => "persistToMajority",
        };

        write!(f, "{}", alias)
    }
}

impl TryFrom<&str> for DurabilityLevel {
    type Error = BuilderError;

    fn try_from(alias: &str) -> Result<Self, Self::Error> {
        match alias {
            "none" => Ok(DurabilityLevel::None),
            "majority" => Ok(DurabilityLevel::Majority),
            "majorityAndPersistActive" => Ok(DurabilityLevel::MajorityAndPersistOnMaster),
            "persistToMajority" => Ok(DurabilityLevel::PersistToMajority),
            _ => Err(BuilderError {
                message: "invalid durability level".to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BucketType {
    Couchbase,
    Memcached,
    Ephemeral,
}

impl TryFrom<&str> for BucketType {
    type Error = BuilderError;

    fn try_from(alias: &str) -> Result<Self, Self::Error> {
        match alias {
            "couchbase" => Ok(BucketType::Couchbase),
            "membase" => Ok(BucketType::Couchbase),
            "memcached" => Ok(BucketType::Memcached),
            "ephemeral" => Ok(BucketType::Ephemeral),
            _ => Err(BuilderError {
                message: "invalid bucket type".to_string(),
            }),
        }
    }
}

impl Display for BucketType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let alias = match *self {
            BucketType::Couchbase => "couchbase",
            BucketType::Memcached => "memcached",
            BucketType::Ephemeral => "ephemeral",
        };

        write!(f, "{}", alias)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConflictResolutionType {
    Timestamp,
    SequenceNumber,
}

impl TryFrom<&str> for ConflictResolutionType {
    type Error = BuilderError;

    fn try_from(alias: &str) -> Result<Self, Self::Error> {
        match alias {
            "lww" => Ok(ConflictResolutionType::Timestamp),
            "seqno" => Ok(ConflictResolutionType::SequenceNumber),
            _ => Err(BuilderError {
                message: "invalid conflict resolution policy".to_string(),
            }),
        }
    }
}

impl Display for ConflictResolutionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let alias = match *self {
            ConflictResolutionType::Timestamp => "lww",
            ConflictResolutionType::SequenceNumber => "seqno",
        };

        write!(f, "{}", alias)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EvictionPolicy {
    Full,
    ValueOnly,
    NotRecentlyUsed,
    NoEviction,
}
impl TryFrom<&str> for EvictionPolicy {
    type Error = BuilderError;

    fn try_from(alias: &str) -> Result<Self, Self::Error> {
        match alias {
            "fullEviction" => Ok(EvictionPolicy::Full),
            "valueOnly" => Ok(EvictionPolicy::ValueOnly),
            "nruEviction" => Ok(EvictionPolicy::NotRecentlyUsed),
            "noEviction" => Ok(EvictionPolicy::NoEviction),
            _ => Err(BuilderError {
                message: "invalid eviction policy".to_string(),
            }),
        }
    }
}

impl Display for EvictionPolicy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let alias = match *self {
            EvictionPolicy::Full => "fullEviction",
            EvictionPolicy::ValueOnly => "valueOnly",
            EvictionPolicy::NotRecentlyUsed => "nruEviction",
            EvictionPolicy::NoEviction => "noEviction",
        };

        write!(f, "{}", alias)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CompressionMode {
    Off,
    Passive,
    Active,
}

impl TryFrom<&str> for CompressionMode {
    type Error = BuilderError;

    fn try_from(alias: &str) -> Result<Self, Self::Error> {
        match alias {
            "off" => Ok(CompressionMode::Off),
            "passive" => Ok(CompressionMode::Passive),
            "active" => Ok(CompressionMode::Active),
            _ => Err(BuilderError {
                message: "invalid compression mode".to_string(),
            }),
        }
    }
}

impl Display for CompressionMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let alias = match *self {
            CompressionMode::Off => "off",
            CompressionMode::Passive => "passive",
            CompressionMode::Active => "active",
        };

        write!(f, "{}", alias)
    }
}

pub struct BucketSettingsBuilder {
    name: String,
    ram_quota_mb: u64,
    flush_enabled: bool,
    num_replicas: u32,
    bucket_type: BucketType,
    max_expiry: Duration,
    durability_level: DurabilityLevel,
}

impl BucketSettingsBuilder {
    pub fn new<S: Into<String>>(name: S) -> BucketSettingsBuilder {
        Self {
            name: name.into(),
            ram_quota_mb: 100,
            flush_enabled: false,
            num_replicas: 0,
            bucket_type: BucketType::Couchbase,
            max_expiry: Duration::from_secs(0),
            durability_level: DurabilityLevel::None,
        }
    }

    pub fn ram_quota_mb(mut self, ram_quota_mb: u64) -> BucketSettingsBuilder {
        self.ram_quota_mb = ram_quota_mb;
        self
    }

    pub fn flush_enabled(mut self, enabled: bool) -> BucketSettingsBuilder {
        self.flush_enabled = enabled;
        self
    }

    pub fn num_replicas(mut self, num_replicas: u32) -> BucketSettingsBuilder {
        self.num_replicas = num_replicas;
        self
    }

    pub fn bucket_type(mut self, bucket_type: BucketType) -> BucketSettingsBuilder {
        self.bucket_type = bucket_type;
        self
    }

    pub fn max_expiry(mut self, max_expiry: Duration) -> BucketSettingsBuilder {
        self.max_expiry = max_expiry;
        self
    }

    pub fn minimum_durability_level(
        mut self,
        durability_level: DurabilityLevel,
    ) -> BucketSettingsBuilder {
        self.durability_level = durability_level;
        self
    }

    pub fn build(self) -> BucketSettings {
        BucketSettings {
            name: self.name,
            ram_quota_mb: self.ram_quota_mb,
            flush_enabled: self.flush_enabled,
            num_replicas: self.num_replicas,
            bucket_type: self.bucket_type,
            max_expiry: self.max_expiry,
            durability_level: self.durability_level,
        }
    }
}

#[derive(Debug)]
pub(crate) struct BucketSettings {
    name: String,
    ram_quota_mb: u64,
    flush_enabled: bool,
    num_replicas: u32,
    bucket_type: BucketType,
    max_expiry: Duration,
    durability_level: DurabilityLevel,
}

#[derive(Debug, Deserialize)]
struct JSONControllers {
    #[serde(default)]
    flush: String,
}

#[derive(Debug, Deserialize)]
struct JSONQuota {
    #[serde(rename = "rawRAM")]
    raw_ram: u64,
}

#[derive(Debug, Deserialize)]
pub struct JSONBucketSettings {
    name: String,
    controllers: JSONControllers,
    quota: JSONQuota,
    #[serde(rename = "replicaNumber")]
    num_replicas: u32,
    #[serde(rename = "bucketType")]
    bucket_type: String,
    #[serde(rename = "maxTTL")]
    max_expiry: u32,
    #[serde(rename = "durabilityMinLevel", default)]
    durability_level: String,
}

impl TryFrom<JSONBucketSettings> for BucketSettings {
    type Error = BuilderError;

    fn try_from(settings: JSONBucketSettings) -> Result<Self, Self::Error> {
        Ok(BucketSettings {
            name: settings.name,
            ram_quota_mb: settings.quota.raw_ram / 1024 / 1024,
            flush_enabled: !settings.controllers.flush.is_empty(),
            num_replicas: settings.num_replicas,
            bucket_type: BucketType::try_from(settings.bucket_type.as_str())?,
            max_expiry: Duration::from_secs(settings.max_expiry as u64),
            durability_level: DurabilityLevel::try_from(settings.durability_level.as_str())?,
        })
    }
}

impl TryFrom<&Bucket> for BucketSettings {
    type Error = BuilderError;

    fn try_from(settings: &Bucket) -> Result<Self, Self::Error> {
        Ok(BucketSettings {
            name: settings.name(),
            ram_quota_mb: settings.ram_quota(),
            flush_enabled: settings.flush(),
            num_replicas: settings.replicas(),
            bucket_type: BucketType::try_from(settings.bucket_type().as_str())?,
            max_expiry: Duration::from_secs(settings.ttl_seconds()),
            durability_level: DurabilityLevel::try_from(settings.durability_level().as_str())?,
        })
    }
}

impl BucketSettings {
    pub fn validate(&self, is_capella: bool) -> Result<(), BuilderError> {
        if self.ram_quota_mb < 100 {
            return Err(BuilderError {
                message: "ram quota must be more than 100mb".to_string(),
            });
        }

        if let BucketType::Memcached = self.bucket_type {
            if is_capella {
                return Err(BuilderError {
                    message: "memcached buckets are not supported on Capella clusters".to_string(),
                });
            }
            if self.num_replicas > 0 {
                return Err(BuilderError {
                    message: "num replicas cannot be used with memcached buckets".to_string(),
                });
            }
        }

        Ok(())
    }

    pub fn as_json(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut json = serde_json::Map::new();
        json.insert("name".into(), self.name.clone().into());
        json.insert("memoryAllocationInMb".into(), self.ram_quota_mb.into());
        json.insert("flush".into(), self.flush_enabled.into());
        json.insert("type".into(), self.bucket_type.to_string().into());
        json.insert(
            "timeToLiveInSeconds".into(),
            self.max_expiry.as_secs().into(),
        );
        json.insert(
            "durabilityLevel".into(),
            self.durability_level.to_string().into(),
        );

        if self.num_replicas > 0 {
            json.insert("replicas".into(), self.num_replicas.into());
        };

        json
    }

    pub fn as_form(&self) -> Vec<(&str, String)> {
        let flush_enabled = match self.flush_enabled {
            true => "1",
            false => "0",
        };
        let mut form = vec![
            ("name", self.name.clone()),
            ("ramQuotaMB", self.ram_quota_mb.to_string()),
            ("flushEnabled", flush_enabled.into()),
            ("bucketType", self.bucket_type.to_string()),
        ];

        form.push(("durabilityMinLevel", self.durability_level.to_string()));

        if self.max_expiry.as_secs() > 0 {
            form.push(("maxTTL", self.max_expiry.as_secs().to_string()));
        }

        if self.num_replicas > 0 {
            form.push(("replicaNumber", self.num_replicas.to_string()));
        }

        form
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ram_quota_mb(&self) -> u64 {
        self.ram_quota_mb
    }

    pub fn flush_enabled(&self) -> bool {
        self.flush_enabled
    }

    pub fn num_replicas(&self) -> u32 {
        self.num_replicas
    }

    pub fn bucket_type(&self) -> BucketType {
        self.bucket_type
    }

    pub fn minimum_durability_level(&self) -> DurabilityLevel {
        self.durability_level
    }

    pub fn max_expiry(&self) -> i64 {
        self.max_expiry.as_secs() as i64
    }

    pub fn set_ram_quota_mb(&mut self, ram_quota_mb: u64) {
        self.ram_quota_mb = ram_quota_mb;
    }

    pub fn set_flush_enabled(&mut self, enabled: bool) {
        self.flush_enabled = enabled;
    }

    pub fn set_num_replicas(&mut self, num_replicas: u32) {
        self.num_replicas = num_replicas;
    }

    pub fn set_max_expiry(&mut self, max_expiry: Duration) {
        self.max_expiry = max_expiry;
    }

    pub fn set_minimum_durability_level(&mut self, durability_level: DurabilityLevel) {
        self.durability_level = durability_level;
    }
}
