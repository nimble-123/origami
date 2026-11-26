//! Explanations for the line breaks origami introduces.
//!
//! The final formatting pass is printed a second time without a line-width
//! limit. Output token k is input token k (the self-check guarantees it), so
//! for each token we know whether it starts a line in the input, in the
//! output and in the wide print. A line start in the output that was not one
//! in the input is an introduced break:
//!
//! * not a line start in the wide print: caused by the width limit;
//! * the token before it carries an end-of-line comment: caused by it;
//! * otherwise: a layout rule always breaks there.
//!
//! An end-of-line comment inside a construct forces it onto several lines.
//! So a break that no rule explains (a rule break without a rule ID, or a
//! width break where neither the construct nor its line exceeds the limit in
//! the wide print) is attributed to such a comment if there is one; a width
//! break without one is a layout choice.
//!
//! Breaks are grouped into one [`Note`] per construct and cause. The
//! construct is the innermost rule span that contains the break (recorded
//! by the language crate), else the ABAP statement or CDS layout unit.

use crate::Inner;
use crate::check::{self, Checked, LineMap};
use crate::options::Options;
use crate::result::FormatResult;
use origami_syntax::{
    BreakCause, Language, Note, Placement, RuleId, Scope, SourceText, TokenIndex, TokenStream,
    TriviaKind, TriviaPosition, UnitId,
};
use std::collections::{BTreeMap, BTreeSet};

/// A formatting result with explanations of its introduced line breaks.
#[derive(Debug, Clone)]
pub struct Explained {
    /// The same result as `format_lines` returns.
    pub result: FormatResult,
    /// Notes in output order; empty if nothing changed, on error, or when the
    /// whole file stayed as written.
    pub notes: Vec<Note>,
    /// The input, decoded and normalized (LF line ends, no BOM).
    pub input_text: String,
    /// The output, normalized like `input_text`.
    pub output_text: String,
}

/// What the explain pass needs from formatting.
pub(crate) struct Parts {
    pub src: SourceText,
    pub lang: Language,
    pub identity: bool,
    /// The final pass, with the full output (before line selection).
    pub checked: Checked,
    /// With `lines`: where the kept changes went (`None`: none could be kept).
    pub selection: Option<Option<LineMap>>,
    /// The output as returned (after line selection).
    pub output_text: String,
}

pub(crate) fn explain(inner: Inner, opts: &Options) -> Explained {
    let Inner { result, parts } = inner;
    let Some(parts) = parts else {
        return Explained {
            result,
            notes: Vec::new(),
            input_text: String::new(),
            output_text: String::new(),
        };
    };
    let quiet = !result.changed || parts.checked.whole_file_fallback;
    let mut notes = if quiet {
        Vec::new()
    } else {
        notes(&parts, opts)
    };
    match &parts.selection {
        None => {}
        Some(None) => notes.clear(),
        Some(Some(map)) => notes = select(notes, map),
    }
    let notes = notes.into_iter().map(|l| l.note).collect();
    Explained {
        result,
        notes,
        input_text: parts.src.text().to_string(),
        output_text: parts.output_text,
    }
}

// ---- sentences ------------------------------------------------------------

/// Rule ID, rule phrase and construct noun. Each phrase is also the
/// `Explanation:` line of its rule in docs/STYLE.md (checked by a test).
pub const RULE_PHRASES: &[(RuleId, &str, &str)] = &[
    (
        "ABAP-STMT-001",
        "each statement on its own line",
        "statement",
    ),
    ("ABAP-CALL-001", "one parameter per line", "call"),
    (
        "ABAP-CALL-002",
        "each parameter group on its own line",
        "call",
    ),
    (
        "ABAP-CALL-003",
        "each parameter group on its own line",
        "call",
    ),
    (
        "ABAP-VALUE-001",
        "one row per line",
        "constructor expression",
    ),
    (
        "ABAP-VALUE-002",
        "iteration and rows on their own lines",
        "constructor expression",
    ),
    (
        "ABAP-COND-001",
        "condition split before AND, OR",
        "condition",
    ),
    (
        "ABAP-COND-002",
        "each WHEN branch on its own line",
        "conditional expression",
    ),
    ("ABAP-DECL-001", "one declaration per line", "declaration"),
    ("ABAP-CHAIN-001", "one item per line", "chain"),
    ("ABAP-ALIGN-005", "one test method per line", "chain"),
    ("ABAP-ALIGN-006", "one redefinition per line", "chain"),
    ("ABAP-ALIGN-007", "one alias per line", "chain"),
    ("ABAP-ALIGN-009", "one item per line", "statement"),
    ("ABAP-ALIGN-010", "one key component per line", "statement"),
    (
        "ABAP-METHODS-001",
        "each section and parameter on its own line",
        "method signature",
    ),
    (
        "ABAP-FORM-001",
        "each section and parameter on its own line",
        "FORM signature",
    ),
    ("ABAP-SQL-001", "each clause on its own line", "statement"),
    ("ABAP-SQL-003", "one field per line", "field list"),
    (
        "ABAP-SQL-002",
        "each common table expression on its own line",
        "query",
    ),
    (
        "ABAP-EML-001",
        "each entity and operation on its own line",
        "statement",
    ),
    (
        "CDS-UNIT-001",
        "each annotation and element on its own line",
        "element",
    ),
    (
        "CDS-ANNO-001",
        "annotation values on their own lines",
        "annotation",
    ),
    (
        "CDS-HEAD-001",
        "each clause of the header on its own line",
        "header",
    ),
    (
        "CDS-ELEM-001",
        "each WHEN on its own line",
        "case expression",
    ),
    (
        "CDS-CLAUSE-001",
        "each query clause on its own line",
        "query",
    ),
    (
        "DCL-GRANT-001",
        "condition split before AND, OR",
        "access condition",
    ),
    (
        "CDS-ALIGN-002",
        "each parameter of the data source on its own line",
        "data source",
    ),
    (
        "CDS-ALIGN-003",
        "each function parameter on its own line",
        "function call",
    ),
    (
        "CDS-ALIGN-004",
        "each name of the list on its own line",
        "list",
    ),
];

/// Rules that break whatever the width; all other rules break a construct
/// only when it does not fit.
const ALWAYS_BREAKS: &[RuleId] = &[
    "ABAP-STMT-001",
    "ABAP-ALIGN-005",
    "ABAP-ALIGN-006",
    "ABAP-ALIGN-007",
    "ABAP-ALIGN-009",
    "ABAP-ALIGN-010",
    "ABAP-SQL-002",
    "CDS-UNIT-001",
    "CDS-HEAD-001",
    "CDS-ELEM-001",
    "CDS-CLAUSE-001",
    "CDS-ALIGN-002",
    "CDS-ALIGN-003",
    "CDS-ALIGN-004",
];

/// Rule phrase and construct noun of `rule`.
pub(crate) fn rule_phrase(rule: RuleId) -> Option<(&'static str, &'static str)> {
    RULE_PHRASES
        .iter()
        .find(|(r, ..)| *r == rule)
        .map(|&(_, phrase, noun)| (phrase, noun))
}

fn sentence(rule: Option<RuleId>, cause: &BreakCause, noun: &str) -> String {
    let ruled = rule.and_then(|r| rule_phrase(r).map(|(phrase, n)| (r, phrase, n)));
    match (cause, ruled) {
        (
            BreakCause::Width {
                column,
                limit,
                scope,
            },
            ruled,
        ) => {
            let what = match scope {
                Scope::Construct => format!("the {}", ruled.map_or(noun, |r| r.2)),
                Scope::Line => "the line".to_string(),
            };
            match ruled {
                Some((r, phrase, _)) => format!(
                    "{r}: {phrase}, because {what} would end at column {column} (limit {limit})"
                ),
                None => {
                    format!("split because {what} would end at column {column} (limit {limit})")
                }
            }
        }
        (BreakCause::Rule, Some((r, phrase, _))) => format!("{r}: {phrase}"),
        (BreakCause::Rule, None) => format!("line break required by the layout of this {noun}"),
        (BreakCause::Comment { text }, _) => {
            format!("split because of the comment `{text}`")
        }
        (BreakCause::Layout, _) => "laid out on several lines to fit the layout rules".to_string(),
    }
}

// ---- classification ---------------------------------------------------------

/// For each token: whether it is the first token on its line (`k > 0`).
fn line_starts(text: &str, stream: &TokenStream) -> Vec<bool> {
    let t = &stream.tokens;
    (0..t.len())
        .map(|k| k > 0 && text[t[k - 1].end as usize..t[k].start as usize].contains('\n'))
        .collect()
}

/// 1-based line of each token's start.
fn token_lines(text: &str, stream: &TokenStream) -> Vec<u32> {
    let mut line = 1u32;
    let mut at = 0usize;
    stream
        .tokens
        .iter()
        .map(|t| {
            let s = t.start as usize;
            line += text[at..s].bytes().filter(|&b| b == b'\n').count() as u32;
            at = s;
            line
        })
        .collect()
}

/// Column (1-based, in characters) of the last character of token `k`.
fn end_column(text: &str, stream: &TokenStream, k: usize) -> u32 {
    let end = stream.tokens[k].end as usize;
    let line_start = text[..end].rfind('\n').map_or(0, |i| i + 1);
    text[line_start..end].chars().count() as u32
}

/// Length in characters of the line that holds token `k` (without trailing
/// blanks).
fn line_length(text: &str, stream: &TokenStream, k: usize) -> u32 {
    let at = stream.tokens[k].start as usize;
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    text[line_start..line_end].trim_end().chars().count() as u32
}

/// Length of the widest line that holds tokens `first..=last` (`starts`:
/// which tokens start a line).
fn widest_line(
    text: &str,
    stream: &TokenStream,
    starts: &[bool],
    first: usize,
    last: usize,
) -> Option<u32> {
    (first..=last)
        .filter(|&k| k == first || starts[k])
        .map(|k| line_length(text, stream, k))
        .max()
}

/// The end-of-line comment that trails token `k` in `text`, if any.
fn trailing_comment(text: &str, stream: &TokenStream, k: usize) -> Option<String> {
    stream.trivia.iter().find_map(|t| {
        let eol = matches!(
            t.kind,
            TriviaKind::Comment {
                placement: Placement::EndOfLine,
                ..
            }
        );
        (eol && t.position == TriviaPosition::Trailing && t.anchor.0 as usize == k)
            .then(|| text[t.start as usize..t.end as usize].trim().to_string())
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Construct {
    /// Index into the rule spans.
    Span(usize),
    Unit(UnitId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Rule,
    Comment,
    Width,
}

/// A note and the output lines (1-based) of its break tokens.
struct Located {
    note: Note,
    break_lines: Vec<u32>,
}

fn notes(parts: &Parts, opts: &Options) -> Vec<Located> {
    let Parts {
        src, lang, checked, ..
    } = parts;
    let r = &checked.result;
    let output = r.output.as_str();
    let wide = check::print_wide(src, *lang, opts, parts.identity, checked);
    let (Ok(out_stream), Ok(wide_stream)) =
        (check::relex(*lang, output), check::relex(*lang, &wide))
    else {
        return Vec::new();
    };
    let n = r.input_tokens.tokens.len();
    if out_stream.tokens.len() != n || wide_stream.tokens.len() != n {
        return Vec::new();
    }
    let in_start = line_starts(src.text(), &r.input_tokens);
    let out_start = line_starts(output, &out_stream);
    let wide_start = line_starts(&wide, &wide_stream);
    let skipped: BTreeSet<UnitId> = checked.quarantine.union(&checked.opaque).copied().collect();
    let unit_rule = if *lang == Language::Abap {
        "ABAP-STMT-001"
    } else {
        "CDS-UNIT-001"
    };
    let noun = if *lang == Language::Abap {
        "statement"
    } else {
        "element"
    };

    // Introduced breaks, grouped.
    let mut groups: BTreeMap<(Construct, Kind, Option<RuleId>), Vec<TokenIndex>> = BTreeMap::new();
    let mut comments: BTreeMap<TokenIndex, String> = BTreeMap::new();
    for k in 1..n {
        if !out_start[k] || in_start[k] {
            continue;
        }
        let ti = TokenIndex(k as u32);
        let Some(unit) = r.unit_containing(ti) else {
            continue;
        };
        if skipped.contains(&unit.id) {
            continue;
        }
        let kind = if !wide_start[k] {
            Kind::Width
        } else if let Some(c) = trailing_comment(output, &out_stream, k - 1) {
            comments.insert(ti, c);
            Kind::Comment
        } else {
            Kind::Rule
        };
        // Innermost span: fewest tokens; for equal ranges the last pushed.
        let span = r
            .rule_spans
            .iter()
            .enumerate()
            .filter(|(_, s)| s.contains_break(ti))
            .min_by_key(|(i, s)| (s.len(), usize::MAX - i))
            .map(|(i, s)| (i, s.rule));
        let (construct, rule) = match span {
            Some((i, rule)) => (Construct::Span(i), Some(rule)),
            None if unit.tokens.start == ti => (Construct::Unit(unit.id), Some(unit_rule)),
            None => (Construct::Unit(unit.id), None),
        };
        let rule = if kind == Kind::Comment { None } else { rule };
        groups.entry((construct, kind, rule)).or_default().push(ti);
    }

    let out_lines = token_lines(output, &out_stream);
    let limit = opts.lang.line_width;
    let mut notes: Vec<(u32, Kind, Located)> = groups
        .into_iter()
        .map(|((construct, kind, rule), break_tokens)| {
            let (first, last) = match construct {
                Construct::Span(i) => {
                    let s = &r.rule_spans[i];
                    (s.first.0 as usize, s.last.0 as usize)
                }
                Construct::Unit(id) => {
                    let u = &r.units[id as usize];
                    (u.tokens.start.0 as usize, u.tokens.end.0 as usize - 1)
                }
            };
            // An end-of-line comment inside the construct (not after its
            // last token) forces the construct onto several lines.
            let inner_comment =
                || (first..last).find_map(|j| trailing_comment(output, &out_stream, j));
            let (cause, rule) = match kind {
                // A rule that breaks only for width cannot cause a break that
                // survives the wide print; a comment inside does.
                Kind::Rule if rule.is_none_or(|r| !ALWAYS_BREAKS.contains(&r)) => {
                    match inner_comment() {
                        Some(text) => (BreakCause::Comment { text }, None),
                        None => (BreakCause::Rule, rule),
                    }
                }
                Kind::Rule => (BreakCause::Rule, rule),
                Kind::Comment => (
                    BreakCause::Comment {
                        text: comments[&break_tokens[0]].clone(),
                    },
                    None,
                ),
                Kind::Width => {
                    let column = end_column(&wide, &wide_stream, last);
                    let line = line_length(&wide, &wide_stream, last);
                    if column > limit as u32 {
                        (
                            BreakCause::Width {
                                column,
                                limit,
                                scope: Scope::Construct,
                            },
                            rule,
                        )
                    } else if line > limit as u32 {
                        (
                            BreakCause::Width {
                                column: line,
                                limit,
                                scope: Scope::Line,
                            },
                            rule,
                        )
                    } else if let Some(widest) =
                        widest_line(&wide, &wide_stream, &wide_start, first, last)
                            .filter(|&w| w > limit as u32)
                    {
                        // A layout that is chosen only when every line fits
                        // (ABAP-COND-002): its widest line does not.
                        (
                            BreakCause::Width {
                                column: widest,
                                limit,
                                scope: Scope::Line,
                            },
                            rule,
                        )
                    } else if let Some(text) = inner_comment() {
                        (BreakCause::Comment { text }, None)
                    } else {
                        (BreakCause::Layout, None)
                    }
                }
            };
            let text = sentence(rule, &cause, noun);
            let break_lines = break_tokens
                .iter()
                .map(|t| out_lines[t.0 as usize])
                .collect();
            let note = Note {
                lines: out_lines[first]..=out_lines[last],
                break_tokens,
                rule,
                cause,
                text,
            };
            (out_lines[first], kind, Located { note, break_lines })
        })
        .collect();
    notes.sort_by_key(|(line, kind, l)| (*line, *l.note.lines.end(), *kind));
    notes.into_iter().map(|(_, _, l)| l).collect()
}

/// With `--lines`: keeps the notes whose breaks all lie in kept changes and
/// moves their lines to the partly formatted text.
fn select(notes: Vec<Located>, map: &LineMap) -> Vec<Located> {
    let kept = |l: u32| {
        map.kept_change
            .get(l as usize - 1)
            .copied()
            .unwrap_or(false)
    };
    let to_mixed = |l: u32| {
        map.mixed
            .get(l as usize - 1)
            .copied()
            .flatten()
            .map(|m| m + 1)
    };
    notes
        .into_iter()
        .filter_map(|mut l| {
            if !l.break_lines.iter().all(|&b| kept(b)) {
                return None;
            }
            // The construct's first or last line may be outside the kept
            // text; then the break lines bound it.
            let breaks = || l.break_lines.iter().filter_map(|&b| to_mixed(b));
            let first = to_mixed(*l.note.lines.start()).or_else(|| breaks().min())?;
            let last = to_mixed(*l.note.lines.end()).or_else(|| breaks().max())?;
            l.note.lines = first..=last.max(first);
            Some(l)
        })
        .collect()
}
