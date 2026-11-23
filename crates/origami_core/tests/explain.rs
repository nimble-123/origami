//! Line-break explanations (`origami_core::explain`).
//!
//! The samples in `tests/explain/` are checked against their `.notes` files
//! (one line per note: `<first line>: <text>`). Set `ORIGAMI_BLESS=1` to
//! rewrite the `.notes` files.

use origami_core::{
    BreakCause, Explained, LanguageHint, Options, RULE_PHRASES, Scope, explain, format,
    format_lines,
};
use std::path::{Path, PathBuf};

fn samples() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/explain");
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e != "notes"))
        .collect();
    v.sort();
    v
}

fn hint(name: &str) -> LanguageHint {
    LanguageHint::Path(name.to_string())
}

fn run(name: &str, text: &str) -> Explained {
    explain(text.as_bytes(), &hint(name), &Options::default(), &[])
}

fn render(e: &Explained) -> String {
    e.notes
        .iter()
        .map(|n| format!("{}: {}\n", n.lines.start(), n.text))
        .collect()
}

#[test]
fn samples_match_their_notes() {
    let bless = std::env::var_os("ORIGAMI_BLESS").is_some();
    let all = samples();
    assert!(all.len() >= 6, "expected at least 6 samples");
    for path in all {
        let bytes = std::fs::read(&path).unwrap();
        let h = hint(&path.to_string_lossy());
        let e = explain(&bytes, &h, &Options::default(), &[]);
        assert_eq!(
            e.result,
            format(&bytes, &h, &Options::default()),
            "{}",
            path.display()
        );
        assert!(e.result.changed, "{}: sample should change", path.display());
        assert!(
            !e.notes.is_empty(),
            "{}: sample should have notes",
            path.display()
        );
        let got = render(&e);
        let notes = path.with_file_name(format!(
            "{}.notes",
            path.file_name().unwrap().to_string_lossy()
        ));
        if bless {
            std::fs::write(&notes, &got).unwrap();
            continue;
        }
        let want = std::fs::read_to_string(&notes)
            .unwrap_or_else(|_| panic!("{} missing (ORIGAMI_BLESS=1)", notes.display()));
        assert_eq!(got, want, "{}", path.display());
    }
}

#[test]
fn statements_sharing_a_line_get_a_statement_note() {
    let e = run("z.prog.abap", "lv_a = 1. lv_b = 2.\n");
    assert_eq!(e.output_text, "lv_a = 1.\nlv_b = 2.\n");
    assert_eq!(e.notes.len(), 1);
    let n = &e.notes[0];
    assert_eq!(n.rule, Some("ABAP-STMT-001"));
    assert_eq!(n.cause, BreakCause::Rule);
    assert_eq!(n.lines, 2..=2);
    assert_eq!(n.text, "ABAP-STMT-001: each statement on its own line");
}

#[test]
fn width_note_measures_the_statement() {
    let src = "rv_ok = zcl_check=>validate( iv_id = iv_id iv_date = iv_date iv_mode = iv_mode iv_flag = abap_false iv_x = 1 iv_y = 2 iv_z = 3 ).\n";
    let e = run("z.prog.abap", src);
    assert_eq!(e.notes.len(), 1, "{:#?}", e.notes);
    let BreakCause::Width { column, limit, .. } = e.notes[0].cause else {
        panic!("not a width note: {:?}", e.notes[0]);
    };
    assert_eq!(limit, 120);
    // The call's construct ends at `)`, one column before the period.
    assert_eq!(column as usize, src.trim_end().chars().count() - 1);
    assert!(column > 120);
}

#[test]
fn a_comment_inside_a_statement_is_named() {
    let src = "lt_x = VALUE #( ( a = 1 b = 2 ) ( a = 3 \" why\n b = 4 ) ).\n";
    let e = run("z.prog.abap", src);
    assert!(
        e.notes.iter().any(|n| n.cause
            == BreakCause::Comment {
                text: "\" why".into()
            }),
        "{:#?}\n{}",
        e.notes,
        e.output_text
    );
}

#[test]
fn kept_breaks_get_no_note() {
    // Only spacing changes; the line break already exists in the input.
    let e = run("z.prog.abap", "lv_a   =   1.\nlv_b = 2.\n");
    assert!(e.result.changed);
    assert!(e.notes.is_empty(), "{:#?}", e.notes);
}

#[test]
fn unchanged_and_failed_sources_get_no_notes() {
    let e = run("z.prog.abap", "lv_a = 1.\n");
    assert!(!e.result.changed);
    assert!(e.notes.is_empty());
    let e = run("z.prog.abap", "lv_a = 1\n");
    assert!(e.result.output.is_none());
    assert!(e.notes.is_empty());
    assert!(e.output_text.is_empty());
}

#[test]
fn cds_unit_start_gets_a_unit_note() {
    let e = run(
        "zi_a.ddls.asddls",
        "define view entity ZI_A as select from t {\n  key a, b\n}\n",
    );
    assert!(
        e.notes.iter().any(|n| n.rule == Some("CDS-UNIT-001")),
        "{:#?}\n{}",
        e.notes,
        e.output_text
    );
}

#[test]
fn lines_keep_only_notes_of_kept_changes_and_map_them() {
    let src = "lv_a = 1. lv_b = 2.\nlv_x = 0.\nlv_c = 3. lv_d = 4.\n";
    let opts = Options::default();
    // Only input line 3 is formatted; its note moves to the mixed text.
    let e = explain(src.as_bytes(), &hint("z.prog.abap"), &opts, &[(3, 3)]);
    assert_eq!(
        e.result,
        format_lines(src.as_bytes(), &hint("z.prog.abap"), &opts, &[(3, 3)])
    );
    assert_eq!(
        e.output_text,
        "lv_a = 1. lv_b = 2.\nlv_x = 0.\nlv_c = 3.\nlv_d = 4.\n"
    );
    assert_eq!(e.notes.len(), 1, "{:#?}", e.notes);
    assert_eq!(e.notes[0].lines, 4..=4);
    // The whole file: two notes, on output lines 2 and 5.
    let all = run("z.prog.abap", src);
    assert_eq!(all.notes.len(), 2, "{:#?}", all.notes);
    assert_eq!(*all.notes[0].lines.start(), 2);
    assert_eq!(*all.notes[1].lines.start(), 5);
}

#[test]
fn rule_phrases_equal_the_explanation_lines_in_style_md() {
    let style =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/STYLE.md"))
            .unwrap();
    let mut documented = std::collections::BTreeMap::new();
    let mut rule = None;
    for line in style.lines() {
        if let Some(h) = line.strip_prefix("### ") {
            rule = h.split(':').next().map(str::to_string);
        } else if let Some(phrase) = line.strip_prefix("Explanation: ") {
            documented.insert(
                rule.clone().expect("Explanation before a rule"),
                phrase.to_string(),
            );
        }
    }
    for (id, phrase, _) in RULE_PHRASES {
        assert_eq!(
            documented.get(*id).map(String::as_str),
            Some(*phrase),
            "{id}: the Explanation line in STYLE.md differs from the code"
        );
    }
    assert_eq!(
        documented.len(),
        RULE_PHRASES.len(),
        "STYLE.md has Explanation lines without a code entry"
    );
}

#[test]
fn a_long_call_gets_one_call_001_width_note() {
    let src = "rv_ok = zcl_check=>validate( iv_id = iv_id iv_date = iv_date iv_mode = iv_mode iv_flag = abap_false iv_x = 1 iv_y = 2 iv_z = 3 ).\n";
    let e = run("z.prog.abap", src);
    assert_eq!(e.notes.len(), 1, "{:#?}", e.notes);
    let n = &e.notes[0];
    assert_eq!(n.rule, Some("ABAP-CALL-001"));
    let BreakCause::Width { column, scope, .. } = n.cause else {
        panic!("{n:?}");
    };
    assert!(column > 120);
    assert_eq!(scope, Scope::Construct);
    assert!(
        n.text.starts_with(
            "ABAP-CALL-001: one parameter per line, because the call would end at column"
        ),
        "{}",
        n.text
    );
}

#[test]
fn call_function_with_a_keyword_group_gets_call_003() {
    let src = "CALL FUNCTION 'Z_LONG_FUNCTION_NAME' EXPORTING iv_first = lv_first iv_second = lv_second IMPORTING ev_result = lv_result EXCEPTIONS OTHERS = 1.\n";
    let e = run("z.prog.abap", src);
    assert!(!e.notes.is_empty(), "{}", e.output_text);
    assert!(
        e.notes.iter().all(|n| n.rule == Some("ABAP-CALL-003")),
        "{:#?}\n{}",
        e.notes,
        e.output_text
    );
}

#[test]
fn a_cds_header_clause_gets_cds_head_001() {
    let e = run(
        "zi_a.ddls.asddls",
        "define view entity ZI_A as select from t association [0..1] to ZI_B as _B on $projection.a = _B.a {\n  key a,\n  _B\n}\n",
    );
    assert!(
        e.notes.iter().any(|n| n.rule == Some("CDS-HEAD-001")),
        "{:#?}\n{}",
        e.notes,
        e.output_text
    );
}

#[test]
fn a_query_clause_gets_cds_clause_001_and_a_kept_break_gets_none() {
    let e = run(
        "zi_a.ddls.asddls",
        "define view entity ZI_A as select from t\n{\n  key a\n} where a = 1 group by a\nhaving count(*) > 1\n",
    );
    let clauses = e
        .notes
        .iter()
        .filter(|n| n.rule == Some("CDS-CLAUSE-001"))
        .count();
    // `where` and `group by` start lines; `having` already did.
    assert_eq!(clauses, 2, "{:#?}\n{}", e.notes, e.output_text);
}

#[test]
fn a_failed_parse_attempt_leaves_no_span() {
    // `READ ENTITIES` with an unknown addition fails in the EML parser and
    // falls back to the keyword-statement engine or the generic layout; no
    // EML-001 note may survive the failed attempt.
    let src = "READ ENTITIES OF zi_travel ENTITY travel FIELDS ( a b ) WITH VALUE #( ( a = 1 ) ) RESULT DATA(lt) FAILED DATA(f) REPORTED DATA(r) UNKNOWN_ADDITION x.\n";
    let e = run("z.prog.abap", src);
    assert!(
        e.notes.iter().all(|n| n.rule != Some("ABAP-EML-001")),
        "{:#?}\n{}",
        e.notes,
        e.output_text
    );
}
