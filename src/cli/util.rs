use nu_protocol::{Primitive, TaggedDictBuilder, UntaggedValue, Value};
use nu_source::Tag;

pub fn convert_json_value_to_nu_value(v: &serde_json::Value, tag: impl Into<Tag>) -> Value {
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
        }
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
