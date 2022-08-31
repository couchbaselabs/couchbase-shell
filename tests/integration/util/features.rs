use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, Eq, PartialEq, EnumIter)]
pub enum TestFeature {
    KeyValue,
    Query,
    Collections,
}

impl From<&str> for TestFeature {
    fn from(feature: &str) -> Self {
        match feature {
            "kv" => TestFeature::KeyValue,
            "query" => TestFeature::Query,
            "collections" => TestFeature::Collections,
            _ => panic!("Unrecognized feature : {}", feature),
        }
    }
}
