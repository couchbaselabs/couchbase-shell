use crate::common::playground::CBPlayground;
use nu_test_support::pipeline;

mod common;

#[test]
fn collection_management() {
    CBPlayground::setup("collection_management", None, None, |dirs, sandbox| {
        // Create a new collection without the scope flag to check that we default to the _default
        // scope
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"collections create test_collection"#));
        assert_eq!("", out.err);

        let out = cbsh!(cwd: dirs.test(), pipeline(r#"collections | where collection == "test_collection" | first | to json"#));
        assert_eq!("", out.err);
        let json = sandbox.parse_out_to_json(out.out).unwrap();

        // Since no max_expiry was given we should inherit it
        assert_eq!(json["max_expiry"], "inherited");

        // Drop the collection, again checking that we default to the _default scope
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"collections drop test_collection"#));
        assert_eq!("", out.err);

        // Check that test_collection has been dropped
        let out = cbsh!(cwd: dirs.test(), pipeline(r#""test_collection" in (collections --scope _default| get collection)"#));
        assert_eq!("false", out.out);

        // Create a new collection with max expiry manually set
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"collections create test_collection --max-expiry 100"#));
        assert_eq!("", out.err);

        let out = cbsh!(cwd: dirs.test(), pipeline(r#"collections | where collection == "test_collection" | first | to json"#));
        assert_eq!("", out.err);
        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!(json["max_expiry"], "100s");

        let out = cbsh!(cwd: dirs.test(), pipeline(r#"collections drop test_collection"#));
        assert_eq!("", out.err);
    });
}
