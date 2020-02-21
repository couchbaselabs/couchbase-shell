use couchbase::{Cluster, QueryOptions};
use futures::executor::block_on;
use futures::stream::StreamExt;
use nu::{CommandArgs, CommandRegistry, OutputStream};
use nu_errors::ShellError;
use std::sync::Arc;
use log::debug;
use nu_protocol::{SyntaxShape, Primitive, Signature, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;

pub struct Query {
    cluster: Arc<Cluster>,
}

impl Query {
    pub fn new(cluster: Arc<Cluster>) -> Self {
        Self { cluster }
    }
}

impl nu::WholeStreamCommand for Query {
    fn name(&self) -> &str {
        "query"
    }

    fn signature(&self) -> Signature {
        Signature::build("query").required(
            "statement",
            SyntaxShape::String,
            "the n1ql query statement",
        )
    }

    fn usage(&self) -> &str {
        "Performs a N1QL query"
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        let args = args.evaluate_once(registry)?;
        let statement = args.nth(0).expect("need statement").as_string()?;

        debug!("Running N1QL Query {}", &statement);
        let mut result =
            block_on(self.cluster.query(statement, QueryOptions::default())).unwrap();
        let stream = result.rows::<serde_json::Value>().map(|v| {
            convert_json_value_to_nu_value(&v.unwrap(), Tag::default())
        });
        Ok(OutputStream::from_input(stream))
    }
}

fn convert_json_value_to_nu_value(v: &serde_json::Value, tag: impl Into<Tag>) -> Value {
    let tag = tag.into();

    match v {
        serde_json::Value::Null => UntaggedValue::Primitive(Primitive::Nothing).into_value(&tag),
        serde_json::Value::Bool(b) => UntaggedValue::boolean(*b).into_value(&tag),
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                UntaggedValue::int(n.as_i64().unwrap()).into_value(&tag)
            } else {
                UntaggedValue::decimal(n.as_f64().unwrap()).into_value(&tag)
            }
        },
        serde_json::Value::String(s) => {
            UntaggedValue::Primitive(Primitive::String(String::from(s))).into_value(&tag)
        }
        serde_json::Value::Array(a) => UntaggedValue::Table(
            a.iter()
                .map(|x| convert_json_value_to_nu_value(x, &tag))
                .collect(),
        )
        .into_value(tag),
        serde_json::Value::Object(o) => {
            let mut collected = TaggedDictBuilder::new(&tag);
            for (k, v) in o.iter() {
                collected.insert_value(k.clone(), convert_json_value_to_nu_value(v, &tag));
            }

            collected.into_value()
        }
    }
}