mod common;

use crate::common::playground::CBPlayground;
use nu_test_support::pipeline;

#[test]
#[cfg_attr(not(feature = "vector"), ignore)]
fn create_vector_index() {
    CBPlayground::setup("create vector index", None, None, |dirs, _sandbox| {
        let out =
            cbsh!(cwd: dirs.test(), pipeline(r#"vector create-index test-index test-field 1024"#));
        assert_eq!("", out.err);
    });
}
