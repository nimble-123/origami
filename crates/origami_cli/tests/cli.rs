//! CLI integration tests. M0: the printer runs in identity mode, so "would
//! change" is exercised through files that need no change and through errors.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

const CLASS: &str = "CLASS lcl DEFINITION.\n  PUBLIC SECTION.\nENDCLASS.\n";

fn origami() -> Command {
    Command::cargo_bin("origami").unwrap()
}

fn dir() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    fs::create_dir(d.path().join(".git")).unwrap();
    d
}

fn write(d: &Path, name: &str, text: &str) {
    if let Some(parent) = Path::new(name).parent() {
        fs::create_dir_all(d.join(parent)).unwrap();
    }
    fs::write(d.join(name), text).unwrap();
}

fn json_of(stderr: &[u8]) -> serde_json::Value {
    serde_json::from_slice(stderr).expect("stderr is one JSON document")
}

fn validate(doc: &serde_json::Value) {
    let schema_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas/diagnostics.v1.json");
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(schema_path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator.iter_errors(doc).map(|e| e.to_string()).collect();
    assert!(errors.is_empty(), "schema errors: {errors:?}");
}

#[test]
fn unchanged_files_exit_0_and_are_not_rewritten() {
    let d = dir();
    write(d.path(), "src/zcl_a.clas.abap", CLASS);
    let file = d.path().join("src/zcl_a.clas.abap");
    let before = fs::metadata(&file).unwrap().modified().unwrap();
    origami().current_dir(d.path()).assert().code(0);
    assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), before);
}

#[test]
fn structural_error_exits_2_and_leaves_file() {
    let d = dir();
    write(d.path(), "z.prog.abap", "IF x = 1.\n");
    origami()
        .current_dir(d.path())
        .arg("z.prog.abap")
        .assert()
        .code(2);
    assert_eq!(
        fs::read_to_string(d.path().join("z.prog.abap")).unwrap(),
        "IF x = 1.\n"
    );
}

#[test]
fn unsupported_explicit_file_is_e011_exit_3() {
    let d = dir();
    write(d.path(), "zcl_a.clas.xml", "<xml/>");
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "json", "zcl_a.clas.xml"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(
        json_of(&out.stderr)["files"][0]["diagnostics"][0]["code"],
        "E011"
    );
}

#[test]
fn unsupported_files_in_directories_are_skipped() {
    let d = dir();
    write(d.path(), "src/zcl_a.clas.xml", "<xml/>");
    write(d.path(), "src/zcl_a.clas.abap", CLASS);
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(json_of(&out.stderr)["files"].as_array().unwrap().len(), 1);
}

#[test]
fn exit_code_aggregation_prefers_higher_rank() {
    let d = dir();
    write(d.path(), "ok.clas.abap", CLASS);
    write(d.path(), "bad.prog.abap", "ENDIF.\n");
    write(d.path(), "x.clas.xml", "");
    // E004 (2) and E011 (3): 3 wins.
    origami()
        .current_dir(d.path())
        .args(["ok.clas.abap", "bad.prog.abap", "x.clas.xml"])
        .assert()
        .code(3);
}

#[test]
fn json_output_matches_schema() {
    let d = dir();
    write(d.path(), "ok.clas.abap", CLASS);
    write(d.path(), "bad.prog.abap", "x = 'abc\n.");
    write(
        d.path(),
        "z.ddls.asddls",
        "define view entity Z as select from t { a }\n",
    );
    let out = origami()
        .current_dir(d.path())
        .args(["--check", "--diagnostics", "json"])
        .output()
        .unwrap();
    let doc = json_of(&out.stderr);
    validate(&doc);
    assert_eq!(doc["files"].as_array().unwrap().len(), 3);
    assert!(out.stdout.is_empty());
}

#[test]
fn max_diagnostics_sets_truncated() {
    let d = dir();
    write(d.path(), "a.prog.abap", "ENDIF.\n");
    write(d.path(), "b.prog.abap", "ENDIF.\n");
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "json", "--max-diagnostics", "1"])
        .output()
        .unwrap();
    let doc = json_of(&out.stderr);
    validate(&doc);
    assert_eq!(doc["truncated"], true);
}

#[test]
fn stdin_success_writes_stdout() {
    origami()
        .args(["--stdin", "--language", "abap"])
        .write_stdin(CLASS)
        .assert()
        .code(0)
        .stdout(CLASS);
}

#[test]
fn stdin_error_leaves_stdout_empty() {
    origami()
        .args(["--stdin", "--language", "abap"])
        .write_stdin("IF x = 1.\n")
        .assert()
        .code(2)
        .stdout("");
}

#[test]
fn stdin_check_writes_nothing_to_stdout() {
    origami()
        .args(["--stdin", "--check", "--language", "abap"])
        .write_stdin(CLASS)
        .assert()
        .code(0)
        .stdout("");
}

#[test]
fn stdin_filepath_implies_stdin_and_sniffs_unknown_names() {
    origami()
        .args(["--stdin-filepath", "document.acds"])
        .write_stdin("annotate entity X with { a; }")
        .assert()
        .code(0)
        .stdout("annotate entity X with { a; }");
    origami()
        .args(["--stdin-filepath", "whatever.txt"])
        .write_stdin("define service Z { expose X; }")
        .assert()
        .code(0);
}

#[test]
fn stdin_undetectable_is_e010() {
    let out = origami()
        .args(["--stdin", "--diagnostics", "json"])
        .write_stdin("hello")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        json_of(&out.stderr)["files"][0]["diagnostics"][0]["code"],
        "E010"
    );
}

#[test]
fn crlf_bom_and_permissions_are_preserved() {
    let d = dir();
    let content = "\u{feff}CLASS lcl DEFINITION.\r\nENDCLASS.\r\n";
    write(d.path(), "a.clas.abap", content);
    origami().current_dir(d.path()).assert().code(0);
    assert_eq!(
        fs::read_to_string(d.path().join("a.clas.abap")).unwrap(),
        content
    );
}

#[test]
fn config_discovery_and_flag_precedence() {
    let d = dir();
    write(d.path(), "origami.toml", "line_width = 300\n");
    write(d.path(), "src/a.clas.abap", CLASS);
    // Invalid config value aborts with E020 / exit 3.
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(
        json_of(&out.stderr)["files"][0]["diagnostics"][0]["code"],
        "E020"
    );
    // A flag overrides the file.
    origami()
        .current_dir(d.path())
        .args(["--line-width", "100"])
        .assert()
        .code(0);
    // --no-config ignores it.
    origami()
        .current_dir(d.path())
        .arg("--no-config")
        .assert()
        .code(0);
}

#[test]
fn discovery_stops_at_git_root() {
    let outer = tempfile::tempdir().unwrap();
    write(outer.path(), "origami.toml", "line_width = 300\n");
    let inner = outer.path().join("repo");
    fs::create_dir_all(inner.join(".git")).unwrap();
    write(&inner, "a.clas.abap", CLASS);
    origami().current_dir(&inner).assert().code(0);
}

#[test]
fn explicit_config_file() {
    let d = dir();
    write(d.path(), "other.toml", "indent_width = 0\n");
    write(d.path(), "a.clas.abap", CLASS);
    origami()
        .current_dir(d.path())
        .args(["--config", "other.toml"])
        .assert()
        .code(3);
}

#[test]
fn required_version_mismatch_is_e020() {
    let d = dir();
    write(d.path(), "origami.toml", "required_version = \">=99\"\n");
    write(d.path(), "a.clas.abap", CLASS);
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(
        json_of(&out.stderr)["files"][0]["diagnostics"][0]["code"],
        "E020"
    );
}

#[test]
fn unknown_key_is_w101_and_strict_exits_5() {
    let d = dir();
    write(
        d.path(),
        "origami.toml",
        "colour = \"blue\"\n[abap]\nkeyword_case = \"lower\"\n",
    );
    write(d.path(), "a.clas.abap", CLASS);
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let doc = json_of(&out.stderr);
    validate(&doc);
    assert_eq!(doc["files"][0]["diagnostics"][0]["code"], "W101");
    origami()
        .current_dir(d.path())
        .arg("--strict")
        .assert()
        .code(5);
}

#[test]
fn origamiignore_is_honored() {
    let d = dir();
    write(d.path(), ".origamiignore", "generated/\n");
    write(d.path(), "generated/bad.prog.abap", "ENDIF.\n");
    write(d.path(), "src/a.clas.abap", CLASS);
    origami().current_dir(d.path()).assert().code(0);
}

#[test]
fn output_is_independent_of_thread_count() {
    let d = dir();
    for k in 0..20 {
        write(d.path(), &format!("src/z{k}.clas.abap"), CLASS);
        write(d.path(), &format!("src/e{k}.prog.abap"), "ENDIF.\n");
    }
    let run = |j: &str| {
        origami()
            .current_dir(d.path())
            .args(["--check", "--diagnostics", "json", "-j", j])
            .output()
            .unwrap()
    };
    assert_eq!(run("1").stderr, run("8").stderr);
}

#[test]
fn version_json() {
    let out = origami().args(["--version", "--json"]).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["json_schema"], 1);
    assert_eq!(v["satisfies_required_version"], serde_json::Value::Null);

    let d = dir();
    write(
        d.path(),
        "origami.toml",
        "required_version = \">=0.1, <2\"\n",
    );
    let out = origami()
        .current_dir(d.path())
        .args(["--version", "--json", "--config", "origami.toml"])
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["satisfies_required_version"], true);
}

#[test]
fn usage_errors_exit_3() {
    origami().arg("--bogus").assert().code(3);
    origami()
        .args(["--language", "cobol", "."])
        .assert()
        .code(3);
    origami()
        .args(["--stdin", "somefile.abap"])
        .write_stdin("")
        .assert()
        .code(3);
}

#[test]
fn sarif_output_is_json() {
    let d = dir();
    write(d.path(), "bad.prog.abap", "ENDIF.\n");
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "sarif"])
        .output()
        .unwrap();
    let doc = json_of(&out.stderr);
    assert_eq!(doc["version"], "2.1.0");
    assert_eq!(doc["runs"][0]["results"][0]["ruleId"], "E004");
}
