mod common;

use crate::common::{playground::CBPlayground, support};

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn import_a_document() {
    CBPlayground::setup("import_a_document", None, None, |dirs, sandbox| {
        let content = r#"{id: 123, foo: bar, fizz: buzz}"#;
        let out =
            cbsh!(cwd: dirs.test(),  support::cb_pipeline(format!("{} | save foo.json", content)));
        assert_eq!("", out.err);

        let out =
            cbsh!(cwd: dirs.test(), support::cb_pipeline("doc import foo.json | first | to json"));
        assert_eq!("", out.err);

        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!(1, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(0, json["failed"]);
        assert_eq!("", json["failures"]);
    });
}

#[test]
#[cfg_attr(not(feature = "key_value"), ignore)]
fn import_missing_id() {
    CBPlayground::setup("import_missing_id", None, None, |dirs, sandbox| {
        let content = r#"{foo: bar, fizz: buzz}"#;
        let out =
            cbsh!(cwd: dirs.test(),  support::cb_pipeline(format!("{} | save bar.json", content)));
        assert_eq!("", out.err);

        let out =
            cbsh!(cwd: dirs.test(), support::cb_pipeline("doc import bar.json | first | to json"));
        assert_eq!("", out.err);

        let json = sandbox.parse_out_to_json(out.out).unwrap();
        assert_eq!(0, json["success"]);
        assert_eq!(1, json["processed"]);
        assert_eq!(1, json["failed"]);
        assert_eq!("Missing doc id", json["failures"]);
    });
}
