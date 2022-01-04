use crate::cli::cloud_json::{
    JSONCloudClustersSummaries, JSONCloudClustersSummariesV3, JSONCloudsProjectsResponse,
    JSONCloudsResponse,
};
use crate::client::{CapellaClient, CapellaRequest};
use crate::state::{RemoteCluster, State};
use nu_engine::CommandArgs;
use nu_errors::{CoerceInto, ShellError};
use nu_protocol::{Primitive, TaggedDictBuilder, UnspannedPathMember, UntaggedValue, Value};
use nu_source::{Tag, TaggedItem};
use num_traits::cast::ToPrimitive;
use regex::Regex;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::Instant;

pub fn convert_row_to_nu_value(
    v: &serde_json::Value,
    tag: impl Into<Tag>,
    cluster_identifier: String,
) -> Result<Value, ShellError> {
    let tag = tag.into();

    match v {
        serde_json::Value::Object(o) => {
            let mut collected = TaggedDictBuilder::new(&tag);
            for (k, v) in o.iter() {
                collected.insert_value(k.clone(), convert_json_value_to_nu_value(v, &tag)?);
            }
            collected.insert_value("cluster", cluster_identifier);

            Ok(collected.into_value())
        }
        _ => Err(ShellError::unexpected(
            "row not an object - malformed response",
        )),
    }
}

pub fn convert_json_value_to_nu_value(
    v: &serde_json::Value,
    tag: impl Into<Tag>,
) -> Result<Value, ShellError> {
    let tag = tag.into();
    let span = tag.span;

    let result = match v {
        serde_json::Value::Null => UntaggedValue::Primitive(Primitive::Nothing).into_value(&tag),
        serde_json::Value::Bool(b) => UntaggedValue::boolean(*b).into_value(&tag),
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                if let Some(nas) = n.as_i64() {
                    UntaggedValue::int(nas).into_value(&tag)
                } else {
                    return Err(ShellError::unexpected(format!(
                        "Could not get value as number {}",
                        v
                    )));
                }
            } else if let Some(nas) = n.as_f64() {
                UntaggedValue::decimal_from_float(nas, span).into_value(&tag)
            } else {
                return Err(ShellError::unexpected(format!(
                    "Could not get value as number {}",
                    v
                )));
            }
        }
        serde_json::Value::String(s) => {
            UntaggedValue::Primitive(Primitive::String(String::from(s))).into_value(&tag)
        }
        serde_json::Value::Array(a) => {
            let t = a
                .iter()
                .map(|x| convert_json_value_to_nu_value(x, &tag).ok())
                .flatten()
                .collect();
            UntaggedValue::Table(t).into_value(tag)
        }
        serde_json::Value::Object(o) => {
            let mut collected = TaggedDictBuilder::new(&tag);
            for (k, v) in o.iter() {
                collected.insert_value(k.clone(), convert_json_value_to_nu_value(v, &tag)?);
            }

            collected.into_value()
        }
    };

    Ok(result)
}

// Adapted from https://github.com/nushell/nushell/blob/main/crates/nu-command/src/commands/formats/to/json.rs
pub fn convert_nu_value_to_json_value(v: &Value) -> Result<serde_json::Value, ShellError> {
    Ok(match &v.value {
        UntaggedValue::Primitive(Primitive::Boolean(b)) => serde_json::Value::Bool(*b),
        UntaggedValue::Primitive(Primitive::Filesize(b)) => serde_json::Value::Number(
            serde_json::Number::from(b.to_u64().expect("What about really big numbers")),
        ),
        UntaggedValue::Primitive(Primitive::Duration(i)) => {
            serde_json::Value::String(i.to_string())
        }
        UntaggedValue::Primitive(Primitive::Date(d)) => serde_json::Value::String(d.to_string()),
        UntaggedValue::Primitive(Primitive::EndOfStream) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::BeginningOfStream) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::Decimal(f)) => {
            if let Some(f) = f.to_f64() {
                if let Some(num) = serde_json::Number::from_f64(
                    f.to_f64().expect("TODO: What about really big decimals?"),
                ) {
                    serde_json::Value::Number(num)
                } else {
                    return Err(ShellError::labeled_error(
                        "Could not convert value to decimal number",
                        "could not convert to decimal",
                        &v.tag,
                    ));
                }
            } else {
                return Err(ShellError::labeled_error(
                    "Could not convert value to decimal number",
                    "could not convert to decimal",
                    &v.tag,
                ));
            }
        }

        UntaggedValue::Primitive(Primitive::Int(i)) => {
            serde_json::Value::Number(serde_json::Number::from(*i))
        }
        UntaggedValue::Primitive(Primitive::BigInt(i)) => {
            serde_json::Value::Number(serde_json::Number::from(CoerceInto::<i64>::coerce_into(
                i.tagged(&v.tag),
                "converting to JSON number",
            )?))
        }
        UntaggedValue::Primitive(Primitive::Nothing) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::GlobPattern(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::String(s)) => serde_json::Value::String(s.clone()),
        UntaggedValue::Primitive(Primitive::ColumnPath(path)) => serde_json::Value::Array(
            path.iter()
                .map(|x| match &x.unspanned {
                    UnspannedPathMember::String(string) => {
                        Ok(serde_json::Value::String(string.clone()))
                    }
                    UnspannedPathMember::Int(int) => {
                        Ok(serde_json::Value::Number(serde_json::Number::from(*int)))
                    }
                })
                .collect::<Result<Vec<serde_json::Value>, ShellError>>()?,
        ),
        UntaggedValue::Primitive(Primitive::FilePath(s)) => {
            serde_json::Value::String(s.display().to_string())
        }

        UntaggedValue::Table(l) => serde_json::Value::Array(json_list(l)?),
        UntaggedValue::Error(e) => return Err(e.clone()),
        UntaggedValue::Block(_) | UntaggedValue::Primitive(Primitive::Range(_)) => {
            serde_json::Value::Null
        }
        UntaggedValue::DataFrame(_) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::Binary(b)) => serde_json::Value::Array(
            b.iter()
                .map(|x| {
                    serde_json::Number::from_f64(*x as f64).ok_or_else(|| {
                        ShellError::labeled_error(
                            "Can not convert number from floating point",
                            "can not convert to number",
                            &v.tag,
                        )
                    })
                })
                .collect::<Result<Vec<serde_json::Number>, ShellError>>()?
                .into_iter()
                .map(serde_json::Value::Number)
                .collect(),
        ),
        UntaggedValue::Row(o) => {
            let mut m = serde_json::Map::new();
            for (k, v) in o.entries.iter() {
                m.insert(k.clone(), convert_nu_value_to_json_value(v)?);
            }
            serde_json::Value::Object(m)
        }
        UntaggedValue::FrameStruct(_) => serde_json::Value::Null,
    })
}

fn json_list(input: &[Value]) -> Result<Vec<serde_json::Value>, ShellError> {
    let mut out = vec![];

    for value in input {
        out.push(convert_nu_value_to_json_value(value)?);
    }

    Ok(out)
}

pub fn cluster_identifiers_from(
    state: &Arc<Mutex<State>>,
    args: &CommandArgs,
    default_active: bool,
) -> Result<Vec<String>, ShellError> {
    let state = state.lock().unwrap();
    let identifier_arg: String = match args.get_flag("clusters")? {
        Some(arg) => arg,
        None => {
            if default_active {
                return Ok(vec![state.active()]);
            }
            "".into()
        }
    };

    let re = match Regex::new(identifier_arg.as_str()) {
        Ok(v) => v,
        Err(e) => {
            return Err(ShellError::unexpected(format!(
                "Could not parse regex {}",
                e
            )))
        }
    };
    let clusters: Vec<String> = state
        .clusters()
        .keys()
        .filter(|k| re.is_match(k))
        .cloned()
        .collect();
    if clusters.is_empty() {
        return Err(ShellError::unexpected("Cluster not found"));
    }

    Ok(clusters)
}

pub fn namespace_from_args(
    bucket_flag: Option<String>,
    scope_flag: Option<String>,
    collection_flag: Option<String>,
    active_cluster: &RemoteCluster,
) -> Result<(String, String, String), ShellError> {
    let bucket = match bucket_flag.or_else(|| active_cluster.active_bucket()) {
        Some(v) => Ok(v),
        None => Err(ShellError::unexpected(
            "Could not auto-select a bucket - please use --bucket instead".to_string(),
        )),
    }?;

    let scope = match scope_flag {
        Some(s) => s,
        None => match active_cluster.active_scope() {
            Some(s) => s,
            None => "".into(),
        },
    };

    let collection = match collection_flag {
        Some(c) => c,
        None => match active_cluster.active_collection() {
            Some(c) => c,
            None => "".into(),
        },
    };

    Ok((bucket, scope, collection))
}

pub fn validate_is_cloud(cluster: &RemoteCluster, err_msg: &str) -> Result<(), ShellError> {
    if cluster.capella_org().is_none() {
        return Err(ShellError::unexpected(err_msg));
    }

    Ok(())
}

pub fn validate_is_not_cloud(cluster: &RemoteCluster, err_msg: &str) -> Result<(), ShellError> {
    if cluster.capella_org().is_some() {
        return Err(ShellError::unexpected(err_msg));
    }

    Ok(())
}

pub(crate) fn find_project_id(
    ctrl_c: Arc<AtomicBool>,
    name: String,
    client: &Arc<CapellaClient>,
    deadline: Instant,
) -> Result<String, ShellError> {
    let response = client.capella_request(CapellaRequest::GetProjects {}, deadline, ctrl_c)?;
    if response.status() != 200 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };
    let content: JSONCloudsProjectsResponse = serde_json::from_str(response.content())?;

    for p in content.items() {
        if p.name() == name.clone() {
            return Ok(p.id().to_string());
        }
    }

    Err(ShellError::unexpected("Project could not be found"))
}

pub(crate) fn find_cloud_id(
    ctrl_c: Arc<AtomicBool>,
    name: String,
    client: &Arc<CapellaClient>,
    deadline: Instant,
) -> Result<String, ShellError> {
    let response = client.capella_request(CapellaRequest::GetClouds {}, deadline, ctrl_c)?;
    if response.status() != 200 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };
    let clouds: JSONCloudsResponse = serde_json::from_str(response.content())?;

    for c in clouds.items() {
        if c.name() == name {
            return Ok(c.id().to_string());
        }
    }

    Err(ShellError::unexpected("Cloud could not be found"))
}

pub(crate) fn find_capella_cluster_id_hosted(
    ctrl_c: Arc<AtomicBool>,
    name: String,
    client: &Arc<CapellaClient>,
    deadline: Instant,
) -> Result<String, ShellError> {
    let response = client.capella_request(CapellaRequest::GetClustersV3 {}, deadline, ctrl_c)?;
    if response.status() != 200 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };
    let content: JSONCloudClustersSummariesV3 = serde_json::from_str(response.content())?;

    for c in content.items() {
        if c.name() == name {
            return Ok(c.id().to_string());
        }
    }

    Err(ShellError::unexpected("Cluster could not be found"))
}

pub(crate) fn find_capella_cluster_id_vpc(
    ctrl_c: Arc<AtomicBool>,
    name: String,
    client: &Arc<CapellaClient>,
    deadline: Instant,
) -> Result<String, ShellError> {
    let response = client.capella_request(CapellaRequest::GetClusters {}, deadline, ctrl_c)?;
    if response.status() != 200 {
        return Err(ShellError::unexpected(response.content().to_string()));
    };
    let content: JSONCloudClustersSummaries = serde_json::from_str(response.content())?;

    for c in content.items() {
        if c.name() == name {
            return Ok(c.id().to_string());
        }
    }

    Err(ShellError::unexpected("Cluster could not be found"))
}

// duration_to_golang_string creates a golang formatted string to use with timeouts. Unlike Golang
// strings it does not deal with fracational seconds, we do not need that accuracy.
pub fn duration_to_golang_string(duration: Duration) -> String {
    let mut total_secs = duration.as_secs();
    let secs = total_secs % 60;
    total_secs = total_secs / 60;
    let mut golang_string = format!("{}s", secs);
    if total_secs > 0 {
        let minutes = total_secs % 60;
        total_secs = total_secs / 60;
        golang_string = format!("{}m{}", minutes, golang_string);
        if total_secs > 0 {
            golang_string = format!("{}h{}", total_secs, golang_string)
        }
    }

    golang_string
}

#[cfg(test)]
mod tests {
    use crate::cli::util::duration_to_golang_string;
    use std::time::Duration;

    #[test]
    fn duration_to_golang_string_some_seconds() {
        assert_eq!(
            "2s".to_string(),
            duration_to_golang_string(Duration::from_secs(2))
        );
    }

    #[test]
    fn duration_to_golang_string_some_seconds_and_minutes() {
        assert_eq!(
            "5m2s".to_string(),
            duration_to_golang_string(Duration::from_secs(302))
        );
    }

    #[test]
    fn duration_to_golang_string_some_seconds_and_minutes_and_hours() {
        assert_eq!(
            "1h5m2s".to_string(),
            duration_to_golang_string(Duration::from_secs(3902))
        );
    }
}
