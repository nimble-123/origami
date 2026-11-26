//! CLI integration tests. M0: the printer runs in identity mode, so "would
//! change" is exercised through files that need no change and through errors.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

const CLASS: &str = "CLASS lcl DEFINITION.\n  PUBLIC SECTION.\nENDCLASS.\n";

fn origami() -> Command {
    let mut c = Command::cargo_bin("origami").unwrap();
    // The default diagnostics format depends on the environment.
    c.env_remove("CLAUDECODE")
        .env_remove("ORIGAMI_DIAGNOSTICS")
        .env_remove("CLICOLOR_FORCE");
    c
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
fn discovery_walks_above_a_relative_working_directory() {
    let d = dir();
    write(d.path(), "origami.toml", "line_width = 300\n");
    write(d.path(), "src/a.clas.abap", CLASS);
    // From the project root and from below it, the same file is found.
    for (cwd, file) in [(".", "src/a.clas.abap"), ("src", "a.clas.abap")] {
        origami()
            .current_dir(d.path().join(cwd))
            .arg(file)
            .assert()
            .code(3);
    }
    // Also through `..`.
    write(d.path(), "other/b.clas.abap", CLASS);
    origami()
        .current_dir(d.path().join("src"))
        .arg("../other/b.clas.abap")
        .assert()
        .code(3);
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
fn style_is_read_from_config_and_flag() {
    let d = dir();
    write(d.path(), "origami.toml", "style = \"compact\"\n");
    write(d.path(), "a.clas.abap", CLASS);
    origami().current_dir(d.path()).assert().code(0);
    origami()
        .current_dir(d.path())
        .args(["--style", "aligned"])
        .assert()
        .code(0);
    write(d.path(), "origami.toml", "style = \"wide\"\n");
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

#[cfg(unix)]
#[test]
fn writing_through_a_symlink_keeps_the_link_and_formats_the_target() {
    let d = dir();
    write(d.path(), "real/a.prog.abap", "lv_a   =   1.\n");
    let link = d.path().join("link.prog.abap");
    std::os::unix::fs::symlink("real/a.prog.abap", &link).unwrap();
    origami()
        .current_dir(d.path())
        .arg("link.prog.abap")
        .assert()
        .code(0);
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_to_string(d.path().join("real/a.prog.abap")).unwrap(),
        "lv_a = 1.\n"
    );
}

#[cfg(unix)]
#[test]
fn a_read_only_file_is_not_replaced_and_reports_e031() {
    use std::os::unix::fs::PermissionsExt;
    let d = dir();
    write(d.path(), "a.prog.abap", "lv_a   =   1.\n");
    let file = d.path().join("a.prog.abap");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o444)).unwrap();
    // Root can write a 0444 file, so there is nothing to test as root.
    if fs::OpenOptions::new().write(true).open(&file).is_ok() {
        return;
    }
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "json", "a.prog.abap"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4));
    assert_eq!(
        json_of(&out.stderr)["files"][0]["diagnostics"][0]["code"],
        "E031"
    );
    assert_eq!(fs::read_to_string(&file).unwrap(), "lv_a   =   1.\n");
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o444
    );
}

/// Runs origami with stdout closed before it writes (like `| head -c1`) and
/// returns its exit code and stderr.
#[cfg(unix)]
fn run_with_closed_stdout(args: &[&str], stdin: &str) -> (Option<i32>, String) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new(env!("CARGO_BIN_EXE_origami"))
        .args(args)
        .env_remove("CLAUDECODE")
        .env_remove("ORIGAMI_DIAGNOSTICS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let mut input = child.stdin.take().unwrap();
    let _ = input.write_all(stdin.as_bytes());
    drop(input);
    let out = child.wait_with_output().unwrap();
    (out.status.code(), String::from_utf8(out.stderr).unwrap())
}

#[cfg(unix)]
#[test]
fn a_closed_stdout_is_not_an_internal_error() {
    let (code, err) = run_with_closed_stdout(&["--version"], "");
    assert_eq!((code, err.as_str()), (Some(0), ""));
    let (code, err) = run_with_closed_stdout(&["--version", "--json"], "");
    assert_eq!((code, err.as_str()), (Some(0), ""));
    // The diff of a changed source goes to stdout, the verdict is exit 1.
    let (code, err) = run_with_closed_stdout(
        &["--check", "--diff", "--stdin", "--language", "abap"],
        "lv_a   =   1.\n",
    );
    assert_eq!(code, Some(1), "{err}");
    assert!(!err.contains("X002"), "{err}");
    let (code, err) = run_with_closed_stdout(
        &["--review", "--stdin", "--language", "abap"],
        "lv_a   =   1.\n",
    );
    assert_eq!(code, Some(1), "{err}");
    assert!(!err.contains("X002"), "{err}");
}

#[test]
fn sarif_uris_are_percent_encoded_and_pseudo_paths_have_no_location() {
    let d = dir();
    write(d.path(), "my dir/bad#1.prog.abap", "ENDIF.\n");
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "sarif"])
        .output()
        .unwrap();
    let doc = json_of(&out.stderr);
    let loc = &doc["runs"][0]["results"][0]["locations"][0]["physicalLocation"];
    assert_eq!(loc["artifactLocation"]["uri"], "my%20dir/bad%231.prog.abap");
    // A usage error belongs to no file.
    let out = origami()
        .current_dir(d.path())
        .args(["--diagnostics", "sarif", "--lines", "x", "my dir"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let result = &json_of(&out.stderr)["runs"][0]["results"][0];
    assert_eq!(result["ruleId"], "E021");
    assert!(result.get("locations").is_none(), "{result}");
}

#[test]
fn check_diff_prints_a_unified_diff_to_stdout() {
    let d = dir();
    write(d.path(), "z.prog.abap", "lv_a   =   1.\n");
    let out = origami()
        .current_dir(d.path())
        .args(["--check", "--diff"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let diff = String::from_utf8(out.stdout).unwrap();
    assert!(diff.contains("--- a/z.prog.abap"), "{diff}");
    assert!(diff.contains("-lv_a   =   1."), "{diff}");
    assert!(diff.contains("+lv_a = 1."), "{diff}");
    // Nothing was written.
    assert_eq!(
        fs::read_to_string(d.path().join("z.prog.abap")).unwrap(),
        "lv_a   =   1.\n"
    );
}

#[test]
fn write_mode_rewrites_changed_files() {
    let d = dir();
    write(d.path(), "z.prog.abap", "lv_a   =   1.\n");
    origami().current_dir(d.path()).assert().code(0);
    assert_eq!(
        fs::read_to_string(d.path().join("z.prog.abap")).unwrap(),
        "lv_a = 1.\n"
    );
}

#[test]
fn lines_formats_only_touched_statements() {
    let input = "lv_a   =   1.\nlv_b   =   2.\nlv_c   =   3.\n";
    let out = origami()
        .args([
            "--no-config",
            "--stdin-filepath",
            "z.prog.abap",
            "--lines",
            "2:2",
        ])
        .write_stdin(input)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "lv_a   =   1.\nlv_b = 2.\nlv_c   =   3.\n"
    );
}

#[test]
fn lines_rejects_bad_ranges_and_directories() {
    origami()
        .args([
            "--no-config",
            "--stdin-filepath",
            "z.prog.abap",
            "--lines",
            "3:1",
        ])
        .write_stdin("x = 1.\n")
        .assert()
        .code(3);
    let d = dir();
    origami()
        .current_dir(d.path())
        .args(["--lines", "1:1", "."])
        .assert()
        .code(3);
}

#[test]
fn changed_formats_changed_lines_and_untracked_files() {
    let d = dir();
    let git = |args: &[&str]| {
        let ok = std::process::Command::new("git")
            .args(["-c", "user.email=t@example.com", "-c", "user.name=t"])
            .args(args)
            .current_dir(d.path())
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q"]);
    write(d.path(), "z.prog.abap", "lv_a   =   1.\nlv_b   =   2.\n");
    git(&["add", "."]);
    git(&["commit", "-qm", "init"]);
    write(d.path(), "z.prog.abap", "lv_a   =   1.\nlv_b   =   3.\n");
    write(d.path(), "y.prog.abap", "lv_c   =   4.\n");
    origami()
        .current_dir(d.path())
        .args(["--no-config", "--changed"])
        .assert()
        .code(0);
    assert_eq!(
        fs::read_to_string(d.path().join("z.prog.abap")).unwrap(),
        "lv_a   =   1.\nlv_b = 3.\n"
    );
    assert_eq!(
        fs::read_to_string(d.path().join("y.prog.abap")).unwrap(),
        "lv_c = 4.\n"
    );
}

#[test]
fn changed_outside_git_is_a_usage_error() {
    let d = dir();
    origami()
        .current_dir(d.path())
        .env("GIT_CEILING_DIRECTORIES", d.path())
        .args(["--no-config", "--changed"])
        .assert()
        .code(3);
}

#[test]
fn mcp_server_formats_source() {
    let session = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}"#,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"format","arguments":{"source":"x   =   1.\n","filename":"z.prog.abap"}}}"#,
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"format","arguments":{"source":"x = 1\n","filename":"z.prog.abap"}}}"#,
    ]
    .join("\n");
    let out = origami()
        .arg("--mcp")
        .write_stdin(session)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let replies: Vec<serde_json::Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    // The notification gets no reply.
    assert_eq!(replies.len(), 4);
    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(replies[1]["result"]["tools"].as_array().unwrap().len(), 2);
    assert_eq!(replies[2]["result"]["content"][0]["text"], "x = 1.\n");
    assert_eq!(replies[2]["result"]["isError"], false);
    assert_eq!(replies[3]["result"]["isError"], true);
    assert_eq!(
        replies[3]["result"]["structuredContent"]["diagnostics"][0]["code"],
        "E003"
    );
}

fn project() -> tempfile::TempDir {
    let d = dir();
    write(
        d.path(),
        "src/zcl_ok.clas.abap",
        "CLASS zcl_ok DEFINITION.\nENDCLASS.\n",
    );
    write(d.path(), "src/zcl_a.clas.abap", "lv_a   =   1.\n");
    write(d.path(), "src/zcl_b.prog.abap", "lv_x = ( 1 .\n");
    d
}

#[test]
fn human_output_lists_files_with_a_code_frame_and_a_summary() {
    let d = project();
    let out = origami()
        .current_dir(d.path())
        .args(["--no-config", "--check", "src"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    // Paths use the platform separator; compare with `/`.
    let err = String::from_utf8(out.stderr).unwrap().replace('\\', "/");
    assert!(
        !err.contains('\x1b'),
        "no colors when stderr is not a terminal"
    );
    assert!(err.contains("✎ src/zcl_a.clas.abap  would change"), "{err}");
    assert!(
        err.contains("✗ src/zcl_b.prog.abap  not formatted, left unchanged"),
        "{err}"
    );
    assert!(err.contains("1 │ lv_x = ( 1 ."), "{err}");
    assert!(
        err.contains("✓ 1 unchanged · ✎ 1 would change · ✗ 1 error  (3 files)"),
        "{err}"
    );
}

#[test]
fn colors_can_be_forced_and_disabled() {
    let d = project();
    let run = |env: (&str, &str)| {
        let out = origami()
            .current_dir(d.path())
            .env(env.0, env.1)
            .args(["--no-config", "--check", "src"])
            .output()
            .unwrap();
        String::from_utf8(out.stderr).unwrap()
    };
    assert!(run(("CLICOLOR_FORCE", "1")).contains("\x1b[31m"));
    assert!(!run(("NO_COLOR", "1")).contains('\x1b'));
}

#[test]
fn agent_output_is_compact_with_hints_and_next_step() {
    let d = project();
    let out = origami()
        .current_dir(d.path())
        .args(["--no-config", "--check", "--diagnostics", "agent", "src"])
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap().replace('\\', "/");
    let lines: Vec<&str> = err.lines().collect();
    assert_eq!(
        lines[0],
        "origami: 3 files, 1 would change, 1 with errors, 0 warnings"
    );
    assert!(lines.contains(&"CHANGE src/zcl_a.clas.abap"), "{err}");
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("ERROR src/zcl_b.prog.abap:1:")
                && l.contains("Fix: balance the brackets.")
                && l.ends_with("The file was not changed.")),
        "{err}"
    );
    assert!(
        lines
            .last()
            .unwrap()
            .starts_with("NEXT: fix the errors above"),
        "{err}"
    );
}

#[test]
fn agent_output_is_the_default_for_claude_code_and_can_be_overridden() {
    let d = project();
    let run = |envs: &[(&str, &str)], args: &[&str]| {
        let mut c = origami();
        c.current_dir(d.path());
        for (k, v) in envs {
            c.env(k, v);
        }
        let out = c
            .args(["--no-config", "--check"])
            .args(args)
            .arg("src")
            .output()
            .unwrap();
        String::from_utf8(out.stderr).unwrap().replace('\\', "/")
    };
    assert!(run(&[("CLAUDECODE", "1")], &[]).starts_with("origami: 3 files"));
    assert!(run(&[("ORIGAMI_DIAGNOSTICS", "agent")], &[]).starts_with("origami: 3 files"));
    assert!(run(&[("CLAUDECODE", "1")], &["--diagnostics", "human"]).contains("✎ src/zcl_a"));
}

#[test]
fn code_frame_escapes_control_characters() {
    let out = origami()
        .args([
            "--no-config",
            "--stdin-filepath",
            "z.prog.abap",
            "--diagnostics",
            "human",
        ])
        .write_stdin("lv_x = ( '\x1b[31mred' .\n")
        .output()
        .unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(!err.contains('\x1b'), "{err}");
    assert!(err.contains("\\x1b[31mred"), "{err}");
}

#[test]
fn changed_takes_the_next_argument_as_revision() {
    let d = dir();
    let git = |args: &[&str]| {
        assert!(
            std::process::Command::new("git")
                .args(["-c", "user.email=t@example.com", "-c", "user.name=t"])
                .args(args)
                .current_dir(d.path())
                .output()
                .unwrap()
                .status
                .success()
        );
    };
    git(&["init", "-q"]);
    write(d.path(), "src/z.prog.abap", "lv_a   =   1.\n");
    git(&["add", "."]);
    git(&["commit", "-qm", "init"]);
    git(&["branch", "base"]);
    write(d.path(), "src/z.prog.abap", "lv_a   =   2.\n");
    // `base` is the revision, `src` the path.
    origami()
        .current_dir(d.path())
        .args(["--no-config", "--changed", "base", "src"])
        .assert()
        .code(0);
    assert_eq!(
        fs::read_to_string(d.path().join("src/z.prog.abap")).unwrap(),
        "lv_a = 2.\n"
    );
}

#[test]
fn review_shows_changes_with_line_numbers_and_notes() {
    let d = dir();
    let src = "lv_a = 1. lv_b   =   2.\n";
    write(d.path(), "z.prog.abap", src);
    let out = origami()
        .current_dir(d.path())
        .args(["--no-config", "--review", "z.prog.abap"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let view = String::from_utf8(out.stdout).unwrap();
    assert!(
        view.starts_with("z.prog.abap  (1 change, 1 note)\n"),
        "{view}"
    );
    assert!(
        view.contains("1   │-lv_a = 1.[- -]lv_b[-   -]=[-   -]2.\n"),
        "{view}"
    );
    assert!(view.contains("  2 │+lv_b{+ +}={+ +}2.\n"), "{view}");
    assert!(
        view.contains("↳ ABAP-STMT-001: each statement on its own line"),
        "{view}"
    );
    assert!(
        !view.contains('\x1b'),
        "escape sequences without a terminal"
    );
    assert_eq!(
        fs::read_to_string(d.path().join("z.prog.abap")).unwrap(),
        src
    );
}

#[test]
fn review_of_a_formatted_file_prints_nothing() {
    let d = dir();
    write(d.path(), "z.prog.abap", "lv_a = 1.\n");
    origami()
        .current_dir(d.path())
        .args(["--no-config", "--review", "z.prog.abap"])
        .assert()
        .code(0)
        .stdout("");
}

#[test]
fn review_with_a_structural_error_exits_2_without_a_view() {
    let d = dir();
    write(d.path(), "z.prog.abap", "lv_a   =   1\n");
    origami()
        .current_dir(d.path())
        .args(["--no-config", "--review", "z.prog.abap"])
        .assert()
        .code(2)
        .stdout("");
}

#[test]
fn review_conflicts_with_diff_and_write() {
    for extra in ["--diff", "--write"] {
        origami()
            .args(["--review", extra, "x.prog.abap"])
            .assert()
            .code(3);
    }
}

#[test]
fn review_in_stdin_mode_prints_the_view_not_the_text() {
    let out = origami()
        .args(["--no-config", "--review", "--stdin-filepath", "z.prog.abap"])
        .write_stdin("lv_a   =   1.\n")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let view = String::from_utf8(out.stdout).unwrap();
    assert!(
        view.starts_with("z.prog.abap  (1 change, 0 notes)\n"),
        "{view}"
    );
    assert!(!view.contains("\nlv_a = 1.\n"), "{view}");
}

#[test]
fn review_escapes_control_characters() {
    let d = dir();
    write(d.path(), "z.prog.abap", "lv_x  =  `\x1b[31m\u{202e}`.\n");
    let out = origami()
        .current_dir(d.path())
        .args(["--no-config", "--review", "z.prog.abap"])
        .output()
        .unwrap();
    let view = String::from_utf8(out.stdout).unwrap();
    assert!(view.contains(r"`\x1b[31m\u{202e}`"), "{view}");
    assert!(
        !view.contains('\x1b') && !view.contains('\u{202e}'),
        "{view}"
    );
}

#[test]
fn review_with_changed_shows_only_changed_lines() {
    let d = dir();
    let git = |args: &[&str]| {
        let ok = std::process::Command::new("git")
            .args(["-c", "user.email=t@example.com", "-c", "user.name=t"])
            .args(args)
            .current_dir(d.path())
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?}");
    };
    git(&["init", "-q"]);
    write(
        d.path(),
        "z.prog.abap",
        "lv_a   =   1.\nlv_x = 0.\nlv_y = 0.\nlv_z = 0.\nlv_b   =   2.\n",
    );
    git(&["add", "."]);
    git(&["commit", "-qm", "init"]);
    write(
        d.path(),
        "z.prog.abap",
        "lv_a   =   1.\nlv_x = 0.\nlv_y = 0.\nlv_z = 0.\nlv_b = 3. lv_c   =   4.\n",
    );
    let out = origami()
        .current_dir(d.path())
        .args(["--no-config", "--review", "--changed=HEAD"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let view = String::from_utf8(out.stdout).unwrap();
    assert!(!view.contains("lv_a"), "{view}");
    assert!(view.contains("│+lv_c{+ +}={+ +}4.\n"), "{view}");
    assert!(view.contains("↳ ABAP-STMT-001"), "{view}");
}

#[test]
fn mcp_format_files_stays_below_the_working_directory() {
    let outer = tempfile::tempdir().unwrap();
    write(outer.path(), "outside.prog.abap", "lv_a   =   1.\n");
    let d = outer.path().join("project");
    fs::create_dir_all(d.join(".git")).unwrap();
    write(&d, "src/z.prog.abap", "lv_b   =   2.\n");
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        outer.path().join("outside.prog.abap"),
        d.join("src/link.prog.abap"),
    )
    .unwrap();
    let call = |id: u32, args: &str| {
        format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"format_files","arguments":{args}}}}}"#
        )
    };
    let session = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{}}}"#.to_string(),
        call(2, r#"{"paths":["../outside.prog.abap"],"write":true}"#),
        call(
            3,
            &serde_json::json!({
                "paths": [outer.path().join("outside.prog.abap")],
                "write": true
            })
            .to_string(),
        ),
        call(4, r#"{"paths":["src/../../outside.prog.abap"],"write":true}"#),
        call(5, r#"{"paths":["src/link.prog.abap"],"write":true}"#),
        call(6, r#"{"paths":["src"],"write":true}"#),
    ]
    .join("\n");
    let out = origami()
        .current_dir(&d)
        .arg("--mcp")
        .write_stdin(session)
        .output()
        .unwrap();
    let replies: Vec<serde_json::Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    for r in &replies[1..4] {
        assert_eq!(r["result"]["isError"], true, "{r}");
    }
    // A symbolic link to a file outside is refused when named; a directory
    // walk does not follow it.
    #[cfg(unix)]
    assert_eq!(replies[4]["result"]["isError"], true, "{}", replies[4]);
    assert_eq!(replies[5]["result"]["isError"], false, "{}", replies[5]);
    assert_eq!(
        replies[5]["result"]["structuredContent"]["changed"], 1,
        "{}",
        replies[5]
    );
    assert_eq!(
        fs::read_to_string(outer.path().join("outside.prog.abap")).unwrap(),
        "lv_a   =   1.\n"
    );
    assert_eq!(
        fs::read_to_string(d.join("src/z.prog.abap")).unwrap(),
        "lv_b = 2.\n"
    );
}

#[test]
fn usage_errors_follow_the_diagnostics_format() {
    let out = origami()
        .args(["--diagnostics", "json", "--bogus"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let doc = json_of(&out.stderr);
    validate(&doc);
    assert_eq!(doc["files"][0]["path"], "<args>");
    assert_eq!(doc["files"][0]["diagnostics"][0]["code"], "E021");
    let msg = doc["files"][0]["diagnostics"][0]["message"]
        .as_str()
        .unwrap();
    assert!(msg.contains("--bogus"), "{msg}");
    // From the environment.
    let out = origami()
        .env("ORIGAMI_DIAGNOSTICS", "json")
        .args(["--line-width", "abc"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(
        json_of(&out.stderr)["files"][0]["diagnostics"][0]["code"],
        "E021"
    );
    // Human output stays clap's own.
    let out = origami().arg("--bogus").output().unwrap();
    assert!(String::from_utf8_lossy(&out.stderr).contains("Usage:"));
}

#[test]
fn bad_option_flags_name_the_flag() {
    let d = dir();
    write(d.path(), "origami.toml", "line_width = 100\n");
    let out = origami()
        .current_dir(d.path())
        .args(["--stdin", "--language", "abap", "--line-width", "10"])
        .args(["--diagnostics", "agent"])
        .write_stdin("REPORT z.\n")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("ERROR <args>:1:1 E020 --line-width 10"),
        "{err}"
    );
    assert!(
        err.contains("Fix the flag value on the command line."),
        "{err}"
    );
    assert!(!err.contains("origami.toml"), "{err}");
}

#[test]
fn toml_errors_are_reported_where_they_are() {
    let d = dir();
    write(d.path(), "origami.toml", "line_width = 100\n\nfoo = [\n");
    let out = origami()
        .current_dir(d.path())
        .args(["--stdin", "--language", "abap", "--diagnostics", "json"])
        .write_stdin("REPORT z.\n")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    let doc = json_of(&out.stderr);
    validate(&doc);
    let d0 = &doc["files"][0]["diagnostics"][0];
    assert_eq!(d0["code"], "E020");
    assert_eq!(d0["range"]["start"]["line"], 3);
    assert_eq!(d0["range"]["start"]["column"], 8);
}

#[test]
fn invalid_utf8_is_reported_at_the_first_bad_byte() {
    let out = origami()
        .args(["--stdin", "--language", "abap", "--diagnostics", "json"])
        .write_stdin(&b"\xEF\xBB\xBFREPORT z.\r\nWRITE '\xC3\xA4\xFF'.\n"[..])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let doc = json_of(&out.stderr);
    validate(&doc);
    let d0 = &doc["files"][0]["diagnostics"][0];
    assert_eq!(d0["code"], "E005");
    // Line 2, after `WRITE 'ä`; the offset counts LF line breaks, no BOM.
    assert_eq!(d0["range"]["start"]["line"], 2);
    assert_eq!(d0["range"]["start"]["column"], 9);
    assert_eq!(d0["range"]["start"]["offset"], 19);
}

/// Runs `origami --mcp` on the messages and returns the replies.
fn mcp_session(cwd: &Path, env: &[(&str, &str)], messages: &[String]) -> Vec<serde_json::Value> {
    let mut cmd = origami();
    cmd.current_dir(cwd).arg("--mcp");
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd
        .write_stdin(messages.join("\n"))
        .timeout(std::time::Duration::from_secs(60))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn tool_call(id: u32, name: &str, arguments: serde_json::Value) -> String {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
        "params": {"name": name, "arguments": arguments}})
    .to_string()
}

#[test]
fn mcp_rejects_bad_line_ranges() {
    let d = dir();
    let call = |id, lines| {
        tool_call(
            id,
            "format",
            serde_json::json!({"source": "x   =   1.\n", "language": "abap", "lines": lines}),
        )
    };
    let replies = mcp_session(
        d.path(),
        &[],
        &[
            call(
                1,
                serde_json::json!([[4_294_967_297_u64, 4_294_967_298_u64]]),
            ),
            call(2, serde_json::json!([[0, 1]])),
            call(3, serde_json::json!([[2, 1]])),
            call(4, serde_json::json!([[1, 1]])),
        ],
    );
    for r in &replies[..3] {
        assert_eq!(r["error"]["code"], -32602, "{r}");
    }
    assert_eq!(replies[3]["result"]["content"][0]["text"], "x = 1.\n");
}

#[test]
fn mcp_rejects_too_long_messages_and_keeps_serving() {
    let d = dir();
    let big = tool_call(
        7,
        "format",
        serde_json::json!({"source": "x".repeat(17 * 1024 * 1024)}),
    );
    let ping = r#"{"jsonrpc":"2.0","id":8,"method":"ping"}"#.to_string();
    let replies = mcp_session(d.path(), &[], &[big, ping]);
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["id"], 7);
    assert_eq!(replies[0]["error"]["code"], -32600);
    assert!(
        replies[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("16 MiB")
    );
    assert_eq!(replies[1]["id"], 8);
}

#[test]
fn mcp_tool_calls_time_out() {
    let d = dir();
    let replies = mcp_session(
        d.path(),
        &[("ORIGAMI_MCP_TIMEOUT", "0")],
        &[tool_call(
            1,
            "format",
            serde_json::json!({"source": "x = 1.\n"}),
        )],
    );
    assert_eq!(replies[0]["id"], 1);
    assert_eq!(replies[0]["error"]["code"], -32001, "{}", replies[0]);
}

/// A call that blocks on a named pipe: cancelled, or timed out, while the
/// server keeps answering.
#[cfg(unix)]
#[test]
fn mcp_blocked_calls_can_be_cancelled_and_time_out() {
    let d = dir();
    let fifo = d.path().join("z.prog.abap");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let blocked = |id| {
        tool_call(
            id,
            "format_files",
            serde_json::json!({"paths": ["z.prog.abap"]}),
        )
    };
    let ping = |id: u32| format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"ping"}}"#);
    let cancel = r#"{"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":1}}"#;
    // Cancelled: no reply for it.
    let replies = mcp_session(d.path(), &[], &[blocked(1), cancel.to_string(), ping(2)]);
    assert_eq!(replies.len(), 1, "{replies:?}");
    assert_eq!(replies[0]["id"], 2);
    // Timed out: an error, after the ping that came in meanwhile.
    let replies = mcp_session(
        d.path(),
        &[("ORIGAMI_MCP_TIMEOUT", "0.3")],
        &[blocked(1), ping(2)],
    );
    assert_eq!(replies.len(), 2, "{replies:?}");
    assert_eq!(replies[0]["id"], 2);
    assert_eq!(replies[1]["id"], 1);
    assert_eq!(replies[1]["error"]["code"], -32001);
}

#[test]
fn mcp_format_files_caps_the_result() {
    let d = dir();
    // 40 files with a diff of about 4 KB each: more than the budget.
    let source: String = (0..100).map(|i| format!("lv_{i}   =   {i}.\n")).collect();
    for i in 0..40 {
        write(d.path(), &format!("src/z{i:02}.prog.abap"), &source);
    }
    let replies = mcp_session(
        d.path(),
        &[],
        &[tool_call(
            1,
            "format_files",
            serde_json::json!({"paths": ["src"]}),
        )],
    );
    let r = &replies[0]["result"];
    let s = &r["structuredContent"];
    assert_eq!(s["changed"], 40);
    assert_eq!(s["truncated"], true);
    let files = s["files"].as_array().unwrap();
    assert_eq!(files.len() + s["omitted"].as_u64().unwrap() as usize, 40);
    assert!(files[0].get("diff").is_some());
    assert_eq!(files.last().unwrap()["diff_omitted"], true);
    let text = r["content"][0]["text"].as_str().unwrap();
    assert!(
        text.starts_with("40 file(s) processed, 40 would change"),
        "{text}"
    );
    assert!(
        text.contains("call format_files again with `paths`"),
        "{text}"
    );
    assert!(replies[0].to_string().len() < 160 * 1024);
}
