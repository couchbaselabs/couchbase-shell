use crate::state::{RemoteCluster, State};
use futures::{future::FutureExt, pin_mut, select, Stream, StreamExt};
use nu_cli::{InterruptibleStream, ToPrimitive};
use nu_engine::EvaluatedWholeStreamCommandArgs;
use nu_errors::ShellError;
use nu_protocol::{
    Primitive, ReturnSuccess, TaggedDictBuilder, UnspannedPathMember, UntaggedValue, Value,
};
use nu_source::Tag;
use nu_stream::OutputStream;
use regex::Regex;
use std::fs::File;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

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
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Could not get value as number {}",
                        v
                    )));
                }
            } else {
                if let Some(nas) = n.as_f64() {
                    UntaggedValue::decimal_from_float(nas, span).into_value(&tag)
                } else {
                    return Err(ShellError::untagged_runtime_error(format!(
                        "Could not get value as number {}",
                        v
                    )));
                }
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

// Adapted from https://github.com/nushell/nushell/blob/master/crates/nu-cli/src/commands/to_json.rs
pub fn convert_nu_value_to_json_value(v: &Value) -> Result<serde_json::Value, ShellError> {
    Ok(match &v.value {
        UntaggedValue::Primitive(Primitive::Boolean(b)) => serde_json::Value::Bool(*b),
        UntaggedValue::Primitive(Primitive::Filesize(b)) => serde_json::Value::Number(
            serde_json::Number::from(b.to_u64().expect("what about really big numbers?")),
        ),
        UntaggedValue::Primitive(Primitive::Duration(i)) => {
            serde_json::Value::String(i.to_string())
        }
        UntaggedValue::Primitive(Primitive::Date(d)) => serde_json::Value::String(d.to_string()),
        UntaggedValue::Primitive(Primitive::EndOfStream) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::BeginningOfStream) => serde_json::Value::Null,
        UntaggedValue::Primitive(Primitive::Decimal(f)) => {
            if let Some(f) = f.to_f32() {
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
            if let Some(ias) = i.to_i64() {
                serde_json::Value::Number(serde_json::Number::from(ias))
            } else {
                return Err(ShellError::untagged_runtime_error(format!(
                    "Could not get value as number {}",
                    i
                )));
            }
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
                        if let Some(ias) = int.to_i64() {
                            Ok(serde_json::Value::Number(serde_json::Number::from(ias)))
                        } else {
                            return Err(ShellError::untagged_runtime_error(format!(
                                "Could not get value as number {}",
                                int
                            )));
                        }
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
    state: &Arc<State>,
    args: &EvaluatedWholeStreamCommandArgs,
    default_active: bool,
) -> Result<Vec<String>, ShellError> {
    let identifier_arg = match args
        .call_info
        .args
        .get("clusters")
        .map(|id| id.as_string().ok())
        .flatten()
    {
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
            return Err(ShellError::untagged_runtime_error(format!(
                "Could not parse regex {}",
                e
            )))
        }
    };
    Ok(state
        .clusters()
        .keys()
        .filter(|k| re.is_match(k))
        .map(|v| v.clone())
        .collect())
}

pub fn cbsh_home_path() -> Result<PathBuf, ShellError> {
    let mut path = match dirs::home_dir() {
        Some(f) => f,
        None => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Couldn't find home_dir",
            )))
        }
    };

    path.push(".cbsh");

    Ok(path)
}

pub fn read_file_from_home(filename: String) -> Result<File, ShellError> {
    let mut path = cbsh_home_path()?;
    path.push(filename);
    match File::open(path) {
        Ok(f) => return Ok(f),
        Err(e) => {
            return Err(ShellError::untagged_runtime_error(format!(
                "Failed to open file {}",
                e
            )))
        }
    };
}
