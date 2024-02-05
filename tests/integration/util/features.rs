use strum_macros::EnumIter;

#[derive(Debug, Clone, Copy, Eq, PartialEq, EnumIter)]
pub enum TestFeature {
    KeyValue,
    Query,
    Collections,
    QueryIndex,
    QueryIndexDefinitions,
    QueryIndexAdvise,
    Capella,
}

impl From<&str> for TestFeature {
    fn from(feature: &str) -> Self {
        match feature {
            "kv" => TestFeature::KeyValue,
            "query" => TestFeature::Query,
            "collections" => TestFeature::Collections,
            "queryindex" => TestFeature::QueryIndex,
            "queryindexdefs" => TestFeature::QueryIndexDefinitions,
            "queryindexadvise" => TestFeature::QueryIndexAdvise,
            "capella" => TestFeature::Capella,
            _ => panic!("Unrecognized feature : {}", feature),
        }
    }
}
