use crate::features::TestFeature;
use crate::playground::{CBPlayground, PerTestOptions};
use crate::tests::query::create_index;
use crate::{cbsh, ClusterUnderTest, ConfigAware, TestResult};
use nu_test_support::pipeline;
use serde_json::Value;
use std::collections::HashMap;
use std::ops::Add;
use std::path::PathBuf;
use std::sync::Arc;
use std::time;
use std::time::Instant;

#[derive(Debug, Clone)]
struct Index {
    name: String,
    bucket: String,
    scope: String,
    keyspace: String,
    condition: Value,
    fields: Vec<String>,
    primary: bool,
    state: String,
    index_type: String,
}

fn get_indexes(
    base_cmd: String,
    cwd: &PathBuf,
    index_names: Vec<String>,
    sandbox: &mut CBPlayground,
    flags: impl Into<String>,
) -> HashMap<String, Value> {
    let mut indexes: HashMap<String, Value> = HashMap::new();
    let flags = flags.into();
    CBPlayground::retry_until(
        Instant::now().add(time::Duration::from_secs(30)),
        time::Duration::from_millis(200),
        || -> TestResult<bool> {
            let out = cbsh!(
                cwd,
                pipeline(
                    format!("{} query indexes {} | to json", base_cmd, flags.clone()).as_str()
                )
            );

            if out.err != "" {
                println!("Received error from query: {}", out.err);
                return Ok(false);
            }

            let json = sandbox.parse_out_to_json(out.out)?;

            match json.as_array() {
                Some(arr) => {
                    indexes.drain();
                    for item in arr {
                        let name = item["name"].as_str().unwrap().to_string();
                        if index_names.contains(&name) {
                            indexes.insert(name, item.clone());
                        }
                    }
                    if indexes.len() != index_names.len() {
                        println!(
                            "Expected {} indexes but was: {}",
                            index_names.len(),
                            indexes.len()
                        );
                        return Ok(false);
                    }
                    Ok(true)
                }
                None => {
                    println!("Response from query not an array: {}", json);
                    Ok(false)
                }
            }
        },
    );

    indexes
}

fn assert_index(index: Index, actual: &Value) {
    let bucket = if index.bucket.is_empty() {
        Value::Null
    } else {
        Value::from(index.bucket)
    };
    let scope = if index.scope.is_empty() {
        Value::Null
    } else {
        Value::from(index.scope)
    };
    assert_eq!(bucket, actual["bucket"]);
    assert_eq!(scope, actual["scope"]);
    assert_eq!(index.keyspace, actual["keyspace"]);
    assert_eq!(index.condition, actual["condition"]);
    let mut keys1 = vec![];
    for key in actual["index_key"].as_array().unwrap() {
        keys1.push(key.as_str().unwrap())
    }
    assert_eq!(index.fields, keys1);
    assert_eq!(index.name, actual["name"]);
    assert_eq!(index.primary, actual["primary"]);
    assert_eq!(index.state, actual["state"]);
    assert_eq!(index.index_type, actual["type"]);
}

pub async fn test_should_get_indexes_with_context(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::QueryIndex)
        || !config.supports_feature(TestFeature::Collections)
    {
        return true;
    }

    CBPlayground::setup(
        "test_should_get_indexes_with_context",
        cluster.config(),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            let (cmd, keyspace) = if let Some(s) = config.scope() {
                (
                    format!("cb-env scope \"{}\" |", s),
                    config.collection().unwrap(),
                )
            } else {
                ("".to_string(), config.bucket())
            };

            let fields = "`field1`,`field2`";
            let mut index_name1 = String::new();
            let mut index_name2 = String::new();
            CBPlayground::retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                || -> TestResult<bool> {
                    index_name1 = create_index(cmd.clone(), fields, keyspace.clone(), dirs.test())?;
                    index_name2 = create_index(cmd.clone(), fields, keyspace.clone(), dirs.test())?;
                    Ok(true)
                },
            );

            let indexes = get_indexes(
                cmd,
                dirs.test(),
                vec![index_name1.clone(), index_name2.clone()],
                sandbox,
                "",
            );
            assert_index(
                Index {
                    bucket: config.bucket(),
                    scope: config.scope().unwrap_or("".to_string()),
                    name: index_name1.clone(),
                    keyspace: keyspace.clone(),
                    condition: Value::Null,
                    fields: fields
                        .split(',')
                        .map(|f| f.to_string())
                        .collect::<Vec<String>>(),
                    primary: false,
                    state: "online".to_string(),
                    index_type: "gsi".to_string(),
                },
                indexes.get(&index_name1).unwrap(),
            );
            assert_index(
                Index {
                    bucket: config.bucket(),
                    scope: config.scope().unwrap_or("".to_string()),
                    name: index_name2.clone(),
                    keyspace,
                    condition: Value::Null,
                    fields: fields
                        .split(',')
                        .map(|f| f.to_string())
                        .collect::<Vec<String>>(),
                    primary: false,
                    state: "online".to_string(),
                    index_type: "gsi".to_string(),
                },
                indexes.get(&index_name2).unwrap(),
            );
        },
    );

    false
}

pub async fn test_should_get_indexes(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::QueryIndex) {
        return true;
    }

    CBPlayground::setup(
        "test_should_get_indexes",
        cluster.config(),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            let keyspace = config.bucket();
            let cmd = "".to_string();
            let fields = "`field1`,`field2`";
            let mut index_name1 = String::new();
            let mut index_name2 = String::new();
            CBPlayground::retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                || -> TestResult<bool> {
                    index_name1 = create_index(cmd.clone(), fields, keyspace.clone(), dirs.test())?;
                    index_name2 = create_index(cmd.clone(), fields, keyspace.clone(), dirs.test())?;
                    Ok(true)
                },
            );

            let indexes = get_indexes(
                cmd,
                dirs.test(),
                vec![index_name1.clone(), index_name2.clone()],
                sandbox,
                "",
            );
            assert_index(
                Index {
                    bucket: "".to_string(),
                    scope: "".to_string(),
                    name: index_name1.clone(),
                    keyspace: keyspace.clone(),
                    condition: Value::Null,
                    fields: fields
                        .split(',')
                        .map(|f| f.to_string())
                        .collect::<Vec<String>>(),
                    primary: false,
                    state: "online".to_string(),
                    index_type: "gsi".to_string(),
                },
                indexes.get(&index_name1).unwrap(),
            );
            assert_index(
                Index {
                    bucket: "".to_string(),
                    scope: "".to_string(),
                    name: index_name2.clone(),
                    keyspace,
                    condition: Value::Null,
                    fields: fields
                        .split(',')
                        .map(|f| f.to_string())
                        .collect::<Vec<String>>(),
                    primary: false,
                    state: "online".to_string(),
                    index_type: "gsi".to_string(),
                },
                indexes.get(&index_name2).unwrap(),
            );
        },
    );

    false
}

pub async fn test_should_get_index_definitions(cluster: Arc<ClusterUnderTest>) -> bool {
    let config = cluster.config();
    if !config.supports_feature(TestFeature::QueryIndexDefinitions) {
        return true;
    }

    CBPlayground::setup(
        "test_should_get_index_definitions",
        cluster.config(),
        PerTestOptions::default().set_no_default_collection(true),
        |dirs, sandbox| {
            let keyspace = config.bucket();
            let cmd = "".to_string();
            let fields = "`field1`,`field2`";
            let mut index_name = String::new();
            CBPlayground::retry_until(
                Instant::now().add(time::Duration::from_secs(30)),
                time::Duration::from_millis(200),
                || -> TestResult<bool> {
                    index_name = create_index(cmd.clone(), fields, keyspace.clone(), dirs.test())?;
                    Ok(true)
                },
            );

            let indexes = get_indexes(
                cmd,
                dirs.test(),
                vec![index_name.clone()],
                sandbox,
                "--definitions",
            );
            let actual = indexes.get(&index_name).unwrap();
            assert_eq!(index_name, actual["name"]);
            assert_eq!(keyspace, actual["bucket"]);
            assert_eq!("_default", actual["scope"]);
            assert_eq!("_default", actual["collection"]);
            // Status could be ready or building so just ensure that it's something.
            assert_ne!("", actual["status"]);
            assert_eq!("plasma", actual["storage_mode"]);
            assert_ne!("", actual["definition"]);
            assert_ne!(Value::Null, actual["replicas"]);
        },
    );

    false
}
