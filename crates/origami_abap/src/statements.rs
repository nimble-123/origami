//! Dispatch from a statement to its parser.
//!
//! * Assignments and standalone calls (`stmt.rs`).
//! * Declarations `DATA`, `TYPES`, `CONSTANTS`, `CLASS-DATA`, `STATICS`,
//!   `FIELD-SYMBOLS`, including chains (ABAP-DECL-001, ABAP-ALIGN-003).
//! * Method signatures `METHODS`, `CLASS-METHODS` (ABAP-METHODS-001).
//! * Keyword statements described by a [`Spec`]: leading keywords, clause
//!   keywords (a broken statement breaks before them at the continuation
//!   indent), other keywords, and operands or logical expressions in between.
//!
//! Anything a parser does not fully understand returns `None` and the
//! statement falls back to the generic layout.

use crate::kinds::*;
use crate::stmt::P;
use origami_layout::{AlignItem, Doc, builder as b};

pub fn parse(p: &mut P) -> Option<Doc> {
    let start = p.pos;
    let w0 = p.upper(start);
    let call_like = p.glued(start + 1) && matches!(p.kind(start + 1), LPAREN | SELECTOR | LBRACKET);
    if !call_like {
        if DECL_LEADS.contains(&w0.as_str()) {
            if let Some(d) = declaration(p) {
                return Some(d);
            }
            p.pos = start;
            return None;
        }
        if (w0 == "READ" || w0 == "MODIFY")
            && (p.is(start + 1, "ENTITIES") || p.is(start + 1, "ENTITY"))
        {
            let saved = p.c.keywords.len();
            if let Some(d) = eml(p) {
                return Some(d);
            }
            p.c.keywords.truncate(saved);
            p.pos = start;
            return None;
        }
        if w0 == "METHODS" || w0 == "CLASS-METHODS" {
            let r = methods(p);
            if r.is_none() {
                p.pos = start;
            }
            return r;
        }
        for spec in SPECS {
            if matches_words(p, start, spec.lead) {
                let saved = p.c.keywords.len();
                if let Some(d) = keyword_statement(p, spec) {
                    return Some(d);
                }
                p.c.keywords.truncate(saved);
                p.pos = start;
            }
        }
    }
    p.assignment_or_call()
}

/// How the parts between keywords are parsed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Operands {
    Expr,
    Logical,
}

struct Spec {
    /// Leading keywords (space-separated words each).
    lead: &'static [&'static str],
    /// Clause keywords; a broken statement breaks before them.
    clauses: &'static [&'static str],
    /// Other keywords.
    words: &'static [&'static str],
    /// Operands right after the lead.
    after_lead: Operands,
    /// Clauses followed by a logical expression.
    logical_clauses: &'static [&'static str],
    /// The statement may end right after the lead.
    bare: bool,
    /// Commas separate list items (ABAP SQL field lists).
    commas: bool,
    /// A broken statement breaks before every clause (ABAP-SQL-001) instead
    /// of only before clauses that do not fit.
    all_clauses: bool,
}

const fn spec(lead: &'static [&'static str]) -> Spec {
    Spec {
        lead,
        clauses: &[],
        words: &[],
        after_lead: Operands::Expr,
        logical_clauses: &[],
        bare: true,
        commas: false,
        all_clauses: false,
    }
}

const LOGIC_WHERE: &[&str] = &["WHERE"];

static SPECS: &[Spec] = &[
    // Block statements without operands.
    spec(&["ENDMETHOD"]),
    spec(&["ENDIF"]),
    spec(&["ELSE"]),
    spec(&["ENDLOOP"]),
    spec(&["ENDTRY"]),
    spec(&["TRY"]),
    spec(&["ENDCASE"]),
    spec(&["ENDCLASS"]),
    spec(&["ENDINTERFACE"]),
    spec(&["ENDDO"]),
    spec(&["ENDWHILE"]),
    spec(&["ENDSELECT"]),
    spec(&["ENDAT"]),
    spec(&["CLEANUP"]),
    spec(&["CONTINUE"]),
    spec(&["EXIT"]),
    spec(&["PUBLIC", "SECTION"]),
    spec(&["PROTECTED", "SECTION"]),
    spec(&["PRIVATE", "SECTION"]),
    // Statements with operands.
    Spec {
        words: &["INTO", "RESUMING", "BEFORE", "UNWIND"],
        ..spec(&["CATCH"])
    },
    Spec {
        words: &[],
        ..spec(&["METHOD"])
    },
    spec(&["RETURN"]),
    Spec {
        after_lead: Operands::Logical,
        bare: false,
        ..spec(&["IF"])
    },
    Spec {
        after_lead: Operands::Logical,
        bare: false,
        ..spec(&["ELSEIF"])
    },
    Spec {
        after_lead: Operands::Logical,
        bare: false,
        ..spec(&["WHILE"])
    },
    Spec {
        after_lead: Operands::Logical,
        bare: false,
        ..spec(&["CHECK"])
    },
    Spec {
        after_lead: Operands::Logical,
        bare: false,
        ..spec(&["ASSERT"])
    },
    Spec {
        words: &["OR", "OTHERS", "TYPE", "INTO"],
        bare: false,
        ..spec(&["WHEN"])
    },
    Spec {
        words: &["TYPE", "OF"],
        bare: false,
        ..spec(&["CASE"])
    },
    Spec {
        words: &["TIMES"],
        ..spec(&["DO"])
    },
    Spec {
        clauses: &[
            "INTO",
            "ASSIGNING",
            "REFERENCE INTO",
            "TRANSPORTING NO FIELDS",
            "WHERE",
            "FROM",
            "TO",
            "USING KEY",
            "GROUP BY",
            "STEP",
        ],
        words: &[
            "CASTING",
            "ELSE",
            "UNASSIGN",
            "WITHOUT",
            "MEMBERS",
            "ASCENDING",
            "DESCENDING",
            "AS",
            "LOOP",
        ],
        logical_clauses: LOGIC_WHERE,
        bare: false,
        ..spec(&["LOOP", "AT"])
    },
    Spec {
        words: &[],
        bare: false,
        ..spec(&["LOOP", "AT", "GROUP"])
    },
    Spec {
        clauses: &["TO", "REFERENCE INTO", "ASSIGNING", "SORTED BY"],
        words: &[
            "INITIAL", "LINE", "LINES", "OF", "FROM", "CASTING", "ELSE", "UNASSIGN",
        ],
        bare: false,
        ..spec(&["APPEND"])
    },
    Spec {
        clauses: &[
            "INTO TABLE",
            "INTO",
            "INDEX",
            "REFERENCE INTO",
            "ASSIGNING",
            "FROM",
            "TO",
        ],
        words: &[
            "INITIAL", "LINE", "LINES", "OF", "CASTING", "ELSE", "UNASSIGN",
        ],
        bare: false,
        ..spec(&["INSERT"])
    },
    Spec {
        clauses: &[
            "INTO",
            "ASSIGNING",
            "REFERENCE INTO",
            "TRANSPORTING NO FIELDS",
            "TRANSPORTING",
            "WITH TABLE KEY",
            "WITH KEY",
            "INDEX",
            "FROM",
            "USING KEY",
            "BINARY SEARCH",
        ],
        words: &[
            "COMPARING",
            "ALL",
            "FIELDS",
            "CASTING",
            "ELSE",
            "UNASSIGN",
            "COMPONENTS",
        ],
        bare: false,
        ..spec(&["READ", "TABLE"])
    },
    Spec {
        words: &["WITH", "INITIAL"],
        bare: false,
        ..spec(&["CLEAR"])
    },
    Spec {
        words: &[],
        bare: false,
        ..spec(&["FREE"])
    },
    Spec {
        clauses: &["TYPE", "EXPORTING", "MESSAGE"],
        words: &["RESUMABLE", "WITH", "NEW"],
        bare: false,
        ..spec(&["RAISE", "EXCEPTION"])
    },
    Spec {
        clauses: &["TYPE", "EXPORTING", "EXCEPTIONS"],
        words: &["AREA", "HANDLE"],
        bare: false,
        ..spec(&["CREATE", "OBJECT"])
    },
    Spec {
        clauses: &["TO", "CASTING", "ELSE"],
        words: &[
            "COMPONENT",
            "OF",
            "STRUCTURE",
            "TYPE",
            "LIKE",
            "INCREMENT",
            "RANGE",
            "UNASSIGN",
        ],
        bare: false,
        ..spec(&["ASSIGN"])
    },
    Spec {
        words: &[],
        bare: false,
        ..spec(&["UNASSIGN"])
    },
    Spec {
        clauses: &["BY"],
        words: &["ASCENDING", "DESCENDING", "AS", "TEXT", "STABLE"],
        bare: false,
        ..spec(&["SORT"])
    },
    Spec {
        clauses: &["WHERE", "FROM", "TO", "INDEX", "USING KEY", "COMPARING"],
        words: &[
            "TABLE",
            "ADJACENT",
            "DUPLICATES",
            "ALL",
            "FIELDS",
            "WITH",
            "KEY",
        ],
        logical_clauses: LOGIC_WHERE,
        bare: false,
        ..spec(&["DELETE"])
    },
    Spec {
        clauses: &["FROM", "TRANSPORTING", "WHERE", "INDEX", "USING KEY"],
        words: &["TABLE"],
        logical_clauses: LOGIC_WHERE,
        bare: false,
        ..spec(&["MODIFY"])
    },
    Spec {
        words: &["ABSTRACT", "METHODS", "FINAL", "ALL", "DATA", "VALUES"],
        bare: false,
        ..spec(&["INTERFACES"])
    },
    Spec {
        words: &["FOR"],
        bare: false,
        ..spec(&["ALIASES"])
    },
    Spec {
        clauses: &[
            "INHERITING FROM",
            "CREATE",
            "FOR TESTING",
            "RISK LEVEL",
            "DURATION",
            "FRIENDS",
            "GLOBAL FRIENDS",
        ],
        words: &[
            "DEFINITION",
            "IMPLEMENTATION",
            "PUBLIC",
            "ABSTRACT",
            "FINAL",
            "PROTECTED",
            "PRIVATE",
            "HARMLESS",
            "DANGEROUS",
            "CRITICAL",
            "SHORT",
            "MEDIUM",
            "LONG",
            "DEFERRED",
            "LOAD",
            "LOCAL",
            "SHARED",
            "MEMORY",
            "ENABLED",
        ],
        bare: false,
        ..spec(&["CLASS"])
    },
    Spec {
        words: &["PUBLIC", "DEFERRED", "LOAD"],
        bare: false,
        ..spec(&["INTERFACE"])
    },
    Spec {
        words: &["OF", "TO", "DEFAULT"],
        bare: false,
        ..spec(&["MESSAGE"])
    },
    Spec {
        words: &[],
        bare: false,
        ..spec(&["CONDENSE"])
    },
    Spec {
        words: &["TO", "UPPER", "LOWER", "CASE"],
        bare: false,
        ..spec(&["TRANSLATE"])
    },
    Spec {
        clauses: &["INTO", "SEPARATED BY"],
        words: &[
            "LINES",
            "OF",
            "RESPECTING",
            "BLANKS",
            "IN",
            "CHARACTER",
            "BYTE",
            "MODE",
        ],
        bare: false,
        ..spec(&["CONCATENATE"])
    },
    Spec {
        clauses: &["AT", "INTO"],
        words: &["TABLE", "IN", "CHARACTER", "BYTE", "MODE"],
        bare: false,
        ..spec(&["SPLIT"])
    },
    Spec {
        words: &["FOR", "ABSTRACT", "FINAL", "REDEFINITION", "TESTING"],
        bare: false,
        ..spec(&["EVENTS"])
    },
    Spec {
        words: &["EXPORTING"],
        bare: false,
        ..spec(&["RAISE", "EVENT"])
    },
    Spec {
        words: &["FOR", "ACTIVATION", "ALL", "INSTANCES"],
        bare: false,
        ..spec(&["SET", "HANDLER"])
    },
    Spec {
        words: &["STAMP", "FIELD"],
        bare: false,
        ..spec(&["GET", "TIME"])
    },
    Spec {
        words: &[],
        bare: false,
        ..spec(&["COMMIT", "WORK"])
    },
    Spec {
        words: &[],
        bare: false,
        ..spec(&["ROLLBACK", "WORK"])
    },
    Spec {
        words: &[],
        bare: false,
        ..spec(&["INCLUDE"])
    },
    Spec {
        clauses: &["RESPONSE OF", "FAILED", "REPORTED", "MAPPED"],
        words: &["IN", "SIMULATION", "MODE"],
        ..spec(&["COMMIT", "ENTITIES"])
    },
    spec(&["ROLLBACK", "ENTITIES"]),
    Spec {
        clauses: &["ENTITY", "FROM", "REQUEST", "RESULT", "FAILED", "REPORTED"],
        words: &[
            "ONLY",
            "GLOBAL",
            "INSTANCE",
            "AUTHORIZATION",
            "FEATURES",
            "OF",
            "IN",
            "LOCAL",
            "MODE",
        ],
        bare: false,
        ..spec(&["GET", "PERMISSIONS"])
    },
    Spec {
        clauses: &["ENTITY", "FROM", "FAILED", "REPORTED"],
        words: &["OF", "IN", "LOCAL", "MODE"],
        bare: false,
        ..spec(&["SET", "LOCKS"])
    },
    Spec {
        clauses: &["FROM"],
        bare: false,
        ..spec(&["RAISE", "ENTITY", "EVENT"])
    },
    Spec {
        clauses: &[
            "FROM",
            "FIELDS",
            "WHERE",
            "GROUP BY",
            "HAVING",
            "ORDER BY",
            "INTO",
            "APPENDING",
            "UP TO",
            "FOR ALL ENTRIES IN",
            "INNER JOIN",
            "LEFT OUTER JOIN",
            "RIGHT OUTER JOIN",
            "CROSS JOIN",
            "UNION",
            "PACKAGE SIZE",
            "OFFSET",
        ],
        words: &[
            "SINGLE",
            "DISTINCT",
            "AS",
            "TABLE",
            "CORRESPONDING",
            "FIELDS",
            "OF",
            "ROWS",
            "ON",
            "ASCENDING",
            "DESCENDING",
            "PRIMARY",
            "KEY",
            "NEW",
            "BYPASSING",
            "BUFFER",
            "CLIENT",
            "SPECIFIED",
            "USING",
            "ALL",
            "ENTRIES",
            "IN",
            "FOR",
            "UPDATE",
            "INDICATORS",
            "NULL",
        ],
        logical_clauses: &["WHERE", "HAVING"],
        bare: false,
        commas: true,
        all_clauses: true,
        ..spec(&["SELECT"])
    },
];

fn matches_words(p: &P, at: usize, words: &[&str]) -> bool {
    words.iter().enumerate().all(|(k, w)| {
        let i = at + k;
        p.kind(i) == WORD && p.text(i).eq_ignore_ascii_case(w) && (k == 0 || !p.glued(i))
    })
}

/// Longest clause or word phrase at `at`, as a number of words.
fn phrase_at(p: &P, at: usize, phrases: &[&str]) -> usize {
    phrases
        .iter()
        .map(|ph| ph.split(' ').collect::<Vec<_>>())
        .filter(|ws| !p.glued(at) && matches_words(p, at, ws) && !operand_follows_glued(p, at))
        .map(|ws| ws.len())
        .max()
        .unwrap_or(0)
}

/// A keyword candidate directly followed by a glued `(`, `->` or `-x` is a
/// name, not a keyword.
fn operand_follows_glued(p: &P, at: usize) -> bool {
    p.glued(at + 1) && matches!(p.kind(at + 1), LPAREN | SELECTOR | LBRACKET)
}

fn keyword_statement(p: &mut P, spec: &Spec) -> Option<Doc> {
    let start = p.pos;
    let mut head = Vec::new();
    for k in 0..spec.lead.len() {
        let i = start + k;
        if k > 0 {
            head.push(p.sp(i));
        }
        head.push(p.kw(i));
    }
    p.pos = start + spec.lead.len();
    if p.at_end() {
        return spec
            .bare
            .then(|| b::concat(vec![b::concat(head), p.period()]));
    }
    // Parts: the first follows the lead on its line; each clause starts one.
    let mut parts: Vec<Doc> = Vec::new();
    let mut current: Vec<Doc> = Vec::new();
    let mut mode = spec.after_lead;
    while !p.at_end() {
        let i = p.pos;
        let n_clause = phrase_at(p, i, spec.clauses);
        let n_word = phrase_at(p, i, spec.words);
        if n_clause > 0 && n_clause >= n_word {
            parts.push(b::concat(std::mem::take(&mut current)));
            current.push(p.ln(i));
            for k in 0..n_clause {
                if k > 0 {
                    current.push(p.sp(i + k));
                }
                current.push(p.kw(i + k));
            }
            let phrase = spec.clauses.iter().find(|c| {
                c.split(' ').count() == n_clause
                    && matches_words(p, i, &c.split(' ').collect::<Vec<_>>())
            });
            mode = if phrase.is_some_and(|c| spec.logical_clauses.contains(c)) {
                Operands::Logical
            } else {
                Operands::Expr
            };
            p.pos = i + n_clause;
            continue;
        }
        if n_word > 0 {
            for k in 0..n_word {
                current.push(p.sp(i + k));
                current.push(p.kw(i + k));
            }
            p.pos = i + n_word;
            continue;
        }
        if spec.commas && p.kind(i) == COMMA {
            current.push(p.sp(i));
            current.push(p.t(i));
            p.pos = i + 1;
            continue;
        }
        // Chain colon or comma: not supported by this engine.
        if matches!(p.kind(i), COLON | COMMA | PRAGMA) {
            return None;
        }
        // `*` as a field list (ABAP SQL).
        if spec.commas && p.kind(i) == WORD && p.text(i) == "*" {
            current.push(p.sp(i));
            current.push(p.t(i));
            p.pos = i + 1;
            continue;
        }
        current.push(p.sp(i));
        // Comparison or key assignment inside a clause (`WITH KEY a = b`).
        if p.kind(i) == WORD
            && mode == Operands::Expr
            && matches!(p.text(i), "=" | "<>" | "<" | ">" | "<=" | ">=")
        {
            current.push(p.t(i));
            p.pos = i + 1;
            continue;
        }
        let operand = match mode {
            Operands::Expr => p.expr()?,
            Operands::Logical => p.logical()?,
        };
        current.push(operand);
    }
    parts.push(b::concat(current));
    let mut docs = vec![b::concat(head)];
    let first = parts.remove(0);
    // Breaks inside the first part continue at the continuation indent.
    docs.push(b::cont(vec![first]));
    if spec.all_clauses {
        docs.push(b::cont(parts));
    } else {
        // Each clause: (separator, content). The separator is the first element.
        let clauses: Vec<(Doc, Doc)> = parts.into_iter().map(split_sep).collect();
        docs.push(b::cont(vec![b::fill(clauses)]));
    }
    docs.push(p.period());
    Some(b::group(docs))
}

/// Splits a clause doc built as `concat([separator, …])` into its parts.
fn split_sep(d: Doc) -> (Doc, Doc) {
    match d.into_parts() {
        Some((sep, rest)) => (sep, rest),
        None => (b::empty(), b::empty()),
    }
}

// ---- declarations ---------------------------------------------------------

const DECL_LEADS: &[&str] = &[
    "DATA",
    "TYPES",
    "CONSTANTS",
    "CLASS-DATA",
    "STATICS",
    "FIELD-SYMBOLS",
];

const DECL_KW: &[&str] = &[
    "TYPE",
    "LIKE",
    "REF",
    "TO",
    "STANDARD",
    "SORTED",
    "HASHED",
    "ANY",
    "INDEX",
    "TABLE",
    "OF",
    "WITH",
    "DEFAULT",
    "NON-UNIQUE",
    "UNIQUE",
    "KEY",
    "EMPTY",
    "COMPONENTS",
    "RANGE",
    "LINE",
    "VALUE",
    "READ-ONLY",
    "BEGIN",
    "END",
    "INITIAL",
    "SIZE",
    "IS",
    "LENGTH",
    "DECIMALS",
    "BOXED",
    "ALIAS",
    "PRIMARY_KEY",
    "STRUCTURE",
    "ENUM",
    "BASE",
    "INCLUDE",
    "AS",
    "RENAMING",
    "SUFFIX",
    "OCCURS",
    "HEADER",
    "SECONDARY",
    "OPTIONAL",
];

/// Keywords whose next word is always an operand (a name or a type).
fn takes_operand(w: &str) -> bool {
    matches!(
        w,
        "TYPE"
            | "LIKE"
            | "TO"
            | "OF"
            | "VALUE"
            | "LENGTH"
            | "DECIMALS"
            | "SIZE"
            | "KEY"
            | "ALIAS"
            | "SUFFIX"
            | "BASE"
            | "STRUCTURE"
    )
}

/// After a keyword that takes an operand, these still count as keywords.
fn keyword_after(prev: &str, w: &str) -> bool {
    match prev {
        "TYPE" | "LIKE" => matches!(
            w,
            "REF"
                | "STANDARD"
                | "SORTED"
                | "HASHED"
                | "ANY"
                | "INDEX"
                | "TABLE"
                | "RANGE"
                | "LINE"
                | "BEGIN"
                | "END"
                | "ENUM"
                | "STRUCTURE"
        ),
        "VALUE" => w == "IS",
        "KEY" => matches!(w, "COMPONENTS" | "ALIAS"),
        "OF" => matches!(w, "STRUCTURE"),
        _ => false,
    }
}

/// Lays out tokens `from..to` of a declaration item flat, marking keywords.
fn decl_tokens(p: &mut P, from: usize, to: usize) -> Vec<Doc> {
    let mut docs = Vec::new();
    let mut prev_kw: Option<String> = None;
    let mut in_components = false;
    for i in from..to {
        if i > from {
            docs.push(p.sp(i));
        }
        let w = p.upper(i);
        let is_word = p.kind(i) == WORD && !p.glued(i);
        let operand_slot = prev_kw.as_deref().is_some_and(takes_operand)
            && !prev_kw.as_deref().is_some_and(|pk| keyword_after(pk, &w));
        // After COMPONENTS, words are field names unless they start the
        // next key clause.
        let ends_components = matches!(
            w.as_str(),
            "WITH" | "UNIQUE" | "NON-UNIQUE" | "DEFAULT" | "EMPTY" | "KEY"
        );
        let keyword_here = !in_components || ends_components;
        if is_word && !operand_slot && DECL_KW.contains(&w.as_str()) && keyword_here {
            docs.push(p.kw(i));
            in_components = w == "COMPONENTS";
            prev_kw = Some(w);
        } else {
            docs.push(p.t(i));
            if is_word {
                prev_kw = None;
            }
        }
    }
    docs
}

/// End of the item's name: the first token plus glued tokens after it.
fn name_end(p: &P, from: usize, to: usize) -> usize {
    let mut e = from + 1;
    while e < to && p.glued(e) && !matches!(p.kind(e), COMMA) {
        e += 1;
    }
    e
}

enum Line {
    /// An aligned row: name, then the rest starting with a keyword.
    Row(Vec<Doc>, Vec<Doc>, bool),
    /// A line without an anchor.
    Plain(Doc),
}

fn declaration(p: &mut P) -> Option<Doc> {
    let start = p.pos;
    let lead = p.kw(start);
    let chain = p.kind(start + 1) == COLON;
    if !chain {
        if start + 1 >= p.end {
            return None;
        }
        let ne = name_end(p, start + 1, p.end);
        let mut docs = vec![lead, p.sp(start + 1)];
        for i in start + 1..ne {
            if i > start + 1 {
                docs.push(p.sp(i));
            }
            docs.push(p.t(i));
        }
        docs.extend(
            decl_tokens(p, ne, p.end)
                .into_iter()
                .enumerate()
                .map(|(k, d)| {
                    if k == 0 {
                        b::concat(vec![p.sp(ne), d])
                    } else {
                        d
                    }
                }),
        );
        docs.push(p.period());
        p.pos = p.end;
        return Some(b::group(docs));
    }
    let colon = start + 1;
    // Split items at commas on bracket depth 0.
    let mut items = Vec::new();
    let mut item_start = colon + 1;
    let mut depth = 0i32;
    for i in colon + 1..p.end {
        match p.kind(i) {
            LPAREN | LBRACKET => depth += 1,
            RPAREN | RBRACKET => depth -= 1,
            COMMA if depth == 0 => {
                items.push((item_start, i));
                item_start = i + 1;
            }
            COLON | PRAGMA => return None,
            _ => {}
        }
    }
    if item_start < p.end {
        items.push((item_start, p.end));
    }
    // Items must be separable: an item glued to the previous comma cannot be
    // moved to its own line.
    if items.is_empty() || items.iter().any(|&(a, e)| a >= e || p.glued(a)) {
        return None;
    }
    let mut lines: Vec<(u32, Line, usize)> = Vec::new(); // (level, line, first token)
    let mut level = 0u32;
    let mut forced = false;
    for (n, &(a, e)) in items.iter().enumerate() {
        let comma = if e < p.end && p.kind(e) == COMMA {
            Some(e)
        } else {
            None
        };
        let begin = p.is(a, "BEGIN") && p.is(a + 1, "OF");
        let end = p.is(a, "END") && p.is(a + 1, "OF");
        if end {
            level = level.checked_sub(1)?;
        }
        let mut tail_end = vec![];
        if let Some(c) = comma {
            tail_end.push(p.sp(c));
            tail_end.push(p.t(c));
        }
        let line = if begin || end {
            forced = true;
            let mut d = decl_tokens(p, a, e);
            d.extend(tail_end);
            Line::Plain(b::concat(d))
        } else {
            let ne = name_end(p, a, e);
            let mut head = Vec::new();
            for i in a..ne {
                if i > a {
                    head.push(p.sp(i));
                }
                head.push(p.t(i));
            }
            if ne < e {
                let mut tail = vec![p.sp(ne)];
                tail.extend(decl_tokens(p, ne, e));
                tail.extend(tail_end);
                Line::Row(head, tail, n > 0 && p.c.commented[a])
            } else {
                head.extend(tail_end);
                Line::Plain(b::concat(head))
            }
        };
        lines.push((level, line, a));
        if begin {
            level += 1;
        }
    }
    if level != 0 {
        return None;
    }
    // Group consecutive rows of the same level into alignment groups.
    let mut docs: Vec<Doc> = Vec::new();
    let mut k = 0;
    while k < lines.len() {
        let (lvl, _, first_tok) = lines[k];
        let mut content: Vec<Doc>;
        if matches!(lines[k].1, Line::Row(..)) {
            let mut items: Vec<AlignItem> = Vec::new();
            while k < lines.len() && lines[k].0 == lvl && matches!(lines[k].1, Line::Row(..)) {
                let Line::Row(head, tail, brk) =
                    std::mem::replace(&mut lines[k].1, Line::Plain(b::empty()))
                else {
                    unreachable!()
                };
                if brk {
                    items.push(AlignItem::Break);
                }
                items.push(b::row(head, tail));
                k += 1;
            }
            content = vec![b::align(items)];
        } else {
            let Line::Plain(d) = std::mem::replace(&mut lines[k].1, Line::Plain(b::empty())) else {
                unreachable!()
            };
            content = vec![d];
            k += 1;
        }
        if !docs.is_empty() {
            let sep = if forced {
                p.hl(first_tok)
            } else {
                p.ln(first_tok)
            };
            content.insert(0, sep);
        }
        docs.push((0..lvl).fold(b::concat(content), |d, _| b::indent(vec![d])));
    }
    p.pos = p.end;
    let body = b::hang(docs);
    Some(b::group(vec![
        lead,
        p.sp(colon),
        p.t(colon),
        p.sp(colon + 1),
        body,
        p.period(),
    ]))
}

// ---- method signatures ----------------------------------------------------

const SECTIONS: &[&str] = &[
    "IMPORTING",
    "EXPORTING",
    "CHANGING",
    "RETURNING",
    "RAISING",
    "EXCEPTIONS",
];

fn is_section(p: &P, i: usize) -> bool {
    // A parameter may be named like a section keyword (`IMPORTING importing TYPE i`).
    p.kind(i) == WORD
        && !p.glued(i)
        && SECTIONS.contains(&p.upper(i).as_str())
        && !operand_follows_glued(p, i)
        && !(p.is(i + 1, "TYPE") || p.is(i + 1, "LIKE"))
}

/// Start of a parameter: `name TYPE|LIKE`, or `VALUE(name)`/`REFERENCE(name)`
/// followed by `TYPE|LIKE`.
fn param_start(p: &P, i: usize) -> bool {
    if p.kind(i) != WORD || p.glued(i) {
        return false;
    }
    let mut j = i + 1;
    if p.kind(j) == LPAREN && p.glued(j) {
        match p.matching(j) {
            Some(close) => j = close + 1,
            None => return false,
        }
    }
    p.is(j, "TYPE") || p.is(j, "LIKE")
}

fn methods(p: &mut P) -> Option<Doc> {
    let start = p.pos;
    if p.kind(start + 1) == COLON {
        return None;
    }
    let lead = p.kw(start);
    let name = start + 1;
    if p.kind(name) != WORD {
        return None;
    }
    let mut first_line = vec![lead, p.sp(name), p.t(name)];
    let mut i = name + 1;
    // Header words up to the first section keyword (FOR TESTING, ABSTRACT,
    // RAP handler phrases …), printed as written.
    while i < p.end && !is_section(p, i) {
        first_line.push(p.sp(i));
        let w = p.upper(i);
        let simple = matches!(
            w.as_str(),
            "ABSTRACT" | "FINAL" | "REDEFINITION" | "TESTING" | "IGNORE" | "FAIL"
        );
        first_line.push(if simple && p.kind(i) == WORD {
            p.kw(i)
        } else {
            p.t(i)
        });
        i += 1;
    }
    let mut sections = Vec::new();
    while i < p.end {
        if !is_section(p, i) {
            return None;
        }
        let kw = p.kw(i);
        let mut j = i + 1;
        let section_end = (j..p.end).find(|&k| is_section(p, k)).unwrap_or(p.end);
        // Split the section's content into parameters.
        let mut params: Vec<(usize, usize)> = Vec::new();
        let raising = p.is(i, "RAISING");
        if raising {
            while j < section_end {
                let mut e = j + 1;
                while e < section_end && p.glued(e) {
                    e += 1;
                }
                params.push((j, e));
                j = e;
            }
        } else if param_start(p, j) {
            let mut s = j;
            j += 1;
            while j < section_end {
                if param_start(p, j) {
                    params.push((s, j));
                    s = j;
                }
                j += 1;
            }
            params.push((s, section_end));
        } else if j < section_end {
            params.push((j, section_end));
        }
        let mut param_docs = Vec::new();
        for (a, e) in params {
            let mut d = vec![p.ln(a)];
            let ne = name_end(p, a, e);
            for k in a..ne {
                if k > a {
                    d.push(p.sp(k));
                }
                // VALUE( / REFERENCE( are keywords.
                let w = p.upper(k);
                if k == a && (w == "VALUE" || w == "REFERENCE") && p.kind(k + 1) == LPAREN {
                    d.push(p.kw(k));
                } else {
                    d.push(p.t(k));
                }
            }
            if ne < e {
                d.push(p.sp(ne));
                d.extend(decl_tokens(p, ne, e));
                // OPTIONAL / DEFAULT inside parameters.
            }
            param_docs.push(b::concat(d));
        }
        sections.push(b::concat(vec![p.ln(i), kw, b::indent(param_docs)]));
        i = section_end;
    }
    p.pos = p.end;
    let mut docs = vec![b::concat(first_line)];
    docs.push(b::cont(sections));
    docs.push(p.period());
    Some(b::group(docs))
}

// ---- EML ------------------------------------------------------------------

/// A line of a broken EML statement: its level below the statement
/// (1 = `ENTITY` and responses, 2 = operations), first token, content, and
/// data segments (`WITH …`, `FROM …`) that break only when they do not fit.
struct EmlLine {
    level: u32,
    first: usize,
    content: Vec<Doc>,
    data: Vec<(Doc, Doc)>,
}

fn take_kw(p: &mut P, head: &mut Vec<Doc>) {
    let i = p.pos;
    head.push(p.sp(i));
    head.push(p.kw(i));
    p.pos += 1;
}

/// `READ ENTITIES OF … | READ ENTITY … | MODIFY ENTITIES OF … | MODIFY ENTITY …`
/// (ABAP-EML-001). After every keyword that takes an operand, the next token
/// is an operand, so `REPORTED reported` works.
fn eml(p: &mut P) -> Option<Doc> {
    let start = p.pos;
    let mut head = vec![p.kw(start), p.sp(start + 1), p.kw(start + 1)];
    let plural = p.is(start + 1, "ENTITIES");
    p.pos = start + 2;
    if plural {
        if !p.is(p.pos, "OF") {
            return None;
        }
        take_kw(p, &mut head);
        head.push(p.sp(p.pos));
        head.push(p.operand()?);
    }
    loop {
        if p.is(p.pos, "IN") && p.is(p.pos + 1, "LOCAL") && p.is(p.pos + 2, "MODE") {
            for _ in 0..3 {
                take_kw(p, &mut head);
            }
        } else if p.is(p.pos, "PRIVILEGED") {
            take_kw(p, &mut head);
        } else {
            break;
        }
    }
    if !plural {
        head.push(p.sp(p.pos));
        head.push(p.operand()?);
    }
    let op_level = if plural { 2 } else { 1 };
    let mut lines: Vec<EmlLine> = Vec::new();
    let mut op_open = false;
    let mut op_has_data = false;
    while !p.at_end() {
        let i = p.pos;
        if p.glued(i) || p.kind(i) != WORD {
            return None;
        }
        let w = p.upper(i);
        match w.as_str() {
            "ENTITY" if plural => {
                let kw = p.kw(i);
                p.pos += 1;
                let sp = p.sp(p.pos);
                let name = p.operand()?;
                lines.push(EmlLine {
                    level: 1,
                    first: i,
                    content: vec![kw, sp, name],
                    data: vec![],
                });
                op_open = false;
            }
            "FAILED" | "REPORTED" | "MAPPED" | "RESULT" | "LINK" => {
                let level = if matches!(w.as_str(), "RESULT" | "LINK") {
                    op_level
                } else {
                    1
                };
                let kw = p.kw(i);
                p.pos += 1;
                let sp = p.sp(p.pos);
                let value = p.operand()?;
                lines.push(EmlLine {
                    level,
                    first: i,
                    content: vec![kw, sp, value],
                    data: vec![],
                });
                op_open = false;
            }
            "WITH" | "FROM" if op_open && !op_has_data => {
                let kw = p.kw(i);
                p.pos += 1;
                let sp = p.sp(p.pos);
                let value = p.expr()?;
                let sep = p.ln(i);
                lines
                    .last_mut()?
                    .data
                    .push((sep, b::concat(vec![kw, sp, value])));
                op_has_data = true;
            }
            "CREATE" | "UPDATE" | "DELETE" | "EXECUTE" | "BY" | "FIELDS" | "ALL" | "FROM"
            | "SET" | "AUTO" => {
                let (n_words, operand) = match w.as_str() {
                    "ALL" if p.is(i + 1, "FIELDS") => (2, false),
                    "SET" if p.is(i + 1, "FIELDS") => (2, false),
                    "AUTO" if p.is(i + 1, "FILL") && p.is(i + 2, "CID") => (3, false),
                    "FIELDS" | "EXECUTE" | "BY" | "FROM" => (1, true),
                    "CREATE" | "UPDATE" | "DELETE" => (1, false),
                    _ => return None,
                };
                if !op_open || op_has_data {
                    lines.push(EmlLine {
                        level: op_level,
                        first: i,
                        content: vec![],
                        data: vec![],
                    });
                    op_open = true;
                    op_has_data = false;
                }
                let line = lines.len() - 1;
                let mut docs = Vec::new();
                for k in 0..n_words {
                    if !lines[line].content.is_empty() || k > 0 {
                        docs.push(p.sp(i + k));
                    }
                    docs.push(p.kw(i + k));
                }
                p.pos = i + n_words;
                if operand {
                    docs.push(p.sp(p.pos));
                    docs.push(p.expr()?);
                    if w == "FROM" {
                        op_has_data = true;
                    }
                }
                lines[line].content.extend(docs);
            }
            _ => return None,
        }
    }
    if lines.is_empty() {
        return None;
    }
    let mut body = Vec::new();
    for l in lines {
        let mut d = vec![p.ln(l.first)];
        d.extend(l.content);
        if !l.data.is_empty() {
            d.push(b::fill(l.data));
        }
        body.push((1..l.level).fold(b::concat(d), |x, _| b::indent(vec![x])));
    }
    Some(b::group(vec![b::concat(head), b::cont(body), p.period()]))
}
