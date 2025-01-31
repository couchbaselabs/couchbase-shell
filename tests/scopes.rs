use crate::common::playground::CBPlayground;
use nu_test_support::pipeline;
use std::thread;

mod common;

#[test]
fn scope_management() {
    CBPlayground::setup("scope_management", None, None, |dirs, _| {
        let out = cbsh!(cwd: dirs.test(), pipeline(r#"scopes create test_scope"#));
        assert_eq!("", out.err);

        let out = cbsh!(cwd: dirs.test(), pipeline(r#""test_scope" in (scopes | get scope)"#));
        assert_eq!("", out.err);
        assert_eq!("true", out.out);

        let out = cbsh!(cwd: dirs.test(), pipeline(r#"scopes drop test_scope"#));
        assert_eq!("", out.err);

        let out = cbsh!(cwd: dirs.test(), pipeline(r#""test_scope" not-in (scopes | get scope)"#));
        assert_eq!("", out.err);
        assert_eq!("true", out.out);
    });
}
