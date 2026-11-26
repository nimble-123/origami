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
use crate::stmt::{AlignKey, DeclCols, P, Row, Stmt, decl_columns};
use origami_layout::{AlignItem, Doc, builder as b};
use origami_syntax::Style;

pub fn parse(p: &mut P) -> Option<Stmt> {
    let start = p.pos;
    // Pragmas before the statement (`##NEEDED foo = bar.`).
    if p.kind(start) == PRAGMA {
        let mut lead = Vec::new();
        let mut k = start;
        while k < p.end && p.kind(k) == PRAGMA {
            if k > start {
                lead.push(p.sp(k));
            }
            lead.push(p.t(k));
            k += 1;
        }
        if k >= p.end || p.glued(k) {
            return None;
        }
        p.pos = k;
        let rest = parse(p)?.into_doc();
        return Some(Stmt::Doc(b::concat(vec![b::concat(lead), p.sp(k), rest])));
    }
    let w0 = p.upper(start);
    let call_like = p.glued(start + 1) && matches!(p.kind(start + 1), LPAREN | SELECTOR | LBRACKET);
    // ABAP has no reserved words: `message = …` assigns to a variable named
    // like the MESSAGE statement.
    let assign_like = !p.glued(start + 1)
        && p.kind(start + 1) == WORD
        && matches!(
            p.text(start + 1),
            "=" | "?=" | "+=" | "-=" | "*=" | "/=" | "&&="
        );
    if !call_like && !assign_like {
        if DECL_LEADS.contains(&w0.as_str()) {
            if let Some(d) = declaration(p) {
                p.c.span(start, p.end, "ABAP-DECL-001");
                return Some(d);
            }
            p.pos = start;
            return None;
        }
        let eml_ops = ["OPERATIONS", "PRIVILEGED", "FORWARDING"]
            .iter()
            .any(|w| p.is(start + 2, w));
        if (w0 == "READ" || w0 == "MODIFY")
            && (p.is(start + 1, "ENTITIES") || p.is(start + 1, "ENTITY"))
            && !eml_ops
        {
            let saved = p.c.mark();
            if let Some(d) = eml(p) {
                p.c.span(start, p.end, "ABAP-EML-001");
                return Some(Stmt::Doc(d));
            }
            p.c.reset(saved);
            p.pos = start;
            return None;
        }
        // `CALL METHOD OF obj 'name'` (OLE) is a keyword statement.
        if w0 == "CALL"
            && (p.is(start + 1, "FUNCTION") || p.is(start + 1, "METHOD"))
            && !(p.is(start + 1, "METHOD") && p.is(start + 2, "OF"))
        {
            let saved = p.c.mark();
            if let Some(d) = call_with_groups(p) {
                p.c.span(start, p.end, "ABAP-CALL-003");
                return Some(Stmt::Doc(d));
            }
            p.c.reset(saved);
            p.pos = start;
            return None;
        }
        if w0 == "OPEN" && p.is(start + 1, "CURSOR") {
            let saved = p.c.mark();
            if let Some(d) = open_cursor(p) {
                return Some(Stmt::Doc(d));
            }
            p.c.reset(saved);
            p.pos = start;
        }
        if w0 == "WITH" && p.text(start + 1).starts_with('+') {
            let saved = p.c.mark();
            if let Some(d) = with_select(p) {
                return Some(Stmt::Doc(d));
            }
            p.c.reset(saved);
            p.pos = start;
        }
        if w0 == "FORM" && p.kind(start + 1) == WORD {
            let saved = p.c.mark();
            let lead = p.kw(start);
            if let Some(item) = method_item(p, start + 1, p.end, FORM_SECTIONS) {
                p.pos = p.end;
                return Some(Stmt::Doc(b::group(vec![
                    lead,
                    p.sp(start + 1),
                    item,
                    p.period(),
                ])));
            }
            p.c.reset(saved);
            p.pos = start;
        }
        if w0 == "ALIASES" {
            let saved = p.c.mark();
            if let Some(s) = aliases(p) {
                return Some(s);
            }
            p.c.reset(saved);
            p.pos = start;
        }
        if w0 == "METHODS" || w0 == "CLASS-METHODS" {
            let r = methods(p);
            if r.is_none() {
                p.pos = start;
            }
            return r;
        }
        for spec in SPECS {
            // `RAISE x.` (classic) is not `RAISE EXCEPTION …` and friends.
            if spec.lead == ["RAISE"]
                && matches!(
                    p.upper(start + 1).as_str(),
                    "EXCEPTION" | "EVENT" | "SHORTDUMP" | "ENTITY" | "RESUMABLE"
                )
            {
                continue;
            }
            // `LOOP AT GROUP grp …` is not `LOOP AT itab …`.
            if spec.lead == ["LOOP", "AT"]
                && p.is(start + 2, "GROUP")
                && p.kind(start + 3) == WORD
                && !p.glued(start + 3)
                && !matches!(
                    p.upper(start + 3).as_str(),
                    "INTO"
                        | "ASSIGNING"
                        | "REFERENCE"
                        | "TRANSPORTING"
                        | "WHERE"
                        | "USING"
                        | "FROM"
                        | "TO"
                        | "GROUP"
                        | "CASTING"
                )
            {
                continue;
            }
            if matches_words(p, start, spec.lead) {
                let saved = p.c.mark();
                if let Some(d) = keyword_statement(p, spec) {
                    return Some(Stmt::Doc(d));
                }
                p.c.reset(saved);
                p.pos = start;
            }
        }
    }
    p.assignment_or_call()
}

/// Words that are keywords even as the only operand of their statement
/// (`WHEN OTHERS.`, `LOOP AT SCREEN.`), where another word would be a name.
const FIRST_OPERAND_KEYWORDS: &[(&[&str], &str)] = &[
    (&["SELECT"], "SINGLE"),
    (&["SELECT"], "DISTINCT"),
    (&["WHEN"], "OTHERS"),
    (&["LOOP", "AT"], "SCREEN"),
    (&["MODIFY"], "SCREEN"),
    (&["AT", "SELECTION-SCREEN"], "OUTPUT"),
    (&["SELECTION-SCREEN"], "SKIP"),
    (&["SELECTION-SCREEN"], "ULINE"),
];

/// How the parts between keywords are parsed.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Operands {
    Expr,
    Logical,
}

#[derive(Clone, Copy)]
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

const CONVERT_WORDS: &[&str] = &[
    "DATE",
    "TIME STAMP",
    "TIME",
    "UTCLONG",
    "DAYLIGHT SAVING TIME",
    "FRACTIONAL SECONDS",
];

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
    spec(&["ENDCATCH"]),
    Spec {
        words: &["INTO"],
        ..spec(&["CLEANUP"])
    },
    spec(&["CONTINUE"]),
    spec(&["EXIT"]),
    spec(&["PUBLIC", "SECTION"]),
    spec(&["PROTECTED", "SECTION"]),
    spec(&["PRIVATE", "SECTION"]),
    // Statements with operands.
    // Classic `CATCH SYSTEM-EXCEPTIONS exc = n … OTHERS = n.` (before
    // `CATCH`, which would take SYSTEM-EXCEPTIONS as an exception class).
    Spec {
        bare: false,
        ..spec(&["CATCH", "SYSTEM-EXCEPTIONS"])
    },
    Spec {
        words: &["INTO", "RESUMING", "BEFORE", "UNWIND"],
        ..spec(&["CATCH"])
    },
    Spec {
        // AMDP: `METHOD m BY DATABASE PROCEDURE FOR HDB LANGUAGE SQLSCRIPT …`
        words: &[
            "BY DATABASE PROCEDURE",
            "BY DATABASE FUNCTION",
            "BY DATABASE GRAPH WORKSPACE",
            "FOR",
            "HDB",
            "LANGUAGE",
            "SQLSCRIPT",
            "OPTIONS",
            "READ-ONLY",
            "DETERMINISTIC",
            "SUPPRESS SYNTAX ERRORS",
            "USING",
        ],
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
            // Table controls in dynpro flow logic.
            "WITH CONTROL",
            "CURSOR",
            // `LOOP AT SCREEN`
            "SCREEN",
        ],
        logical_clauses: LOGIC_WHERE,
        bare: false,
        ..spec(&["LOOP", "AT"])
    },
    Spec {
        clauses: &["INTO", "ASSIGNING", "REFERENCE INTO", "WHERE", "GROUP BY"],
        words: &[
            "CASTING",
            "ELSE",
            "UNASSIGN",
            "ASCENDING",
            "DESCENDING",
            "WITHOUT",
            "MEMBERS",
            "AS",
        ],
        logical_clauses: LOGIC_WHERE,
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
            "INITIAL",
            "LINE",
            "LINES",
            "OF",
            "CASTING",
            "ELSE",
            "UNASSIGN",
            "TABLE",
            "ACCEPTING DUPLICATE KEYS",
            // `INSERT REPORT prog FROM itab STATE 'I'`, `INSERT TEXTPOOL …`
            "REPORT",
            "TEXTPOOL",
            "STATE",
            "LANGUAGE",
            "EXTENSION TYPE",
            "PROGRAM TYPE",
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
            "WHERE",
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
        logical_clauses: LOGIC_WHERE,
        bare: false,
        ..spec(&["READ", "TABLE"])
    },
    Spec {
        words: &["WITH", "INITIAL"],
        bare: false,
        ..spec(&["CLEAR"])
    },
    Spec {
        words: &["MEMORY ID", "MEMORY", "OBJECT"],
        bare: false,
        ..spec(&["FREE"])
    },
    Spec {
        clauses: &["TYPE", "EXPORTING", "MESSAGE"],
        words: &["WITH", "NEW", "ID", "NUMBER"],
        bare: false,
        ..spec(&["RAISE", "SHORTDUMP"])
    },
    Spec {
        words: &[
            "TYPE",
            "LIKE",
            "REF",
            "TO",
            "TABLE",
            "OF",
            "LINE",
            "STANDARD",
            "SORTED",
            "HASHED",
            "HANDLE",
            "LENGTH",
            "DECIMALS",
            "VALUE",
            "WITH",
            "DEFAULT",
            "KEY",
            "EMPTY",
            "UNIQUE",
            "NON-UNIQUE",
            "RANGE",
            "AREA",
        ],
        bare: false,
        ..spec(&["CREATE", "DATA"])
    },
    Spec {
        clauses: &["ID"],
        words: &["FIELD", "DUMMY", "FOR", "USER"],
        bare: false,
        ..spec(&["AUTHORITY-CHECK", "OBJECT"])
    },
    Spec {
        clauses: &[
            "RESULTS",
            "SUBMATCHES",
            "MATCH COUNT",
            "MATCH OFFSET",
            "MATCH LENGTH",
            "MATCH LINE",
        ],
        words: &[
            "FIRST",
            "ALL",
            "OCCURRENCE",
            "OCCURRENCES",
            "OF",
            "SUBSTRING",
            "REGEX",
            "PCRE",
            "IN",
            "SECTION",
            "OFFSET",
            "LENGTH",
            "TABLE",
            "IGNORING",
            "RESPECTING",
            "CASE",
            "BYTE",
            "CHARACTER",
            "MODE",
            "FROM",
            "TO",
        ],
        bare: false,
        ..spec(&["FIND"])
    },
    Spec {
        clauses: &[
            "WITH",
            "REPLACEMENT COUNT",
            "REPLACEMENT OFFSET",
            "REPLACEMENT LENGTH",
            "RESULTS",
        ],
        words: &[
            "FIRST",
            "ALL",
            "OCCURRENCE",
            "OCCURRENCES",
            "OF",
            "SUBSTRING",
            "REGEX",
            "PCRE",
            "IN",
            "SECTION",
            "OFFSET",
            "LENGTH",
            "TABLE",
            "IGNORING",
            "RESPECTING",
            "CASE",
            "BYTE",
            "CHARACTER",
            "MODE",
            "FROM",
            "TO",
        ],
        bare: false,
        ..spec(&["REPLACE"])
    },
    Spec {
        clauses: &["SOURCE", "RESULT", "OPTIONS", "PARAMETERS", "OBJECTS"],
        words: &["XML", "JSON"],
        bare: false,
        ..spec(&["CALL", "TRANSFORMATION"])
    },
    Spec {
        clauses: &["SET", "WHERE", "FROM"],
        words: &[
            "TABLE",
            "CLIENT SPECIFIED",
            "USING CLIENT",
            "USING ALL CLIENTS",
            "CONNECTION",
        ],
        logical_clauses: LOGIC_WHERE,
        bare: false,
        commas: true,
        ..spec(&["UPDATE"])
    },
    Spec {
        clauses: &["TYPE", "EXPORTING", "MESSAGE"],
        words: &["RESUMABLE", "WITH", "NEW", "ID", "NUMBER", "USING MESSAGE"],
        bare: false,
        ..spec(&["RAISE", "EXCEPTION"])
    },
    Spec {
        clauses: &["TYPE", "EXPORTING", "MESSAGE"],
        words: &["WITH", "NEW", "ID", "NUMBER"],
        bare: false,
        ..spec(&["RAISE", "RESUMABLE", "EXCEPTION"])
    },
    Spec {
        clauses: &["TYPE", "EXPORTING", "EXCEPTIONS"],
        words: &["AREA", "HANDLE", "FOR TESTING"],
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
        clauses: &[
            "WHERE",
            "FROM",
            "TO",
            "INDEX",
            "USING KEY",
            "COMPARING",
            "ORDER BY",
            "UP TO",
            "OFFSET",
        ],
        words: &[
            "TABLE",
            "ADJACENT",
            "DUPLICATES",
            // `DELETE REPORT prog`, `DELETE TEXTPOOL prog LANGUAGE l`
            "REPORT",
            "TEXTPOOL",
            "LANGUAGE",
            "STATE",
            "ALL",
            "FIELDS",
            "WITH",
            "KEY",
            "ROWS",
            "ASCENDING",
            "DESCENDING",
            "PRIMARY KEY",
            "CLIENT SPECIFIED",
            "USING CLIENT",
            "CONNECTION",
        ],
        logical_clauses: LOGIC_WHERE,
        bare: false,
        commas: true,
        ..spec(&["DELETE"])
    },
    Spec {
        clauses: &["FROM", "TRANSPORTING", "WHERE", "INDEX", "USING KEY"],
        words: &["TABLE", "SCREEN"],
        logical_clauses: LOGIC_WHERE,
        bare: false,
        ..spec(&["MODIFY"])
    },
    Spec {
        words: &[
            "ABSTRACT",
            "METHODS",
            "FINAL",
            "ALL",
            "DATA",
            "VALUES",
            "PARTIALLY IMPLEMENTED",
        ],
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
            "FOR BEHAVIOR OF",
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
        words: &[
            "OF",
            "TO",
            "DEFAULT",
            "ID",
            "TYPE",
            "NUMBER",
            "WITH",
            "INTO",
            "RAISING",
            "DISPLAY LIKE",
        ],
        bare: false,
        ..spec(&["MESSAGE"])
    },
    Spec {
        words: &["NO-GAPS"],
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
        words: &[
            "FOR",
            "ABSTRACT",
            "FINAL",
            "REDEFINITION",
            "TESTING",
            "EXPORTING",
            "VALUE",
            "TYPE",
            "LIKE",
            "REF TO",
            "OPTIONAL",
            "DEFAULT",
        ],
        bare: false,
        ..spec(&["EVENTS"])
    },
    Spec {
        words: &["EXPORTING"],
        bare: false,
        ..spec(&["RAISE", "EVENT"])
    },
    spec(&["END-TEST-SEAM"]),
    spec(&["END-TEST-INJECTION"]),
    Spec {
        bare: false,
        ..spec(&["TEST-SEAM"])
    },
    Spec {
        bare: false,
        ..spec(&["TEST-INJECTION"])
    },
    Spec {
        words: &[
            "BY",
            "PLACES",
            "LEFT",
            "RIGHT",
            "CIRCULAR",
            "UP",
            "TO",
            "DELETING",
            "LEADING",
            "TRAILING",
            "IN",
            "CHARACTER",
            "BYTE",
            "MODE",
        ],
        bare: false,
        ..spec(&["SHIFT"])
    },
    Spec {
        clauses: &["TO"],
        words: &["EXPANDING NESTED TABLES", "KEEPING TARGET LINES"],
        bare: false,
        ..spec(&["MOVE-CORRESPONDING"])
    },
    Spec {
        words: &["SECONDS"],
        bare: false,
        ..spec(&["WAIT", "UP", "TO"])
    },
    Spec {
        clauses: &["INTO", "TIME ZONE"],
        words: CONVERT_WORDS,
        bare: false,
        ..spec(&["CONVERT", "DATE"])
    },
    Spec {
        clauses: &["INTO", "TIME ZONE"],
        words: CONVERT_WORDS,
        bare: false,
        ..spec(&["CONVERT", "TIME", "STAMP"])
    },
    Spec {
        clauses: &["INTO", "TIME ZONE"],
        words: CONVERT_WORDS,
        bare: false,
        ..spec(&["CONVERT", "UTCLONG"])
    },
    Spec {
        words: &["FOR", "ACTIVATION", "ALL", "INSTANCES"],
        bare: false,
        ..spec(&["SET", "HANDLER"])
    },
    Spec {
        words: &["STAMP", "FIELD"],
        ..spec(&["GET", "TIME"])
    },
    Spec {
        words: &["AND WAIT"],
        ..spec(&["COMMIT", "WORK"])
    },
    Spec {
        ..spec(&["ROLLBACK", "WORK"])
    },
    Spec {
        words: &[
            "TYPE",
            "STRUCTURE",
            "AS",
            "RENAMING WITH SUFFIX",
            "IF FOUND",
        ],
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
            // One clause: `FIELDS` here is not the `FIELDS` clause.
            "INTO CORRESPONDING FIELDS OF",
            "APPENDING CORRESPONDING FIELDS OF",
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
            "STRUCTURE",
        ],
        logical_clauses: &["WHERE", "HAVING"],
        bare: false,
        commas: true,
        all_clauses: true,
        ..spec(&["SELECT"])
    },
    // ---- classic ABAP: programs, events, selection screens, lists --------
    Spec {
        words: PROGRAM_WORDS,
        ..spec(&["REPORT"])
    },
    Spec {
        words: PROGRAM_WORDS,
        ..spec(&["PROGRAM"])
    },
    Spec {
        words: PROGRAM_WORDS,
        bare: false,
        ..spec(&["FUNCTION-POOL"])
    },
    Spec {
        bare: false,
        ..spec(&["TABLES"])
    },
    Spec {
        bare: false,
        ..spec(&["TYPE-POOLS"])
    },
    spec(&["LOAD-OF-PROGRAM"]),
    spec(&["INITIALIZATION"]),
    spec(&["START-OF-SELECTION"]),
    spec(&["END-OF-SELECTION"]),
    spec(&["END-OF-PAGE"]),
    Spec {
        words: &["DURING LINE-SELECTION"],
        ..spec(&["TOP-OF-PAGE"])
    },
    Spec {
        words: &[
            "OUTPUT",
            "ON",
            "END OF",
            "BLOCK",
            "RADIOBUTTON GROUP",
            "VALUE-REQUEST",
            "HELP-REQUEST",
            "FOR",
            "EXIT-COMMAND",
        ],
        ..spec(&["AT", "SELECTION-SCREEN"])
    },
    spec(&["AT", "LINE-SELECTION"]),
    spec(&["AT", "USER-COMMAND"]),
    spec(&["AT", "FIRST"]),
    spec(&["AT", "LAST"]),
    Spec {
        bare: false,
        ..spec(&["AT", "NEW"])
    },
    Spec {
        bare: false,
        ..spec(&["AT", "END", "OF"])
    },
    spec(&["SUM"]),
    Spec {
        words: PARAMETER_WORDS,
        bare: false,
        ..spec(&["PARAMETERS"])
    },
    Spec {
        words: PARAMETER_WORDS,
        bare: false,
        ..spec(&["PARAMETER"])
    },
    Spec {
        words: PARAMETER_WORDS,
        bare: false,
        ..spec(&["SELECT-OPTIONS"])
    },
    Spec {
        words: &[
            "BEGIN OF",
            "END OF",
            "BLOCK",
            "LINE",
            "SCREEN",
            "TABBED BLOCK",
            "WITH FRAME",
            "TITLE",
            "NO INTERVALS",
            "AS SUBSCREEN",
            "AS WINDOW",
            "COMMENT",
            "FOR FIELD",
            "POSITION",
            "SKIP",
            "ULINE",
            "PUSHBUTTON",
            "USER-COMMAND",
            "MODIF ID",
            "VISIBLE LENGTH",
            "TAB",
            "DEFAULT SCREEN",
            "DEFAULT PROGRAM",
            "FUNCTION KEY",
            "INCLUDE",
            "PARAMETERS",
            "SELECT-OPTIONS",
            "BLOCKS",
            "NESTING LEVEL",
            "LINES",
            "SKIP",
            "ULINE",
        ],
        bare: false,
        ..spec(&["SELECTION-SCREEN"])
    },
    Spec {
        clauses: &["TABLES", "USING", "CHANGING", "RAISING"],
        words: FORM_WORDS,
        bare: false,
        ..spec(&["FORM"])
    },
    spec(&["ENDFORM"]),
    Spec {
        clauses: &["TABLES", "USING", "CHANGING"],
        words: &[
            "IN PROGRAM",
            "IF FOUND",
            "ON COMMIT",
            "ON ROLLBACK",
            "LEVEL",
        ],
        bare: false,
        ..spec(&["PERFORM"])
    },
    Spec {
        bare: false,
        ..spec(&["FUNCTION"])
    },
    spec(&["ENDFUNCTION"]),
    Spec {
        words: &[
            "OUTPUT",
            "INPUT",
            // Calls in dynpro flow logic.
            "ON INPUT",
            "ON REQUEST",
            "ON CHAIN-INPUT",
            "ON CHAIN-REQUEST",
            "ON *-INPUT",
            "AT EXIT-COMMAND",
            "AT CURSOR-SELECTION",
            "SWITCH",
        ],
        bare: false,
        ..spec(&["MODULE"])
    },
    // ---- dynpro flow logic ----------------------------------------------
    spec(&["PROCESS", "BEFORE", "OUTPUT"]),
    spec(&["PROCESS", "AFTER", "INPUT"]),
    spec(&["PROCESS", "ON", "HELP-REQUEST"]),
    spec(&["PROCESS", "ON", "VALUE-REQUEST"]),
    spec(&["CHAIN"]),
    spec(&["ENDCHAIN"]),
    Spec {
        clauses: &["MODULE", "SELECT", "WHERE", "INTO", "WHENEVER NOT FOUND"],
        words: &[
            "ON INPUT",
            "ON REQUEST",
            "ON CHAIN-INPUT",
            "ON CHAIN-REQUEST",
            "ON *-INPUT",
            "AT CURSOR-SELECTION",
            "WITH",
            "VALUES",
            "BETWEEN",
            "AND",
            "NOT",
            "FROM",
            "SEND ERRORMESSAGE",
            "SEND WARNING",
            "*",
        ],
        bare: false,
        ..spec(&["FIELD"])
    },
    Spec {
        words: &["WITH CONTROL", "CURSOR"],
        ..spec(&["LOOP"])
    },
    spec(&["ENDMODULE"]),
    Spec {
        words: WRITE_WORDS,
        bare: false,
        ..spec(&["WRITE"])
    },
    Spec {
        words: &["AT", "/"],
        ..spec(&["ULINE"])
    },
    spec(&["SKIP"]),
    Spec {
        words: &["TO LINE"],
        ..spec(&["SKIP"])
    },
    Spec {
        words: &["NO-SCROLLING", "SCROLLING"],
        ..spec(&["NEW-LINE"])
    },
    Spec {
        words: &[
            "NO-TITLE",
            "WITH-TITLE",
            "NO-HEADING",
            "WITH-HEADING",
            "LINE-SIZE",
            "LINE-COUNT",
        ],
        ..spec(&["NEW-PAGE"])
    },
    Spec {
        words: &["LINES"],
        bare: false,
        ..spec(&["RESERVE"])
    },
    spec(&["BACK"]),
    Spec {
        bare: false,
        ..spec(&["POSITION"])
    },
    Spec {
        words: &[
            "COLOR",
            "INTENSIFIED",
            "INVERSE",
            "HOTSPOT",
            "INPUT",
            "FRAMES",
            "ON",
            "OFF",
            "RESET",
            "COL_BACKGROUND",
            "COL_HEADING",
            "COL_NORMAL",
            "COL_TOTAL",
            "COL_KEY",
            "COL_POSITIVE",
            "COL_NEGATIVE",
            "COL_GROUP",
        ],
        ..spec(&["FORMAT"])
    },
    Spec {
        bare: false,
        ..spec(&["HIDE"])
    },
    Spec {
        clauses: &["TO"],
        bare: false,
        ..spec(&["ADD"])
    },
    Spec {
        clauses: &["FROM"],
        bare: false,
        ..spec(&["SUBTRACT"])
    },
    Spec {
        clauses: &["BY"],
        bare: false,
        ..spec(&["MULTIPLY"])
    },
    Spec {
        clauses: &["BY"],
        bare: false,
        ..spec(&["DIVIDE"])
    },
    Spec {
        clauses: &["TO"],
        words: &["EXACT"],
        bare: false,
        ..spec(&["MOVE"])
    },
    Spec {
        words: &["LINES", "KIND", "OCCURS"],
        bare: false,
        ..spec(&["DESCRIBE", "TABLE"])
    },
    Spec {
        words: &[
            "TYPE",
            "COMPONENTS",
            "LENGTH",
            "IN CHARACTER MODE",
            "IN BYTE MODE",
            "DECIMALS",
            "OUTPUT-LENGTH",
            "HELP-ID",
            "EDIT MASK",
            "INTO",
        ],
        bare: false,
        ..spec(&["DESCRIBE", "FIELD"])
    },
    Spec {
        words: &["EXCLUDING", "OF PROGRAM", "IMMEDIATELY"],
        bare: false,
        ..spec(&["SET", "PF-STATUS"])
    },
    Spec {
        words: &["OF PROGRAM", "WITH"],
        bare: false,
        ..spec(&["SET", "TITLEBAR"])
    },
    Spec {
        words: &["FIELD"],
        bare: false,
        ..spec(&["SET", "PARAMETER", "ID"])
    },
    Spec {
        words: &["FIELD"],
        bare: false,
        ..spec(&["GET", "PARAMETER", "ID"])
    },
    Spec {
        bare: false,
        ..spec(&["SET", "SCREEN"])
    },
    Spec {
        clauses: &["INTO"],
        bare: false,
        ..spec(&["GET", "REFERENCE", "OF"])
    },
    spec(&["LEAVE", "SCREEN"]),
    Spec {
        words: &["AND SKIP FIRST SCREEN"],
        ..spec(&["LEAVE", "TO", "CURRENT", "TRANSACTION"])
    },
    spec(&["LEAVE", "PROGRAM"]),
    spec(&["LEAVE", "LIST-PROCESSING"]),
    Spec {
        words: &["AND RETURN TO SCREEN"],
        ..spec(&["LEAVE", "TO", "LIST-PROCESSING"])
    },
    Spec {
        bare: false,
        ..spec(&["LEAVE", "TO", "SCREEN"])
    },
    Spec {
        words: &["AND SKIP FIRST SCREEN"],
        bare: false,
        ..spec(&["LEAVE", "TO", "TRANSACTION"])
    },
    Spec {
        clauses: &["STARTING AT", "ENDING AT"],
        bare: false,
        ..spec(&["CALL", "SCREEN"])
    },
    Spec {
        clauses: &["USING", "MESSAGES INTO", "OPTIONS FROM"],
        words: &[
            "WITH AUTHORITY-CHECK",
            "WITHOUT AUTHORITY-CHECK",
            "AND SKIP FIRST SCREEN",
            "MODE",
            "UPDATE",
        ],
        bare: false,
        ..spec(&["CALL", "TRANSACTION"])
    },
    Spec {
        clauses: &[
            "WITH",
            "USING SELECTION-SET",
            "VIA SELECTION-SCREEN",
            "AND RETURN",
            "EXPORTING LIST TO MEMORY",
            "TO SAP-SPOOL",
        ],
        words: &[
            "SIGN",
            "IN",
            "USER",
            "VIA JOB",
            "NUMBER",
            "SPOOL PARAMETERS",
            "WITHOUT SPOOL DYNPRO",
            "LINE-SIZE",
            "LINE-COUNT",
            "SELECTION-TABLE",
            // `WITH p EQ v`, `WITH s BETWEEN a AND b`
            "EQ",
            "NE",
            "CP",
            "NP",
            "GT",
            "GE",
            "LT",
            "LE",
            "BETWEEN",
            "AND",
            "NOT",
        ],
        bare: false,
        ..spec(&["SUBMIT"])
    },
    Spec {
        clauses: &["TO", "FROM"],
        words: &[
            "MEMORY ID",
            "DATA BUFFER",
            "INTERNAL TABLE",
            "DATABASE",
            "SHARED MEMORY",
            "SHARED BUFFER",
            "ID",
        ],
        bare: false,
        ..spec(&["EXPORT"])
    },
    Spec {
        clauses: &["TO", "FROM"],
        words: &[
            "MEMORY ID",
            "DATA BUFFER",
            "INTERNAL TABLE",
            "DATABASE",
            "SHARED MEMORY",
            "SHARED BUFFER",
            "ID",
        ],
        bare: false,
        ..spec(&["IMPORT"])
    },
    Spec {
        words: &["ID"],
        ..spec(&["FREE", "MEMORY"])
    },
    Spec {
        words: &["TYPE", "TABLEVIEW", "TABSTRIP", "USING SCREEN"],
        bare: false,
        ..spec(&["CONTROLS"])
    },
    Spec {
        words: &["FROM TABLE"],
        bare: false,
        ..spec(&["REFRESH"])
    },
    Spec {
        clauses: &["STARTING AT", "ENDING AT", "USING SELECTION-SET"],
        bare: false,
        ..spec(&["CALL", "SELECTION-SCREEN"])
    },
    Spec {
        words: DATASET_WORDS,
        bare: false,
        ..spec(&["OPEN", "DATASET"])
    },
    Spec {
        bare: false,
        ..spec(&["CLOSE", "DATASET"])
    },
    Spec {
        bare: false,
        ..spec(&["DELETE", "DATASET"])
    },
    Spec {
        clauses: &["INTO"],
        words: &["MAXIMUM LENGTH", "ACTUAL LENGTH", "LENGTH"],
        bare: false,
        ..spec(&["READ", "DATASET"])
    },
    Spec {
        clauses: &["TO"],
        words: &["LENGTH", "NO END OF LINE"],
        bare: false,
        ..spec(&["TRANSFER"])
    },
    Spec {
        clauses: &["INTO", "APPENDING"],
        words: &["TABLE", "CORRESPONDING FIELDS OF", "PACKAGE SIZE"],
        bare: false,
        ..spec(&["FETCH", "NEXT", "CURSOR"])
    },
    Spec {
        bare: false,
        ..spec(&["CLOSE", "CURSOR"])
    },
    Spec {
        clauses: &["INTO"],
        words: &["STATE"],
        bare: false,
        ..spec(&["READ", "REPORT"])
    },
    Spec {
        clauses: &["INTO"],
        words: &["LANGUAGE", "STATE"],
        bare: false,
        ..spec(&["READ", "TEXTPOOL"])
    },
    Spec {
        clauses: &["INTO"],
        words: &["OF"],
        bare: false,
        ..spec(&["GET", "BIT"])
    },
    Spec {
        words: &["OF", "TO"],
        bare: false,
        ..spec(&["SET", "BIT"])
    },
    // ---- ABAP SQL MERGE (8.16) and EML operation tables ------------------
    Spec {
        clauses: &[
            "USING",
            "ON",
            "WHEN MATCHED",
            "WHEN NOT MATCHED",
            "WHEN NOT MATCHED BY SOURCE",
        ],
        words: &[
            "AS",
            "THEN",
            "DELETE",
            "UPDATE",
            "ALL",
            "SET",
            "INSERT",
            "VALUES",
            "AND",
            "IS",
            "NOT",
            "NULL",
            "DECLARE CLIENT",
        ],
        logical_clauses: &["ON"],
        bare: false,
        commas: true,
        all_clauses: true,

        ..spec(&["MERGE", "INTO"])
    },
    Spec {
        clauses: &["MAPPED", "FAILED", "REPORTED", "RESULT"],
        words: &["FORWARDING", "PRIVILEGED", "OPERATIONS"],
        bare: false,
        ..spec(&["MODIFY", "ENTITIES"])
    },
    Spec {
        clauses: &["RESULT", "FAILED", "REPORTED"],
        words: &["PRIVILEGED", "OPERATIONS"],
        bare: false,
        ..spec(&["READ", "ENTITIES"])
    },
    // ---- rare and obsolete statements (abaplint statement catalog) --------
    Spec {
        bare: false,
        ..spec(&["ROLLBACK", "CONNECTION"])
    },
    Spec {
        bare: false,
        ..spec(&["COMMIT", "CONNECTION"])
    },
    Spec {
        bare: false,
        ..spec(&["TYPE-POOL"])
    },
    spec(&["SUMMARY"]),
    spec(&["DETAIL"]),
    spec(&["INPUT"]),
    spec(&["RESUME"]),
    spec(&["SUPPRESS", "DIALOG"]),
    Spec {
        words: &["STARTING AT", "ENDING AT"],
        bare: false,
        ..spec(&["WINDOW"])
    },
    Spec {
        words: &[
            "BETWEEN",
            "AND",
            "INTO",
            "IN BYTE MODE",
            "IN CHARACTER MODE",
        ],
        bare: false,
        ..spec(&["DESCRIBE", "DISTANCE"])
    },
    Spec {
        words: &[
            "NUMBER OF LINES",
            "NUMBER OF PAGES",
            "INDEX",
            "LINE",
            "PAGE",
            "LINE-SIZE",
            "LINE-COUNT",
            "LINES",
            "FIRST-LINE",
            "TOP-LINES",
            "TITLE-LINES",
            "HEAD-LINES",
            "END-LINES",
        ],
        bare: false,
        ..spec(&["DESCRIBE", "LIST"])
    },
    Spec {
        clauses: &["INTO"],
        words: &["ASSIGNING", "REFERENCE INTO", "CASTING"],
        bare: false,
        ..spec(&["COLLECT"])
    },
    Spec {
        clauses: &["UP TO"],
        words: &["SECONDS"],
        after_lead: Operands::Logical,
        bare: false,
        ..spec(&["WAIT", "UNTIL"])
    },
    Spec {
        clauses: &["UNTIL", "UP TO"],
        words: &[
            "ASYNCHRONOUS TASKS",
            "MESSAGING CHANNELS",
            "PUSH CHANNELS",
            "SECONDS",
        ],
        logical_clauses: &["UNTIL"],
        bare: false,
        ..spec(&["WAIT", "FOR"])
    },
    Spec {
        words: &["EXACT"],
        bare: false,
        ..spec(&["COMPUTE"])
    },
    Spec {
        clauses: &["INTO", "TIME ZONE"],
        words: CONVERT_WORDS,
        bare: false,
        ..spec(&["CONVERT", "TIME"])
    },
    Spec {
        clauses: &["INTO"],
        words: &["DATE", "INVERTED-DATE"],
        bare: false,
        ..spec(&["CONVERT", "INVERTED-DATE"])
    },
    Spec {
        clauses: &["INTO"],
        words: &["SORTABLE CODE"],
        bare: false,
        ..spec(&["CONVERT", "TEXT"])
    },
    Spec {
        words: &["EXPORTING", "TYPE", "LIKE", "OPTIONAL", "DEFAULT", "REF TO"],
        bare: false,
        ..spec(&["CLASS-EVENTS"])
    },
    Spec {
        clauses: &[
            "TOKENS INTO",
            "STATEMENTS INTO",
            "LEVELS INTO",
            "STRUCTURES INTO",
            "ENHANCEMENTS INTO",
            "KEYWORDS FROM",
            "MESSAGE INTO",
            "INCLUDE INTO",
            "LINE INTO",
            "WORD INTO",
            "OFFSET INTO",
            "OVERFLOW INTO",
            "FRAME PROGRAM FROM",
            "INCLUDE PROGRAM FROM",
            "PROGRAM FROM",
            "WITH ANALYSIS",
            "WITH COMMENTS",
            "WITH INCLUDES",
            "WITHOUT TRMAC",
            "WITH DECLARATIONS",
            "WITH BLOCKS",
            "WITH LIST TOKENIZATION",
            "WITH PRAGMAS",
            "WITH TYPE-POOLS",
            "WITH EXPLICIT ENHANCEMENTS",
            "WITH IMPLICIT ENHANCEMENTS",
            "WITH INACTIVE ENHANCEMENTS",
            "REPLACING",
            "ID",
            "PRESERVING IDENTIFIER ESCAPING",
            "ENHANCEMENT OPTIONS INTO",
        ],
        words: &["TABLE", "INTO", "FROM"],
        bare: false,
        ..spec(&["SCAN", "ABAP-SOURCE"])
    },
    Spec {
        clauses: &[
            "MESSAGE",
            "LINE",
            "WORD",
            "PROGRAM",
            "DIRECTORY ENTRY",
            "INCLUDE",
            "OFFSET",
            "MESSAGE-ID",
            "ID",
            "REPLACING",
            "FRAME ENTRY",
            "TRACE-TABLE",
            "SHORTDUMP-ID",
            "FILTER",
        ],
        words: &["TABLE", "FOR"],
        bare: false,
        ..spec(&["SYNTAX-CHECK", "FOR"])
    },
    Spec {
        words: &[
            "INDEX",
            "TO FIRST PAGE",
            "TO LAST PAGE",
            "TO PAGE",
            "TO COLUMN",
            "LINE",
            "FORWARD",
            "BACKWARD",
            "LEFT",
            "RIGHT",
            "BY",
            "PLACES",
            "PAGES",
        ],
        ..spec(&["SCROLL", "LIST"])
    },
    Spec {
        words: &[
            "INIT",
            "ALLOCATE",
            "ACCEPT",
            "SEND",
            "RECEIVE",
            "DEALLOCATE",
            "ID",
            "DESTINATION",
            "BUFFER",
            "LENGTH",
            "DATAINFO",
            "STATUSINFO",
            "RECEIVED",
            "HOLD",
            "RETURNCODE",
            "TYPE",
        ],
        bare: false,
        ..spec(&["COMMUNICATION"])
    },
    Spec {
        words: &["ID", "AT NEXT APPLICATION STATEMENT"],
        ..spec(&["BREAK-POINT"])
    },
    Spec {
        bare: false,
        ..spec(&["BREAK"])
    },
    Spec {
        words: &["ID", "SUBKEY", "FIELDS"],
        bare: false,
        ..spec(&["LOG-POINT"])
    },
    Spec {
        clauses: &[
            "MESSAGE",
            "INCLUDE",
            "LINE",
            "WORD",
            "OFFSET",
            "MESSAGE-ID",
            "SHORTDUMP-ID",
            "DIRECTORY ENTRY",
            "TRACE-FILE",
            "WITHOUT SELECTION-SCREEN",
        ],
        bare: false,
        ..spec(&["GENERATE", "REPORT"])
    },
    Spec {
        clauses: &[
            "NAME",
            "MESSAGE",
            "INCLUDE",
            "LINE",
            "WORD",
            "OFFSET",
            "MESSAGE-ID",
            "SHORTDUMP-ID",
        ],
        bare: false,
        ..spec(&["GENERATE", "SUBROUTINE", "POOL"])
    },
    Spec {
        clauses: &["ID", "MESSAGE", "LINE", "WORD", "OFFSET", "TRACE-FILE"],
        bare: false,
        ..spec(&["GENERATE", "DYNPRO"])
    },
    Spec {
        words: &[
            "LINE",
            "INDEX",
            "OF PAGE",
            "OF CURRENT PAGE",
            "FIELD VALUE",
            "INTO",
            "LINE VALUE INTO",
            "CURRENT LINE",
        ],
        bare: false,
        ..spec(&["READ", "LINE"])
    },
    Spec {
        words: &["FIELD VALUE", "INTO", "LINE VALUE INTO"],
        ..spec(&["READ", "CURRENT", "LINE"])
    },
    Spec {
        words: &[
            "FIELD",
            "LINE",
            "VALUE",
            "LENGTH",
            "OFFSET",
            "AREA",
            "DISPLAY",
            "MEMORY OFFSET",
        ],
        bare: false,
        ..spec(&["GET", "CURSOR"])
    },
    Spec {
        words: &["FIELD", "LINE", "OFFSET", "DISPLAY OFFSET"],
        bare: false,
        ..spec(&["SET", "CURSOR"])
    },
    Spec {
        words: &["TYPE", "FILTERS", "CONTEXT"],
        bare: false,
        ..spec(&["GET", "BADI"])
    },
    Spec {
        words: &["OF", "NO FLUSH", "QUEUE-ONLY", "EXPORTING"],
        bare: false,
        ..spec(&["GET", "PROPERTY"])
    },
    Spec {
        words: &["OF", "NO FLUSH", "EXPORTING"],
        bare: false,
        ..spec(&["SET", "PROPERTY"])
    },
    Spec {
        words: &["OF", "NO FLUSH", "QUEUE-ONLY", "EXPORTING"],
        bare: false,
        ..spec(&["CALL", "METHOD", "OF"])
    },
    Spec {
        words: &[
            "NAME",
            "OCCURS",
            "MODE",
            "VALID FROM",
            "TO",
            "AS PERSON TABLE",
        ],
        bare: false,
        ..spec(&["INFOTYPES"])
    },
    Spec {
        words: &["FOR", "OCCURS"],
        bare: false,
        ..spec(&["RANGES"])
    },
    Spec {
        words: &[
            "FOR",
            "IN BYTE MODE",
            "IN CHARACTER MODE",
            "STARTING AT",
            "ENDING AT",
            "ABBREVIATED",
            "AND MARK",
        ],
        bare: false,
        ..spec(&["SEARCH"])
    },
    Spec {
        clauses: &["EXPORTING", "IMPORTING", "USING"],
        words: &["FROM", "TO", "AND SKIP FIRST SCREEN", "MODE"],
        bare: false,
        ..spec(&["CALL", "DIALOG"])
    },
    Spec {
        words: &["SPOTS", "STATIC", "INCLUDE BOUND"],
        bare: false,
        ..spec(&["ENHANCEMENT-POINT"])
    },
    Spec {
        words: &[
            "INDEX-LINE",
            "FUNCTION",
            "LINE",
            "POSITION",
            "CPI",
            "LPI",
            "SIZE",
            "COLOR",
            "FONT",
            "LEFT MARGIN",
        ],
        bare: false,
        ..spec(&["PRINT-CONTROL"])
    },
    Spec {
        clauses: &["IMPORTING", "TABLES", "CHANGING", "EXCEPTIONS"],
        words: &["KEEPING TASK", "OTHERS"],
        bare: false,
        ..spec(&["RECEIVE", "RESULTS", "FROM", "FUNCTION"])
    },
    Spec {
        words: &["CLOCK RESOLUTION", "HIGH", "LOW", "ANALYZER", "ON", "OFF"],
        bare: false,
        ..spec(&["SET", "RUN", "TIME"])
    },
    Spec {
        words: &["FIELD"],
        bare: false,
        ..spec(&["GET", "RUN", "TIME"])
    },
    Spec {
        bare: false,
        ..spec(&["SYSTEM-CALL"])
    },
    Spec {
        clauses: &["EXPORTING", "IMPORTING"],
        words: &["CONNECTION", "PARAMETER-TABLE"],
        bare: false,
        ..spec(&["CALL", "DATABASE", "PROCEDURE"])
    },
    Spec {
        words: &["FOR", "TITLE", "DISPLAY-MODE", "BACKUP INTO", "ID"],
        bare: false,
        ..spec(&["EDITOR-CALL"])
    },
    Spec {
        words: &["PROGRAM", "EXCLUDING"],
        bare: false,
        ..spec(&["GET", "PF-STATUS"])
    },
    Spec {
        bare: false,
        ..spec(&["LOCAL"])
    },
    Spec {
        words: &["LANGUAGE", "COUNTRY", "MODIFIER"],
        bare: false,
        ..spec(&["SET", "LOCALE"])
    },
    Spec {
        words: &["LANGUAGE", "COUNTRY", "MODIFIER"],
        bare: false,
        ..spec(&["GET", "LOCALE"])
    },
    Spec {
        bare: false,
        ..spec(&["SET", "MARGIN"])
    },
    Spec {
        words: &["ON", "OFF"],
        bare: false,
        ..spec(&["SET", "BLANK", "LINES"])
    },
    Spec {
        words: &["COLUMN", "LINE"],
        ..spec(&["SET", "LEFT", "SCROLL-BOUNDARY"])
    },
    Spec {
        words: &["POSITION", "END OF FILE", "ATTRIBUTE", "ATTRIBUTES"],
        bare: false,
        ..spec(&["SET", "DATASET"])
    },
    Spec {
        words: &["POSITION", "ATTRIBUTE", "ATTRIBUTES"],
        bare: false,
        ..spec(&["GET", "DATASET"])
    },
    Spec {
        words: &["AT CURRENT POSITION", "AT POSITION"],
        bare: false,
        ..spec(&["TRUNCATE", "DATASET"])
    },
    Spec {
        words: &["ON", "OFF"],
        bare: false,
        ..spec(&["SET", "EXTENDED", "CHECK"])
    },
    Spec {
        bare: false,
        ..spec(&["SET", "COUNTRY"])
    },
    Spec {
        bare: false,
        ..spec(&["SET", "LANGUAGE"])
    },
    spec(&["SET", "UPDATE", "TASK", "LOCAL"]),
    spec(&["SET", "USER-COMMAND"]),
    Spec {
        clauses: &["INTO"],
        words: &["PART"],
        bare: false,
        ..spec(&["LOAD", "REPORT"])
    },
    Spec {
        bare: false,
        ..spec(&["CONTEXTS"])
    },
    Spec {
        words: &["FROM CONTEXT"],
        bare: false,
        ..spec(&["DEMAND"])
    },
    Spec {
        words: &["TO CONTEXT"],
        bare: false,
        ..spec(&["SUPPLY"])
    },
    Spec {
        bare: false,
        ..spec(&["FIELD-GROUPS"])
    },
    Spec {
        bare: false,
        ..spec(&["FIELDS"])
    },
    spec(&["EXTRACT"]),
    Spec {
        bare: false,
        ..spec(&["NODES"])
    },
    Spec {
        bare: false,
        ..spec(&["PUT"])
    },
    Spec {
        words: &["WITH", "USING"],
        bare: false,
        ..spec(&["OVERLAY"])
    },
    Spec {
        words: &["TO"],
        bare: false,
        ..spec(&["PACK"])
    },
    Spec {
        words: &["TO"],
        bare: false,
        ..spec(&["UNPACK"])
    },
    Spec {
        clauses: &["TO"],
        bare: false,
        ..spec(&["ADD-CORRESPONDING"])
    },
    Spec {
        clauses: &["FROM"],
        bare: false,
        ..spec(&["SUBTRACT-CORRESPONDING"])
    },
    Spec {
        clauses: &["BY"],
        bare: false,
        ..spec(&["MULTIPLY-CORRESPONDING"])
    },
    Spec {
        clauses: &["BY"],
        bare: false,
        ..spec(&["DIVIDE-CORRESPONDING"])
    },
    Spec {
        words: &["PRIORITY"],
        bare: false,
        ..spec(&["VERIFICATION-MESSAGE"])
    },
    Spec {
        words: &["INCLUDING"],
        bare: false,
        ..spec(&["CALL", "SUBSCREEN"])
    },
    Spec {
        clauses: &["EXPORTING", "IMPORTING", "CHANGING", "TABLES", "EXCEPTIONS"],
        words: &["OTHERS"],
        bare: false,
        ..spec(&["CALL", "CUSTOMER-FUNCTION"])
    },
    Spec {
        words: &["ID", "FIELD"],
        bare: false,
        ..spec(&["CALL"])
    },
    Spec {
        words: &["LATE", "FIELDS"],
        bare: false,
        ..spec(&["GET"])
    },
    Spec {
        words: &["WITH"],
        bare: false,
        ..spec(&["AT"])
    },
    Spec {
        words: &["AS TEXT", "BY", "ASCENDING", "DESCENDING"],
        ..spec(&["SORT"])
    },
    spec(&["LEAVE"]),
    spec(&["STOP"]),
    spec(&["REJECT"]),
    // Classic exception in a function module (`RAISE not_found.`); the
    // dispatcher skips it before EXCEPTION, EVENT, SHORTDUMP, ENTITY and
    // RESUMABLE.
    Spec {
        bare: false,
        ..spec(&["RAISE"])
    },
];

const DATASET_WORDS: &[&str] = &[
    "FOR INPUT",
    "FOR OUTPUT",
    "FOR APPENDING",
    "FOR UPDATE",
    "IN BINARY MODE",
    "IN TEXT MODE",
    "IN LEGACY BINARY MODE",
    "IN LEGACY TEXT MODE",
    "ENCODING DEFAULT",
    "ENCODING UTF-8",
    "ENCODING NON-UNICODE",
    "SKIPPING BYTE-ORDER MARK",
    "WITH BYTE-ORDER MARK",
    "WITH SMART LINEFEED",
    "WITH NATIVE LINEFEED",
    "WITH UNIX LINEFEED",
    "WITH WINDOWS LINEFEED",
    "IGNORING CONVERSION ERRORS",
    "MESSAGE",
    "AT POSITION",
    "FILTER",
    "TYPE",
];

const PROGRAM_WORDS: &[&str] = &[
    "MESSAGE-ID",
    "LINE-SIZE",
    "LINE-COUNT",
    "NO STANDARD PAGE HEADING",
    "REDUCED FUNCTIONALITY",
    "DEFINING DATABASE",
];

const PARAMETER_WORDS: &[&str] = &[
    "TYPE",
    "LIKE",
    "FOR",
    "DEFAULT",
    "TO",
    "OPTION",
    "SIGN",
    "OBLIGATORY",
    "LOWER CASE",
    "AS CHECKBOX",
    "AS LISTBOX",
    "RADIOBUTTON GROUP",
    "MEMORY ID",
    "MATCHCODE OBJECT",
    "VALUE CHECK",
    "NO-DISPLAY",
    "NO-EXTENSION",
    "NO INTERVALS",
    "VISIBLE LENGTH",
    "MODIF ID",
    "USER-COMMAND",
    "DECIMALS",
    "LENGTH",
    "HELP-REQUEST",
    "VALUE-REQUEST",
    "REF TO",
];

const FORM_WORDS: &[&str] = &[
    "TYPE",
    "LIKE",
    "STRUCTURE",
    "REF TO",
    "STANDARD TABLE",
    "SORTED TABLE",
    "HASHED TABLE",
    "ANY TABLE",
    "INDEX TABLE",
    "TABLE",
    "OF",
];

const WRITE_WORDS: &[&str] = &[
    // Line feed (`WRITE / text.`).
    "/",
    // Colors (`COLOR COL_HEADING`).
    "COL_BACKGROUND",
    "COL_HEADING",
    "COL_NORMAL",
    "COL_TOTAL",
    "COL_KEY",
    "COL_POSITIVE",
    "COL_NEGATIVE",
    "COL_GROUP",
    "AT",
    "NO-GAP",
    "COLOR",
    "INTENSIFIED",
    "INVERSE",
    "HOTSPOT",
    "INPUT",
    "FRAMES",
    "ON",
    "OFF",
    "CENTERED",
    "LEFT-JUSTIFIED",
    "RIGHT-JUSTIFIED",
    "UNDER",
    "USING EDIT MASK",
    "USING NO EDIT MASK",
    "CURRENCY",
    "DECIMALS",
    "NO-SIGN",
    "NO-ZERO",
    "NO-GROUPING",
    "ROUND",
    "UNIT",
    "EXPONENT",
    "TIME ZONE",
    "AS CHECKBOX",
    "AS ICON",
    "AS SYMBOL",
    "AS LINE",
    "QUICKINFO",
    "TO",
    "STYLE",
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
        // `/(80)` in list statements is a line feed with a length.
        .filter(|ws| {
            !p.glued(at)
                && matches_words(p, at, ws)
                && (p.text(at) == "/" || !operand_follows_glued(p, at))
        })
        .map(|ws| ws.len())
        .max()
        .unwrap_or(0)
}

/// A keyword candidate directly followed by a glued `(`, `->` or `-x` is a
/// name, not a keyword.
fn operand_follows_glued(p: &P, at: usize) -> bool {
    p.glued(at + 1) && matches!(p.kind(at + 1), LPAREN | SELECTOR | LBRACKET)
}

/// Whether `INSERT`, `UPDATE`, `MODIFY` or `DELETE` at `start` works on a
/// database table (ABAP-SQL-001) rather than an internal table. ABAP decides
/// by the operand's type; without types, a statement counts as database DML
/// when its form exists only for database tables: every `UPDATE`;
/// `INSERT INTO …` and `INSERT dbtab FROM …`; `DELETE FROM …`;
/// `MODIFY dbtab FROM TABLE …`; or any host variable escaped with `@`.
/// Other forms (`MODIFY x FROM wa`, `DELETE x FROM wa`) stay internal-table
/// statements.
fn database_dml(p: &P, start: usize, spec: &Spec) -> bool {
    let lead = match spec.lead {
        [w] => *w,
        _ => return false,
    };
    if !matches!(lead, "INSERT" | "UPDATE" | "MODIFY" | "DELETE") {
        return false;
    }
    if lead == "UPDATE" || (start + 1..p.end).any(|k| p.text(k).starts_with('@')) {
        return true;
    }
    let operand =
        |k: usize| p.kind(k) == WORD && !p.glued(k) && !spec.words.contains(&p.upper(k).as_str());
    match lead {
        "INSERT" => {
            p.is(start + 1, "INTO")
                || (operand(start + 1)
                    && !p.is(start + 1, "INITIAL")
                    && !p.is(start + 1, "LINES")
                    && p.is(start + 2, "FROM"))
        }
        "DELETE" => p.is(start + 1, "FROM"),
        "MODIFY" => operand(start + 1) && p.is(start + 2, "FROM") && p.is(start + 3, "TABLE"),
        _ => false,
    }
}

/// The lead of a `SELECT` in strict syntax, where `FROM` comes before the
/// field list (`SELECT [SINGLE|DISTINCT] FROM …`); `None` otherwise.
fn strict_select_lead(p: &P, start: usize, spec: &Spec) -> Option<&'static [&'static str]> {
    if spec.lead != ["SELECT"] {
        return None;
    }
    if p.is(start + 1, "FROM") {
        return Some(&["SELECT", "FROM"]);
    }
    if p.is(start + 2, "FROM") {
        if p.is(start + 1, "SINGLE") {
            return Some(&["SELECT", "SINGLE", "FROM"]);
        }
        if p.is(start + 1, "DISTINCT") {
            return Some(&["SELECT", "DISTINCT", "FROM"]);
        }
    }
    None
}

fn keyword_statement(p: &mut P, spec: &Spec) -> Option<Doc> {
    let start = p.pos;
    // Database DML follows ABAP-SQL-001 like `SELECT`.
    let db;
    let spec = if !spec.all_clauses && database_dml(p, start, spec) {
        // `DELETE FROM dbtab` and `INSERT INTO dbtab` keep the target on the
        // first line, like `SELECT … FROM`.
        let lead = match spec.lead {
            ["DELETE"] if p.is(start + 1, "FROM") => &["DELETE", "FROM"][..],
            ["INSERT"] if p.is(start + 1, "INTO") => &["INSERT", "INTO"][..],
            other => other,
        };
        db = Spec {
            lead,
            all_clauses: true,
            ..*spec
        };
        &db
    } else {
        spec
    };
    // Strict syntax `SELECT [SINGLE|DISTINCT] FROM dbtab … FIELDS …`: the
    // table stays on the first line (ABAP-SQL-001).
    let strict;
    let spec = match strict_select_lead(p, start, spec) {
        Some(lead) => {
            strict = Spec { lead, ..*spec };
            &strict
        }
        None => spec,
    };
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
    if p.c.style == Style::Aligned
        && matches!(
            spec.lead,
            ["RAISE", "EXCEPTION"] | ["RAISE", "RESUMABLE", "EXCEPTION"] | ["RAISE", "EVENT"]
        )
    {
        let saved = p.c.mark();
        if let Some(body) = raise_exporting(p, start, spec) {
            return Some(b::concat(vec![b::concat(head), body, p.period()]));
        }
        p.c.reset(saved);
        p.pos = start + spec.lead.len();
    }
    if p.c.style == Style::Aligned
        && (spec.lead == ["READ", "TABLE"] || (spec.lead == ["DELETE"] && p.is(start + 1, "TABLE")))
    {
        let saved = p.c.mark();
        if let Some(body) = with_key(p, start, spec) {
            return Some(b::concat(vec![b::concat(head), body, p.period()]));
        }
        p.c.reset(saved);
        p.pos = start + spec.lead.len();
    }
    if p.c.style == Style::Aligned && matches!(spec.lead, ["SORT"] | ["CATCH"]) {
        let saved = p.c.mark();
        if let Some(body) = one_per_line(p, start, spec) {
            return Some(b::concat(vec![b::concat(head), body, p.period()]));
        }
        p.c.reset(saved);
        p.pos = start + spec.lead.len();
    }
    if p.kind(p.pos) == COLON && !spec.commas {
        return spec_chain(p, spec, b::concat(head));
    }
    // Keywords before the colon belong to every item (`WRITE /: a, b.`).
    if !spec.commas {
        let saved = p.c.mark();
        let mut prefix = Vec::new();
        let mut q = p.pos;
        loop {
            let n = phrase_at(p, q, spec.words);
            if n == 0 {
                break;
            }
            for k in 0..n {
                prefix.push(p.sp(q + k));
                prefix.push(p.kw(q + k));
            }
            q += n;
        }
        if q > p.pos && q < p.end && p.kind(q) == COLON {
            p.pos = q;
            head.extend(prefix);
            return spec_chain(p, spec, b::concat(head));
        }
        p.c.reset(saved);
    }
    let body = spec_body(p, spec, true)?;
    if spec.all_clauses {
        p.c.span(start, p.end, "ABAP-SQL-001");
    }
    if spec.all_clauses || spec.lead.first() == Some(&"SELECT") {
        mark_sql_keywords(p, start, p.end);
    }
    Some(b::group(vec![b::concat(head), body, p.period()]))
}

/// `RAISE EXCEPTION TYPE … EXPORTING a = 1 b = 2`, `RAISE EVENT …
/// EXPORTING …`, aligned style (ABAP-CALL-001): with two or more parameters
/// the line breaks before `EXPORTING`, the parameters aligned behind it.
fn raise_exporting(p: &mut P, start: usize, spec: &Spec) -> Option<Doc> {
    let from = p.pos;
    let exp = (from..p.end).find(|&k| p.is(k, "EXPORTING") && !p.glued(k))?;
    if exp == from || (from..p.end).any(|k| p.c.commented[k]) {
        return None;
    }
    let end = p.end;
    p.end = exp;
    let head = spec_body(p, spec, true);
    p.end = end;
    let head = head?;
    let kw = p.kw(exp);
    let mut rows = Vec::new();
    p.pos = exp + 1;
    while !p.at_end() {
        let name = p.pos;
        if p.kind(name) != WORD || !p.is(name + 1, "=") || p.glued(name + 1) || p.glued(name) {
            return None;
        }
        p.pos = name + 2;
        let value = p.expr()?;
        rows.push(b::row_cols(
            vec![if rows.is_empty() {
                vec![kw.clone()]
            } else {
                Vec::new()
            }],
            vec![p.t(name)],
            vec![p.sp(name + 1), p.t(name + 1), p.sp(name + 2), value],
        ));
    }
    if rows.len() < 2 {
        return None;
    }
    p.c.span(start, p.end, "ABAP-CALL-001");
    Some(b::concat(vec![
        head,
        b::cont(vec![p.hl(exp), b::align_broken(rows)]),
    ]))
}

/// `READ TABLE … WITH [TABLE] KEY [k COMPONENTS] a = 1 b = 2 …` and
/// `DELETE TABLE … WITH TABLE KEY …`, aligned style (ABAP-ALIGN-010): two
/// or more key components one per line, aligned under the first; the
/// additions after the last one stay behind it. `None` for other forms.
fn with_key(p: &mut P, start: usize, spec: &Spec) -> Option<Doc> {
    let from = p.pos;
    if (from..p.end).any(|k| p.c.commented[k] || p.c.eol_commented[k]) {
        return None;
    }
    // `WITH [TABLE] KEY` at bracket depth 0.
    let mut k = from;
    let with = loop {
        if k >= p.end {
            return None;
        }
        match p.kind(k) {
            LPAREN | LBRACKET => k = p.matching(k)? + 1,
            _ if p.is(k, "WITH") && !p.glued(k) => break k,
            _ => k += 1,
        }
    };
    let n_kw = if p.is(with + 1, "TABLE") && p.is(with + 2, "KEY") {
        3
    } else if p.is(with + 1, "KEY") {
        2
    } else {
        return None;
    };
    let mut docs = Vec::new();
    if with > from {
        let end = p.end;
        p.end = with;
        let head = spec_body(p, spec, true);
        p.end = end;
        docs.push(head?);
    }
    for i in with..with + n_kw {
        docs.push(p.sp(i));
        docs.push(p.kw(i));
    }
    k = with + n_kw;
    // `WITH KEY name COMPONENTS`
    if p.kind(k) == WORD && !p.glued(k + 1) && p.is(k + 1, "COMPONENTS") {
        docs.push(p.sp(k));
        docs.push(p.t(k));
        docs.push(p.sp(k + 1));
        docs.push(p.kw(k + 1));
        k += 2;
    }
    // Key components: a name (with glued tokens) and `= value`.
    let row_at = |p: &P, k: usize| -> Option<usize> {
        if p.kind(k) != WORD || p.glued(k) {
            return None;
        }
        let mut e = k + 1;
        while e < p.end && p.glued(e) {
            e += 1;
        }
        (p.is(e, "=") && !p.glued(e)).then_some(e)
    };
    let first = k;
    let mut rows: Vec<AlignItem> = Vec::new();
    while let Some(eq) = row_at(p, k) {
        let mut name = Vec::new();
        for i in k..eq {
            name.push(p.t(i));
        }
        p.pos = eq + 1;
        let value = p.expr()?;
        rows.push(b::row(name, vec![p.sp(eq), p.t(eq), p.sp(eq + 1), value]));
        k = p.pos;
    }
    if rows.len() < 2 {
        return None;
    }
    // The additions after the last component stay on its line.
    if k < p.end {
        p.pos = k;
        let tail = spec_body(p, spec, true)?;
        if let Some(AlignItem::Row(r)) = rows.last_mut() {
            r.tail.push(tail);
        }
    }
    p.pos = p.end;
    p.c.span(start, p.end, "ABAP-ALIGN-010");
    docs.push(p.sp(first));
    docs.push(b::hang(vec![b::align_broken(rows)]));
    Some(b::concat(docs))
}

/// `SORT itab BY a b` and `CATCH cx_a cx_b`, aligned style (ABAP-ALIGN-009):
/// two or more sort fields or exception classes go one per line, aligned
/// under the first. `None` for other forms: one item, a dynamic sort field,
/// a comment inside, `CATCH SYSTEM-EXCEPTIONS`.
fn one_per_line(p: &mut P, start: usize, spec: &Spec) -> Option<Doc> {
    const SORT_WORDS: &[&str] = &["STABLE", "ASCENDING", "DESCENDING"];
    let sort = p.is(start, "SORT");
    if (start + 1..p.end).any(|k| p.c.commented[k]) || p.is(start + 1, "SYSTEM-EXCEPTIONS") {
        return None;
    }
    // A token and the tokens glued to it.
    let unit_end = |p: &P, k: usize| {
        let mut e = k + 1;
        while e < p.end && p.glued(e) {
            e += 1;
        }
        e
    };
    let mut docs = Vec::new();
    let mut k = start + 1;
    // Before the list: `itab [STABLE] [ASCENDING|DESCENDING] [AS TEXT] BY`,
    // `[BEFORE UNWIND]`.
    if sort {
        if !p.is(k, "BY") {
            let e = unit_end(p, k);
            for i in k..e {
                docs.push(p.sp(i));
                docs.push(p.t(i));
            }
            k = e;
        }
        loop {
            let n = if p.is(k, "AS") && p.is(k + 1, "TEXT") {
                2
            } else if SORT_WORDS.iter().any(|w| p.is(k, w)) {
                1
            } else {
                break;
            };
            for i in k..k + n {
                docs.push(p.sp(i));
                docs.push(p.kw(i));
            }
            k += n;
        }
        if !p.is(k, "BY") || p.glued(k + 1) {
            return None;
        }
        docs.push(p.sp(k));
        docs.push(p.kw(k));
        k += 1;
    } else if p.is(k, "BEFORE") && p.is(k + 1, "UNWIND") {
        for i in k..k + 2 {
            docs.push(p.sp(i));
            docs.push(p.kw(i));
        }
        k += 2;
    }
    // The list: one item is a token with its glued tokens, for `SORT` with
    // its `ASCENDING`, `DESCENDING` and `AS TEXT`.
    let mut items: Vec<(usize, Vec<Doc>)> = Vec::new();
    while k < p.end && (sort || !p.is(k, "INTO")) {
        if p.glued(k) || matches!(p.kind(k), LPAREN | RPAREN | LBRACKET | RBRACKET | COMMA) {
            return None;
        }
        let first = k;
        let e = unit_end(p, k);
        let mut d = Vec::new();
        for i in k..e {
            if i > k {
                d.push(p.sp(i));
            }
            d.push(p.t(i));
        }
        k = e;
        loop {
            let n = if !sort {
                break;
            } else if p.is(k, "AS") && p.is(k + 1, "TEXT") {
                2
            } else if p.is(k, "ASCENDING") || p.is(k, "DESCENDING") {
                1
            } else {
                break;
            };
            for i in k..k + n {
                d.push(p.sp(i));
                d.push(p.kw(i));
            }
            k += n;
        }
        items.push((first, d));
    }
    if items.len() < 2 {
        return None;
    }
    let first0 = items[0].0;
    let mut lines = Vec::new();
    for (n, (first, d)) in items.into_iter().enumerate() {
        let mut line = vec![if n == 0 { b::empty() } else { p.hl(first) }];
        line.extend(d);
        lines.push(b::concat(line));
    }
    docs.push(p.sp(first0));
    docs.push(b::hang(lines));
    // `CATCH … INTO oref`
    if k < p.end {
        p.pos = k;
        docs.push(spec_body(p, spec, true)?);
    }
    p.pos = p.end;
    p.c.span(start, p.end, "ABAP-ALIGN-009");
    Some(b::concat(docs))
}

/// The space before token `i`, except before the first token of a chain item
/// (`skip`), whose gap the chain prints.
fn gap(p: &P, i: usize, skip: Option<usize>) -> Doc {
    if skip == Some(i) { b::empty() } else { p.sp(i) }
}

/// Operands and clauses of a keyword statement, from `p.pos` to `p.end`.
fn spec_body(p: &mut P, spec: &Spec, first_gap: bool) -> Option<Doc> {
    let skip = (!first_gap).then_some(p.pos);
    let first_operand = Some(p.pos);
    // Parts: the first follows the lead on its line; each clause starts one.
    let mut parts: Vec<Doc> = Vec::new();
    let mut current: Vec<Doc> = Vec::new();
    let mut mode = spec.after_lead;
    let mut prev_kw: Option<String> = None;
    let mut in_exceptions = spec.lead == ["CATCH", "SYSTEM-EXCEPTIONS"];
    // ABAP-SQL-003: a field list follows `SELECT` (after `SINGLE`,
    // `DISTINCT`, …), `FIELDS`, `GROUP BY` and `ORDER BY`; `INTO ( … )`.
    let select = spec.commas && spec.lead.first() == Some(&"SELECT");
    let mut list_next = select && p.pos > 0 && p.is(p.pos - 1, "SELECT");
    let mut order_by = false;
    let mut into_next = false;
    while !p.at_end() {
        let i = p.pos;
        // `EXCEPTIONS … OTHERS = n`; `EVENTS e EXPORTING VALUE(p) …`
        let event_value = spec.lead == ["EVENTS"]
            && p.is(i, "VALUE")
            && p.glued(i + 1)
            && p.kind(i + 1) == LPAREN;
        if event_value
            || (in_exceptions && p.is(i, "OTHERS") && p.is(i + 1, "=") && !p.glued(i + 1))
        {
            current.push(gap(p, i, skip));
            current.push(p.kw(i));
            p.pos = i + 1;
            continue;
        }
        // A word equal to the keyword just before it is a name
        // (`MATCH COUNT count`, `REPORTED reported`).
        let repeated = prev_kw.as_deref() == Some(p.upper(i).as_str());
        // A first operand followed by the same word is a name
        // (`CREATE OBJECT type TYPE cl_x`).
        let repeated = repeated
            || (Some(i) == first_operand && p.kind(i + 1) == WORD && p.upper(i) == p.upper(i + 1));
        let n_clause = if repeated {
            0
        } else {
            phrase_at(p, i, spec.clauses)
        };
        let mut n_word = if repeated {
            0
        } else {
            phrase_at(p, i, spec.words)
        };
        // The first operand is a name when a clause or the end of the
        // statement follows it (`ASSIGN structure TO <fs>`).
        // `SELECT SINGLE FROM …` / `SELECT DISTINCT FROM …` (strict syntax,
        // field list in FIELDS): these words are never names.
        let select_modifier = FIRST_OPERAND_KEYWORDS
            .iter()
            .any(|(lead, w)| spec.lead == *lead && p.is(i, w));
        if n_word == 1
            && Some(i) == first_operand
            && !select_modifier
            && p.text(i)
                .starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
            && (i + n_word >= p.end || phrase_at(p, i + n_word, spec.clauses) > 0)
        {
            n_word = 0;
        }
        prev_kw = None;
        if n_clause > 0 && n_clause >= n_word {
            parts.push(b::concat(std::mem::take(&mut current)));
            current.push(if skip == Some(i) { b::empty() } else { p.ln(i) });
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
            in_exceptions = phrase == Some(&"EXCEPTIONS");
            list_next = select && matches!(phrase, Some(&("FIELDS" | "GROUP BY" | "ORDER BY")));
            order_by = phrase == Some(&"ORDER BY");
            into_next = select && phrase == Some(&"INTO");
            mode = if phrase.is_some_and(|c| spec.logical_clauses.contains(c)) {
                Operands::Logical
            } else {
                Operands::Expr
            };
            p.pos = i + n_clause;
            prev_kw = Some(p.upper(i + n_clause - 1));
            continue;
        }
        if n_word > 0 {
            for k in 0..n_word {
                current.push(gap(p, i + k, skip));
                current.push(p.kw(i + k));
            }
            p.pos = i + n_word;
            prev_kw = Some(p.upper(i + n_word - 1));
            into_next = false;
            continue;
        }
        let (list_here, into_here) = (
            std::mem::take(&mut list_next),
            std::mem::take(&mut into_next),
        );
        if list_here || into_here {
            let saved = p.c.mark();
            let list = if into_here {
                into_list(p, spec, i)
            } else {
                select_list(p, spec, i, order_by)
            };
            match list {
                Some(d) => {
                    current.push(gap(p, i, skip));
                    current.push(d);
                    continue;
                }
                None => {
                    p.c.reset(saved);
                    p.pos = i;
                }
            }
        }
        if spec.commas && p.kind(i) == COMMA {
            current.push(gap(p, i, skip));
            current.push(p.t(i));
            p.pos = i + 1;
            continue;
        }
        // Chain colon or comma: not supported by this engine.
        if matches!(p.kind(i), COLON | COMMA | PRAGMA) {
            return None;
        }
        // Position and length right after a keyword: `WRITE /(80) …`,
        // `ULINE AT /10(20)`; kept as written.
        if p.kind(i) == LPAREN
            && p.glued(i)
            && i > 0
            && p.kind(i - 1) == WORD
            && let Some(rp) = p.matching(i).filter(|&rp| rp < p.end)
        {
            current.push(p.raw(i, rp));
            p.pos = rp + 1;
            continue;
        }
        // `*` as a field list (ABAP SQL).
        if spec.commas && p.kind(i) == WORD && p.text(i) == "*" {
            current.push(gap(p, i, skip));
            current.push(p.t(i));
            p.pos = i + 1;
            continue;
        }
        current.push(gap(p, i, skip));
        // Comparison or key assignment inside a clause (`WITH KEY a = b`).
        if p.kind(i) == WORD
            && mode == Operands::Expr
            && matches!(p.text(i), "=" | "<>" | "<" | ">" | "<=" | ">=")
        {
            current.push(p.t(i));
            p.pos = i + 1;
            continue;
        }
        let saved_kw = p.c.mark();
        let operand = match mode {
            Operands::Expr => p.expr(),
            Operands::Logical => p.logical(),
        };
        match operand {
            Some(d) => current.push(d),
            None => {
                // Bracket groups the expression parser does not know
                // (`( c1, c2 )`, `div( a, b )`, path filters `[ … ]`) stay
                // as written.
                p.c.reset(saved_kw);
                p.pos = i;
                let open = if matches!(p.kind(i), LPAREN | LBRACKET) {
                    i
                } else if p.kind(i) == WORD
                    && p.glued(i + 1)
                    && matches!(p.kind(i + 1), LPAREN | LBRACKET)
                {
                    i + 1
                } else {
                    return None;
                };
                let close = p.matching(open).filter(|&c| c < p.end)?;
                p.mark_phrases(open + 1, close);
                current.push(p.raw(i, close));
                p.pos = close + 1;
            }
        }
    }
    parts.push(b::concat(current));
    let first = parts.remove(0);
    // Breaks inside the first part continue at the continuation indent.
    let mut docs = vec![b::cont(vec![first])];
    if spec.all_clauses {
        docs.push(b::cont(parts));
    } else {
        // Each clause: (separator, content). The separator is the first element.
        let clauses: Vec<(Doc, Doc)> = parts.into_iter().map(split_sep).collect();
        docs.push(b::cont(vec![b::fill(clauses)]));
    }
    Some(b::concat(docs))
}

/// Chain `CLEAR: a, b.` of a keyword statement (ABAP-CHAIN-001): on one line
/// if it fits, otherwise one item per line, aligned after the colon.
fn spec_chain(p: &mut P, spec: &Spec, head: Doc) -> Option<Doc> {
    let colon = p.pos;
    let mut items = Vec::new();
    let mut a = colon + 1;
    let mut depth = 0i32;
    for i in colon + 1..p.end {
        match p.kind(i) {
            LPAREN | LBRACKET => depth += 1,
            RPAREN | RBRACKET => depth -= 1,
            COMMA if depth == 0 => {
                items.push((a, i));
                a = i + 1;
            }
            COLON => return None,
            _ => {}
        }
    }
    if a < p.end {
        items.push((a, p.end));
    }
    if items.is_empty() || items.iter().any(|&(x, e)| x >= e) {
        return None;
    }
    let end = p.end;
    // ABAP-ALIGN-009, aligned style: `CLEAR:` and `FREE:` always one item
    // per line.
    let forced =
        p.c.style == Style::Aligned && matches!(spec.lead, ["CLEAR"] | ["FREE"]) && items.len() > 1;
    let mut docs = Vec::new();
    for (n, &(x, e)) in items.iter().enumerate() {
        p.pos = x;
        p.end = e;
        let item = spec_body(p, spec, false);
        p.end = end;
        let item = item?;
        let mut d = Vec::new();
        // An item glued to its comma stays glued (no line break possible).
        if n > 0 && !p.glued(x) {
            d.push(if forced { p.hl(x) } else { p.ln(x) });
        }
        d.push(b::group(vec![item]));
        if e < end && p.kind(e) == COMMA {
            d.push(p.sp(e));
            d.push(p.t(e));
        }
        docs.push(b::concat(d));
    }
    p.pos = end;
    let rule = if forced {
        "ABAP-ALIGN-009"
    } else {
        "ABAP-CHAIN-001"
    };
    p.c.span(p.first, end, rule);
    Some(b::group(vec![
        head,
        p.sp(colon),
        p.t(colon),
        p.sp(colon + 1),
        b::hang(docs),
        p.period(),
    ]))
}

/// One field of an ABAP SQL field list (ABAP-SQL-003).
struct ListItem {
    first: usize,
    /// Last token of the field without its additions.
    field_end: usize,
    field: Vec<Doc>,
    /// `AS alias`: the `AS` token.
    alias: Option<usize>,
    /// `AS alias`, `ASCENDING`, `NULLS FIRST`, … (first token, docs).
    adds: Option<(usize, Vec<Doc>)>,
    comma: Option<usize>,
}

/// Field list from `from` up to the next clause of a SELECT (ABAP-SQL-003):
/// with simple fields only on one line if it fits, else one field per line,
/// aligned under the first; with a complex field always one field per line,
/// `AS` aligned. `None` (nothing consumed) for a list of fewer than two
/// fields or one this parser does not understand.
fn select_list(p: &mut P, spec: &Spec, from: usize, order_by: bool) -> Option<Doc> {
    // The list ends at the next clause at bracket depth 0.
    let mut depth = 0i32;
    let mut to = from;
    let mut has_comma = false;
    while to < p.end {
        if depth == 0 && to > from && phrase_at(p, to, spec.clauses) > 0 {
            break;
        }
        match p.kind(to) {
            LPAREN | LBRACKET => depth += 1,
            RPAREN | RBRACKET => depth -= 1,
            COMMA if depth == 0 => has_comma = true,
            COLON | PRAGMA => return None,
            _ => {}
        }
        to += 1;
    }
    let end = p.end;
    p.end = to;
    p.pos = from;
    let items = list_items(p, spec, has_comma, order_by);
    p.end = end;
    let items = items.filter(|items| items.len() >= 2)?;
    p.pos = to;
    p.c.span(from, to - 1, "ABAP-SQL-003");

    // Complex: an addition, a `~`, or more than one glued run of tokens.
    let simple = |it: &ListItem| {
        it.adds.is_none()
            && (it.first + 1..=it.field_end).all(|k| p.glued(k))
            && !(it.first..=it.field_end).any(|k| {
                let tilde = p.kind(k) == SELECTOR && p.text(k) == "~";
                tilde
                    || matches!(
                        p.kind(k),
                        LPAREN | RPAREN | LBRACKET | RBRACKET | LBRACE | RBRACE | TEMPLATE_HEAD
                    )
            })
    };
    let complex = !items.iter().all(simple);
    let comma = |p: &P, it: &ListItem| match it.comma {
        Some(c) => b::concat(vec![p.sp(c), p.t(c)]),
        None => b::empty(),
    };
    if !complex {
        let mut docs = Vec::new();
        for (n, it) in items.into_iter().enumerate() {
            if n > 0 {
                docs.push(p.ln(it.first));
            }
            let c = comma(p, &it);
            docs.push(b::concat(it.field));
            docs.push(c);
        }
        return Some(b::group(vec![b::hang(docs)]));
    }
    // `AS` lines up when every field is printed flat (no comment inside).
    let align = items.iter().any(|it| it.alias.is_some())
        && items
            .iter()
            .all(|it| !p.has_comment(it.first, it.field_end + 1));
    let max = items
        .iter()
        .map(|it| p.flat_width(it.first, it.field_end))
        .max()
        .unwrap_or(0);
    let mut rows = Vec::new();
    for (n, it) in items.into_iter().enumerate() {
        if n > 0 {
            rows.push(p.hl(it.first));
        }
        let c = comma(p, &it);
        if align {
            rows.push(p.flat(it.first, it.field_end));
        } else {
            rows.push(b::concat(it.field));
        }
        if let Some((at, adds)) = it.adds {
            let pad = match it.alias {
                Some(_) if align => max - p.flat_width(it.first, it.field_end) + 1,
                _ => 1,
            };
            rows.push(p.spaces(at, pad));
            rows.push(b::concat(adds));
        }
        rows.push(c);
    }
    Some(b::hang(rows))
}

/// The fields of a list from `p.pos` to `p.end`. Without commas (classic
/// syntax) each operand is a field.
fn list_items(p: &mut P, spec: &Spec, has_comma: bool, order_by: bool) -> Option<Vec<ListItem>> {
    let mut items = Vec::new();
    while !p.at_end() {
        let first = p.pos;
        let mut field = Vec::new();
        while !p.at_end() {
            let i = p.pos;
            let addition = p.is(i, "AS")
                || p.is(i, "NULLS")
                || (order_by && (p.is(i, "ASCENDING") || p.is(i, "DESCENDING")));
            if p.kind(i) == COMMA || (addition && !p.glued(i)) || (!has_comma && !field.is_empty())
            {
                break;
            }
            if !field.is_empty() {
                field.push(p.sp(i));
            }
            let n_word = phrase_at(p, i, spec.words);
            if n_word > 0 {
                if !has_comma {
                    return None;
                }
                for k in 0..n_word {
                    if k > 0 {
                        field.push(p.sp(i + k));
                    }
                    field.push(p.kw(i + k));
                }
                p.pos = i + n_word;
                continue;
            }
            if p.kind(i) == WORD && matches!(p.text(i), "*" | "=" | "<>" | "<" | ">" | "<=" | ">=")
            {
                field.push(p.t(i));
                p.pos = i + 1;
                continue;
            }
            let saved = p.c.mark();
            match p.expr() {
                Some(d) => field.push(d),
                None => {
                    // Brackets the expression parser does not know stay as
                    // written (as in `spec_body`).
                    p.c.reset(saved);
                    p.pos = i;
                    let open = if matches!(p.kind(i), LPAREN | LBRACKET) {
                        i
                    } else if p.kind(i) == WORD
                        && p.glued(i + 1)
                        && matches!(p.kind(i + 1), LPAREN | LBRACKET)
                    {
                        i + 1
                    } else {
                        return None;
                    };
                    let close = p.matching(open).filter(|&c| c < p.end)?;
                    p.mark_phrases(open + 1, close);
                    field.push(p.raw(i, close));
                    p.pos = close + 1;
                }
            }
        }
        if field.is_empty() {
            return None;
        }
        let field_end = p.pos - 1;
        let mut alias = None;
        let mut adds = None;
        let a = p.pos;
        if p.is(a, "AS") {
            if a + 1 >= p.end || p.kind(a + 1) != WORD {
                return None;
            }
            alias = Some(a);
            adds = Some((a, vec![p.kw(a), p.sp(a + 1), p.t(a + 1)]));
            p.pos = a + 2;
        } else {
            let mut docs = Vec::new();
            while !p.at_end()
                && p.kind(p.pos) == WORD
                && !p.glued(p.pos)
                && matches!(
                    p.upper(p.pos).as_str(),
                    "ASCENDING" | "DESCENDING" | "NULLS" | "FIRST" | "LAST"
                )
            {
                let k = p.pos;
                if !docs.is_empty() {
                    docs.push(p.sp(k));
                }
                docs.push(if p.is(k, "ASCENDING") || p.is(k, "DESCENDING") {
                    p.kw(k)
                } else {
                    p.t(k)
                });
                p.pos += 1;
            }
            if !docs.is_empty() {
                adds = Some((a, docs));
            }
        }
        let comma = if p.kind(p.pos) == COMMA {
            let c = p.pos;
            p.pos += 1;
            if p.at_end() {
                return None;
            }
            Some(c)
        } else {
            if has_comma && !p.at_end() {
                return None;
            }
            None
        };
        items.push(ListItem {
            first,
            field_end,
            field,
            alias,
            adds,
            comma,
        });
    }
    Some(items)
}

/// `INTO ( a, b, … )` at `lp` (ABAP-SQL-003): on one line if it fits, else
/// one target per line, aligned under the first.
fn into_list(p: &mut P, spec: &Spec, lp: usize) -> Option<Doc> {
    if p.kind(lp) != LPAREN || p.glued(lp) || p.is(lp + 1, "SELECT") {
        return None;
    }
    let rp = p.matching(lp).filter(|&c| c < p.end)?;
    if rp + 1 < p.end && phrase_at(p, rp + 1, spec.clauses) == 0 {
        return None;
    }
    let end = p.end;
    p.end = rp;
    p.pos = lp + 1;
    let mut items = Vec::new();
    let ok = loop {
        if p.at_end() {
            break true;
        }
        let first = p.pos;
        let Some(d) = p.expr() else { break false };
        let comma = if p.kind(p.pos) == COMMA {
            p.pos += 1;
            Some(p.pos - 1)
        } else if p.at_end() {
            None
        } else {
            break false;
        };
        let mut docs = vec![d];
        if let Some(c) = comma {
            if p.at_end() {
                break false;
            }
            docs.extend([p.sp(c), p.t(c)]);
        }
        items.push((first, b::concat(docs)));
    };
    p.end = end;
    if !ok || items.len() < 2 {
        return None;
    }
    p.pos = rp + 1;
    p.c.span(lp, rp, "ABAP-SQL-003");
    let mut docs = Vec::new();
    for (n, (first, d)) in items.into_iter().enumerate() {
        if n > 0 {
            docs.push(p.ln(first));
        }
        docs.push(d);
    }
    Some(b::group(vec![
        p.t(lp),
        p.sp(lp + 1),
        b::hang(docs),
        p.sp(rp),
        p.t(rp),
    ]))
}

/// Keywords inside `WITH ASSOCIATIONS ( … )`.
const ASSOC_WORDS: &[&str] = &[
    "JOIN", "MANY", "ONE", "TO", "EXACT", "AS", "ON", "LEFT", "OUTER", "INNER", "WHERE", "AND",
    "OR",
];

fn select_spec() -> &'static Spec {
    SPECS
        .iter()
        .find(|s| s.lead == ["SELECT"])
        .expect("SELECT spec")
}

/// `( SELECT … )` between `lp` and `rp`, laid out by the SELECT spec.
fn subquery(p: &mut P, lp: usize, rp: usize) -> Option<Doc> {
    if !p.is(lp + 1, "SELECT") {
        return None;
    }
    let body = {
        let mut q = P::new(&mut *p.c, lp + 1, rp, false);
        let lead = strict_select_lead(&q, lp + 1, select_spec()).unwrap_or(&["SELECT"]);
        let mut head = Vec::new();
        for k in 0..lead.len() {
            if k > 0 {
                head.push(q.sp(lp + 1 + k));
            }
            head.push(q.kw(lp + 1 + k));
        }
        let head = b::concat(head);
        q.pos = lp + 1 + lead.len();
        let body = spec_body(&mut q, select_spec(), true)?;
        if !q.at_end() {
            return None;
        }
        b::concat(vec![head, body])
    };
    // Flat: `( SELECT … )`. Broken: the query on its own lines, one level
    // deeper, and `)` on its own line.
    p.c.span(lp, rp, "ABAP-SQL-002");
    Some(b::group(vec![
        p.t(lp),
        b::indent(vec![p.ln(lp + 1), body]),
        p.ln(rp),
        p.t(rp),
    ]))
}

/// `OPEN CURSOR [WITH HOLD] dbcur FOR SELECT …` / `FOR WITH …`; the query
/// follows ABAP-SQL-001.
fn open_cursor(p: &mut P) -> Option<Doc> {
    let start = p.pos;
    let for_at = (start + 2..p.end).find(|&k| p.is(k, "FOR") && !p.glued(k))?;
    if !(p.is(for_at + 1, "SELECT") || p.is(for_at + 1, "WITH")) {
        return None;
    }
    let mut head = vec![p.kw(start), p.sp(start + 1), p.kw(start + 1)];
    let mut k = start + 2;
    if p.is(k, "WITH") && p.is(k + 1, "HOLD") {
        head.extend([p.sp(k), p.kw(k), p.sp(k + 1), p.kw(k + 1)]);
        k += 2;
    }
    // The cursor (`dbcur`, `@DATA(dbcur)`).
    let end = p.end;
    p.pos = k;
    p.end = for_at;
    let cursor = p.expr().filter(|_| p.at_end());
    p.end = end;
    head.push(p.sp(k));
    head.push(cursor?);
    head.push(p.sp(for_at));
    head.push(p.kw(for_at));
    p.pos = for_at + 1;
    let query = if p.is(p.pos, "WITH") {
        with_select(p)?
    } else {
        keyword_statement(p, select_spec())?
    };
    Some(b::concat(vec![b::concat(head), p.sp(for_at + 1), query]))
}

/// `WITH +cte [( cols )] AS ( SELECT … ) [, …] SELECT …` (common table
/// expressions): one CTE per line after `WITH`, the main query on its own
/// line; every query follows ABAP-SQL-001.
fn with_select(p: &mut P) -> Option<Doc> {
    let start = p.pos;
    let mut k = start + 1;
    let mut ctes = Vec::new();
    loop {
        let name = k;
        if p.kind(name) != WORD || !p.text(name).starts_with('+') {
            return None;
        }
        let mut d = vec![p.t(name)];
        k += 1;
        if p.kind(k) == LPAREN && !p.is(k + 1, "SELECT") {
            let c = p.matching(k)?;
            d.push(p.sp(k));
            d.push(p.raw(k, c));
            k = c + 1;
        }
        if !p.is(k, "AS") {
            return None;
        }
        d.push(p.sp(k));
        d.push(p.kw(k));
        k += 1;
        if p.kind(k) != LPAREN {
            return None;
        }
        let rp = p.matching(k)?;
        d.push(p.sp(k));
        d.push(subquery(p, k, rp)?);
        k = rp + 1;
        // `WITH HIERARCHY name`, `WITH ASSOCIATIONS ( … )` (kept as written).
        loop {
            if p.is(k, "WITH") && p.is(k + 1, "HIERARCHY") && k + 2 < p.end {
                d.extend([
                    p.sp(k),
                    p.kw(k),
                    p.sp(k + 1),
                    p.kw(k + 1),
                    p.sp(k + 2),
                    p.t(k + 2),
                ]);
                k += 3;
            } else if p.is(k, "WITH") && p.is(k + 1, "ASSOCIATIONS") && p.kind(k + 2) == LPAREN {
                let c = p.matching(k + 2)?;
                mark_phrase_keywords(p, k + 3, c, ASSOC_WORDS, &["AS"]);
                d.extend([
                    p.sp(k),
                    p.kw(k),
                    p.sp(k + 1),
                    p.kw(k + 1),
                    p.sp(k + 2),
                    p.raw(k + 2, c),
                ]);
                k = c + 1;
            } else {
                break;
            }
        }
        if p.kind(k) == COMMA {
            d.push(p.sp(k));
            d.push(p.t(k));
            ctes.push((name, b::concat(d)));
            k += 1;
            continue;
        }
        ctes.push((name, b::concat(d)));
        break;
    }
    if !p.is(k, "SELECT") {
        return None;
    }
    let mut items = Vec::new();
    for (n, (name, d)) in ctes.into_iter().enumerate() {
        if n > 0 {
            items.push(p.hl(name));
        }
        items.push(d);
    }
    let lead = p.kw(start);
    let first_cte = start + 1;
    p.pos = k;
    let main = keyword_statement(p, select_spec())?;
    p.c.span(start, p.end, "ABAP-SQL-002");
    Some(b::concat(vec![
        lead,
        p.sp(first_cte),
        b::hang(items),
        p.hl(k),
        main,
    ]))
}

/// SQL words that are keywords wherever they stand in an ABAP SQL statement.
const SQL_WORDS: &[&str] = &[
    "AS",
    "AND",
    "OR",
    "NOT",
    "LIKE",
    "IN",
    "BETWEEN",
    "IS",
    "NULL",
    "EXISTS",
    "SELECT",
    "FROM",
    "WHERE",
    "INNER",
    "OUTER",
    "JOIN",
    "ON",
    "DISTINCT",
    "ESCAPE",
    "ASCENDING",
    "DESCENDING",
    "HAVING",
    "SINGLE",
    "INTO",
    "UNION",
    "INTERSECT",
];

/// SQL functions written as keywords when a bracket follows (`COUNT( * )`,
/// `CAST( … )`); other SQL functions are written like built-in functions.
const SQL_FUNCTIONS: &[&str] = &["COUNT", "SUM", "MIN", "MAX", "AVG", "CAST", "OVER"];

/// Dictionary types in `CAST( … AS CHAR( 6 ) )`.
const SQL_TYPES: &[&str] = &[
    "CHAR", "NUMC", "DEC", "CURR", "QUAN", "INT1", "INT2", "INT4", "INT8", "FLTP", "DATS", "TIMS",
    "DATN", "TIMN", "CLNT", "LANG", "UNIT", "CUKY", "SSTRING", "RAW", "D16N", "D34N", "UTCLONG",
];

/// Marks the SQL keywords of an ABAP SQL statement `from..to` that the clause
/// parser left as operands (inside brackets, subqueries, CASE, functions).
fn mark_sql_keywords(p: &mut P, from: usize, to: usize) {
    let mut open_case = 0usize;
    for k in from..to {
        if p.kind(k) != WORD {
            continue;
        }
        let w = p.upper(k);
        let paren = p.glued(k + 1) && p.kind(k + 1) == LPAREN;
        // Inline declaration after `@` (`@DATA(x)`).
        if p.glued(k) {
            if k > from && p.kind(k - 1) == AT && paren && matches!(w.as_str(), "DATA" | "FINAL") {
                p.c.keywords.push(k);
            }
            continue;
        }
        // `alias~field`, `struct-comp`: a name.
        if p.glued(k + 1) && p.kind(k + 1) == SELECTOR {
            continue;
        }
        let after_as = k > from && p.is(k - 1, "AS");
        let kw = if paren {
            SQL_FUNCTIONS.contains(&w.as_str()) || (after_as && SQL_TYPES.contains(&w.as_str()))
        } else if after_as {
            // An alias, or a type without length (`AS INT4 )`).
            SQL_TYPES.contains(&w.as_str()) && p.kind(k + 1) == RPAREN
        } else {
            match w.as_str() {
                "CASE" => {
                    open_case += 1;
                    true
                }
                "WHEN" | "THEN" | "ELSE" => open_case > 0,
                "END" if open_case > 0 => {
                    open_case -= 1;
                    true
                }
                "GROUP" | "ORDER" => p.is(k + 1, "BY"),
                "BY" => {
                    k > from
                        && (p.is(k - 1, "GROUP")
                            || p.is(k - 1, "ORDER")
                            || p.is(k - 1, "PARTITION"))
                }
                "LEFT" | "RIGHT" => p.is(k + 1, "OUTER") || p.is(k + 1, "JOIN"),
                "CROSS" => p.is(k + 1, "JOIN"),
                // Window functions: `OVER( PARTITION BY … ORDER BY … ROWS
                // BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW )`
                "PARTITION" => p.is(k + 1, "BY"),
                "ROWS" | "UNBOUNDED" | "PRECEDING" | "FOLLOWING" => true,
                "CURRENT" => p.is(k + 1, "ROW"),
                "ROW" => k > from && p.is(k - 1, "CURRENT"),
                "ALL" => k > from && p.is(k - 1, "UNION"),
                // `ORDER BY col DESCENDING NULLS FIRST`; a column may be
                // called `first`.
                "NULLS" => p.is(k + 1, "FIRST") || p.is(k + 1, "LAST"),
                "FIRST" | "LAST" => k > from && p.is(k - 1, "NULLS"),
                _ => SQL_WORDS.contains(&w.as_str()),
            }
        };
        if kw {
            p.c.keywords.push(k);
        }
    }
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
    "INDICATORS",
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
        // `WITH EMPTY KEY WITH …`
        "KEY" => matches!(w, "COMPONENTS" | "ALIAS" | "WITH"),
        "OF" => matches!(w, "STRUCTURE" | "ENUM" | "REF"),
        "BASE" => w == "TYPE",
        _ => false,
    }
}

/// Words of a RAP derived type between `FOR` and the entity name
/// (`TYPE TABLE FOR READ RESULT entity`, `TYPE RESPONSE FOR MAPPED LATE
/// entity`).
const DERIVED_TYPE_KW: &[&str] = &[
    "ACTION",
    "AUTHORIZATION",
    "CHANGE",
    "CREATE",
    "DELETE",
    "DETERMINATION",
    "EARLY",
    "EVENT",
    "FAILED",
    "FEATURES",
    "FUNCTION",
    "GLOBAL",
    "HIERARCHY",
    "IMPORT",
    "INSTANCE",
    "KEY",
    "LATE",
    "LINK",
    "LOCK",
    "MAPPED",
    "OF",
    "PERMISSIONS",
    "READ",
    "REPORTED",
    "REQUEST",
    "RESULT",
    "UPDATE",
    "VALIDATION",
];

/// Number of keyword tokens of a RAP derived type starting at `i`, the word
/// after `TYPE`: `TABLE FOR …`, `STRUCTURE FOR …` or `RESPONSE FOR …`, up to
/// the entity name. 0 if `i` does not start one.
pub(crate) fn derived_type_len(p: &P, i: usize) -> usize {
    let starts = matches!(
        p.upper(i).as_str(),
        "TABLE" | "STRUCTURE" | "RESPONSE" | "REQUEST"
    ) && p.is(i + 1, "FOR")
        && !p.glued(i + 1);
    if !starts {
        return 0;
    }
    let mut n = 2;
    while i + n < p.end
        && p.kind(i + n) == WORD
        && !p.glued(i + n)
        && DERIVED_TYPE_KW.contains(&p.upper(i + n).as_str())
        // The last word before the end is the entity name.
        && i + n + 1 < p.end
        && p.kind(i + n + 1) == WORD
    {
        n += 1;
    }
    n
}

/// Lays out tokens `from..to` of a declaration item flat, marking keywords.
fn decl_tokens(p: &mut P, from: usize, to: usize) -> Vec<Doc> {
    let mut docs = Vec::new();
    let mut prev_kw: Option<String> = None;
    let mut in_components = false;
    let mut derived_until = from;
    for i in from..to {
        if i > from {
            docs.push(p.sp(i));
        }
        let w = p.upper(i);
        let is_word = p.kind(i) == WORD && !p.glued(i);
        if prev_kw.as_deref() == Some("TYPE") && is_word {
            derived_until = i + derived_type_len(p, i);
        }
        if i < derived_until {
            docs.push(p.kw(i));
            prev_kw = None;
            continue;
        }
        let operand_slot = prev_kw.as_deref().is_some_and(takes_operand)
            && !prev_kw.as_deref().is_some_and(|pk| keyword_after(pk, &w));
        // After COMPONENTS, words are field names unless they start the
        // next key clause.
        let ends_components = matches!(
            w.as_str(),
            "WITH" | "UNIQUE" | "NON-UNIQUE" | "DEFAULT" | "EMPTY" | "KEY"
        );
        let keyword_here = !in_components || ends_components;
        // Type-phrase words are keywords only with their partner word;
        // otherwise they are type names (`TYPE any`, a type named `line`).
        let partner_ok = match w.as_str() {
            "ANY" | "INDEX" => p.is(i + 1, "TABLE"),
            // `SORTED TABLE`, `WITH NON-UNIQUE SORTED KEY sk`
            "STANDARD" | "SORTED" | "HASHED" => p.is(i + 1, "TABLE") || p.is(i + 1, "KEY"),
            "LINE" | "RANGE" => p.is(i + 1, "OF"),
            "REF" => p.is(i + 1, "TO"),
            _ => true,
        };
        if is_word && !operand_slot && DECL_KW.contains(&w.as_str()) && keyword_here && partner_ok {
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
    /// Aligned style: a row in columns (ABAP-ALIGN-003).
    Cols(DeclCols, bool),
    /// A line without an anchor.
    Plain(Doc),
}

/// The type, `VALUE …` and `READ-ONLY` of a declaration item `ne..e` (after
/// its name), each optional part with the gap before it. `None` if the parts
/// are not in this order.
fn decl_parts(p: &mut P, ne: usize, e: usize) -> Option<DeclCols> {
    let find = |p: &P, w: &str| (ne..e).find(|&k| p.kind(k) == WORD && !p.glued(k) && p.is(k, w));
    let v = find(p, "VALUE");
    let r = find(p, "READ-ONLY");
    if let (Some(v), Some(r)) = (v, r)
        && r < v
    {
        return None;
    }
    let ty_end = v.or(r).unwrap_or(e);
    let ty = if ne < ty_end {
        decl_tokens(p, ne, ty_end)
    } else {
        Vec::new()
    };
    let value = v.map(|v| (p.sp(v), decl_tokens(p, v, r.unwrap_or(e))));
    let ro = r.map(|r| (p.sp(r), decl_tokens(p, r, e)));
    Some(DeclCols {
        name: Vec::new(),
        ty,
        value,
        ro,
    })
}

fn declaration(p: &mut P) -> Option<Stmt> {
    let start = p.pos;
    let lead = p.kw(start);
    let chain = p.kind(start + 1) == COLON;
    let aligned = p.c.style == Style::Aligned;
    if !chain {
        if start + 1 >= p.end {
            return None;
        }
        // `TYPES BEGIN OF s.` / `DATA END OF s.`
        let structure =
            (p.is(start + 1, "BEGIN") || p.is(start + 1, "END")) && p.is(start + 2, "OF");
        if structure {
            p.c.keywords.push(start + 1);
        }
        let ne = name_end(p, start + 1, p.end);
        // ABAP-ALIGN-008: a one-line declaration is aligned with its
        // neighbors of the same keyword.
        let cols = if aligned && !structure && ne < p.end {
            decl_parts(p, ne, p.end).map(|mut d| {
                let mut name = vec![lead.clone(), p.sp(start + 1)];
                for i in start + 1..ne {
                    if i > start + 1 {
                        name.push(p.sp(i));
                    }
                    name.push(p.t(i));
                }
                d.name = name;
                d.end_with(vec![p.period()]);
                d
            })
        } else {
            None
        };
        // ABAP-DECL-002: `TYPES  END OF` puts `END` under `BEGIN`.
        let gap = match p.c.end_gap {
            Some(n) if structure && p.is(start + 1, "END") => b::spaces(n),
            _ => p.sp(start + 1),
        };
        let mut docs = vec![lead, gap];
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
        if let Some(cols) = cols {
            let key = AlignKey::Decl(p.upper(start));
            let mut docs = docs.into_iter();
            let head: Vec<Doc> = docs.by_ref().take(1).collect();
            return Some(Stmt::Row(Row {
                key,
                head,
                gap: b::empty(),
                rest: docs.collect(),
                anchor: 0,
                decl: Some(Box::new(cols)),
            }));
        }
        return Some(Stmt::Doc(b::group(docs)));
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
    if items.is_empty()
        || items
            .iter()
            .any(|&(a, e)| a >= e || (p.glued(a) && a != colon + 1))
    {
        return None;
    }
    let mut lines: Vec<(u32, Line, usize)> = Vec::new(); // (level, line, first token)
    let mut level = 0u32;
    // ABAP-DECL-001, aligned style: a chain of two or more items always has
    // one item per line.
    let mut forced = aligned && items.len() > 1;
    p.c.multi_line |= forced;
    for (n, &(a, e)) in items.iter().enumerate() {
        let comma = if e < p.end && p.kind(e) == COMMA {
            Some(e)
        } else {
            None
        };
        let begin = p.is(a, "BEGIN") && p.is(a + 1, "OF");
        let end = p.is(a, "END") && p.is(a + 1, "OF");
        // A structure may open and close in different statements
        // (`DATA: BEGIN OF s.` … `DATA: END OF s.`).
        if end {
            level = level.saturating_sub(1);
        }
        let mut tail_end = vec![];
        if let Some(c) = comma {
            tail_end.push(p.sp(c));
            tail_end.push(p.t(c));
        }
        let line = if begin || end {
            forced = true;
            p.c.multi_line = true;
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
            let brk = n > 0 && p.c.commented[a];
            let cols = if aligned && ne < e {
                decl_parts(p, ne, e)
            } else {
                None
            };
            if let Some(mut d) = cols {
                d.name = head;
                d.end_with(tail_end);
                Line::Cols(d, brk)
            } else if ne < e {
                let mut tail = vec![p.sp(ne)];
                tail.extend(decl_tokens(p, ne, e));
                tail.extend(tail_end);
                Line::Row(head, tail, brk)
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
    let docs = group_decl_lines(p, &mut lines, 0, forced, aligned);
    p.pos = p.end;
    let body = b::hang(docs);
    let gap = match p.c.end_gap {
        Some(n) if p.is(colon + 1, "END") && p.is(colon + 2, "OF") => b::spaces(n),
        _ => p.sp(colon + 1),
    };
    Some(Stmt::Doc(b::group(vec![
        lead,
        p.sp(colon),
        p.t(colon),
        gap,
        body,
        p.period(),
    ])))
}

/// The lines of a declaration chain, from level `base` on: consecutive rows
/// of the same level form an alignment group, each line indented by its
/// level above `base`. Aligned style (ABAP-ALIGN-003): a nested
/// `BEGIN OF … END OF` between rows of one level is a line of that group, so
/// the rows before and after it align together.
fn group_decl_lines(
    p: &mut P,
    lines: &mut [(u32, Line, usize)],
    base: u32,
    forced: bool,
    aligned: bool,
) -> Vec<Doc> {
    // The line closing the structure that opens at `k` (same level, `END OF`),
    // if only deeper lines come between.
    let nested_end = |p: &P, lines: &[(u32, Line, usize)], k: usize| -> Option<usize> {
        let (lvl, _, first) = lines[k];
        if !(p.is(first, "BEGIN") && p.is(first + 1, "OF")) || p.c.commented[first] {
            return None;
        }
        let m = (k + 1..lines.len()).find(|&m| lines[m].0 <= lvl)?;
        (lines[m].0 == lvl && p.is(lines[m].2, "END") && p.is(lines[m].2 + 1, "OF")).then_some(m)
    };
    let mut docs: Vec<Doc> = Vec::new();
    let mut k = 0;
    while k < lines.len() {
        let (lvl, _, first_tok) = lines[k];
        let mut content: Vec<Doc>;
        if matches!(lines[k].1, Line::Cols(..)) {
            // Rows of this level, and with `aligned` nested structures
            // between them: (index range, nested structure).
            let mut parts: Vec<(usize, usize, bool)> = Vec::new();
            let mut j = k;
            loop {
                let from = j;
                while j < lines.len() && lines[j].0 == lvl && matches!(lines[j].1, Line::Cols(..)) {
                    j += 1;
                }
                if j > from {
                    parts.push((from, j, false));
                }
                let next = if aligned && j < lines.len() && lines[j].0 == lvl {
                    nested_end(p, lines, j)
                } else {
                    None
                };
                // Only a structure followed by more rows of this level joins.
                match next {
                    Some(m)
                        if m + 1 < lines.len()
                            && lines[m + 1].0 == lvl
                            && matches!(lines[m + 1].1, Line::Cols(..))
                            && !p.c.commented[lines[m + 1].2] =>
                    {
                        parts.push((j, m + 1, true));
                        j = m + 1;
                    }
                    _ => break,
                }
            }
            let (value_col, ro_col) = {
                let cols: Vec<&DeclCols> = parts
                    .iter()
                    .filter(|x| !x.2)
                    .flat_map(|&(f, t, _)| lines[f..t].iter())
                    .filter_map(|l| match &l.1 {
                        Line::Cols(d, _) => Some(d),
                        _ => None,
                    })
                    .collect();
                decl_columns(&cols)
            };
            let mut items: Vec<AlignItem> = Vec::new();
            for (f, t, nested) in parts {
                if nested {
                    let inner = group_decl_lines(p, &mut lines[f..t], lvl, forced, aligned);
                    items.push(b::line(vec![b::concat(inner)]));
                    continue;
                }
                for l in &mut lines[f..t] {
                    let Line::Cols(d, brk) = std::mem::replace(&mut l.1, Line::Plain(b::empty()))
                    else {
                        unreachable!()
                    };
                    if brk {
                        items.push(AlignItem::Break);
                    }
                    items.push(d.row(value_col, ro_col));
                }
            }
            k = j;
            content = vec![if forced {
                b::align_broken(items)
            } else {
                b::align(items)
            }];
        } else if matches!(lines[k].1, Line::Row(..)) {
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
        docs.push((base..lvl).fold(b::concat(content), |d, _| b::indent(vec![d])));
    }
    docs
}

// ---- aliases -----------------------------------------------------------------

/// `ALIASES name FOR intf~comp.`, aligned with its neighbors, and chains of
/// two or more such items, one per line and aligned (ABAP-ALIGN-007). Other
/// forms are keyword statements.
fn aliases(p: &mut P) -> Option<Stmt> {
    let start = p.pos;
    let chain = p.kind(start + 1) == COLON;
    let first = if chain { start + 2 } else { start + 1 };
    // (name, FOR, end of the component)
    let mut items = Vec::new();
    let mut x = first;
    loop {
        if x >= p.end || p.kind(x) != WORD || (x > first && p.glued(x)) {
            return None;
        }
        let f = name_end(p, x, p.end);
        if !p.is(f, "FOR") || p.glued(f) || p.kind(f + 1) != WORD || p.glued(f + 1) {
            return None;
        }
        let e = name_end(p, f + 1, p.end);
        items.push((x, f, e));
        if e == p.end {
            break;
        }
        if !chain || p.kind(e) != COMMA {
            return None;
        }
        x = e + 1;
    }
    if chain && items.len() < 2 {
        return None;
    }
    let lead = p.kw(start);
    p.pos = p.end;
    if !chain {
        let (x, f, e) = items[0];
        let for_kw = p.kw(f);
        return Some(Stmt::Row(Row {
            decl: None,
            key: AlignKey::Aliases,
            head: vec![lead, p.sp(x), p.flat(x, f - 1)],
            gap: p.sp(f),
            rest: vec![for_kw, p.sp(f + 1), p.flat(f + 1, e - 1), p.period()],
            anchor: 0,
        }));
    }
    let mut rows = Vec::new();
    for &(x, f, e) in &items {
        let for_kw = p.kw(f);
        let mut rest = vec![for_kw, p.sp(f + 1), p.flat(f + 1, e - 1)];
        if e < p.end {
            rest.push(p.sp(e));
            rest.push(p.t(e));
        } else {
            rest.push(p.period());
        }
        rows.push(b::row(
            vec![p.flat(x, f - 1)],
            vec![p.sp(f), b::group(rest)],
        ));
    }
    p.c.span(start, p.end, "ABAP-ALIGN-007");
    Some(Stmt::Doc(b::concat(vec![
        lead,
        p.sp(start + 1),
        p.t(start + 1),
        p.sp(first),
        b::hang(vec![b::align_lines(rows)]),
    ])))
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

/// Marks the words of `words` in `from..to` as keywords. A word right after
/// a keyword of `takes_operand` is a name (`FOR EVENT clicked OF cl_gui`,
/// `RESULT result`), as is a word glued to its neighbors.
fn mark_phrase_keywords(p: &mut P, from: usize, to: usize, words: &[&str], takes_operand: &[&str]) {
    let mut prev: Option<String> = None;
    for i in from..to {
        let glued_next = i + 1 < to && p.glued(i + 1);
        if p.kind(i) != WORD || p.glued(i) || glued_next {
            prev = None;
            continue;
        }
        let w = p.upper(i);
        let operand = prev.as_deref().is_some_and(|x| takes_operand.contains(&x));
        if words.contains(&w.as_str()) && !operand {
            p.c.keywords.push(i);
            prev = Some(w);
        } else {
            prev = None;
        }
    }
}

const METHOD_HEADER_WORDS: &[&str] = &[
    "ABSTRACT",
    "FINAL",
    "REDEFINITION",
    "DEFAULT",
    "IGNORE",
    "FAIL",
    "FOR",
    "TESTING",
    "RISK",
    "LEVEL",
    "HARMLESS",
    "DANGEROUS",
    "CRITICAL",
    "DURATION",
    "SHORT",
    "MEDIUM",
    "LONG",
    "EVENT",
    "OF",
    "VALIDATE",
    "DETERMINE",
    "ON",
    "SAVE",
    "MODIFY",
    "READ",
    "LOCK",
    "FEATURES",
    "INSTANCE",
    "GLOBAL",
    "AUTHORIZATION",
    "NUMBERING",
    "PRECHECK",
    "ENTITY",
    "TABLE",
    "FUNCTION",
    "AMDP",
    "OPTIONS",
    "READ-ONLY",
    "CDS",
    "SESSION",
    "CLIENT",
    "DEPENDENT",
    "BEHAVIOR",
];

const METHOD_HEADER_OPERAND: &[&str] = &["EVENT", "OF", "FUNCTION"];

const RAP_PARAM_WORDS: &[&str] = &[
    "FOR",
    "ACTION",
    "FUNCTION",
    "CREATE",
    "UPDATE",
    "DELETE",
    "READ",
    "LOCK",
    "FULL",
    "RESULT",
    "LINK",
    "REQUEST",
    "EVENT",
    "DETERMINE",
    "VALIDATE",
    "ON",
    "SAVE",
    "MODIFY",
    "FEATURES",
    "INSTANCE",
    "GLOBAL",
    "AUTHORIZATION",
    "ENTITY",
    "PRECHECK",
    "NUMBERING",
];

const RAP_PARAM_OPERAND: &[&str] = &[
    "ACTION", "FUNCTION", "CREATE", "UPDATE", "DELETE", "LOCK", "FULL", "RESULT", "LINK",
    "REQUEST", "EVENT", "PRECHECK",
];

/// Sections of a `FORM` signature (ABAP-FORM-001), laid out like `METHODS`.
const FORM_SECTIONS: &[&str] = &["TABLES", "USING", "CHANGING", "RAISING"];

fn is_section(p: &P, i: usize, sections: &[&str]) -> bool {
    // A parameter may be named like a section keyword (`IMPORTING importing TYPE i`).
    p.kind(i) == WORD
        && !p.glued(i)
        && sections.contains(&p.upper(i).as_str())
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
    p.is(j, "TYPE") || p.is(j, "LIKE") || p.is(j, "STRUCTURE")
}

fn methods(p: &mut P) -> Option<Stmt> {
    let start = p.pos;
    let lead = p.kw(start);
    // ABAP-ALIGN-005 and ABAP-ALIGN-006 apply to `METHODS` only.
    let alignable = p.is(start, "METHODS");
    if p.kind(start + 1) != COLON {
        let parts = method_parts(p, start + 1, p.end, SECTIONS)?;
        p.pos = p.end;
        if let Some(key) = parts.key.clone().filter(|_| alignable) {
            let (name, gap, mut rest) = parts.split();
            rest.push(p.period());
            let mut head = vec![lead, p.sp(start + 1)];
            head.extend(name);
            return Some(Stmt::Row(Row {
                decl: None,
                key,
                head,
                gap,
                rest,
                anchor: 0,
            }));
        }
        return Some(Stmt::Doc(b::group(vec![
            lead,
            p.sp(start + 1),
            parts.doc(),
            p.period(),
        ])));
    }
    // Chain `METHODS: a …, b … .`: one item per line, aligned after the colon.
    let colon = start + 1;
    let mut items = Vec::new();
    let mut a = colon + 1;
    let mut depth = 0i32;
    for i in colon + 1..p.end {
        match p.kind(i) {
            LPAREN | LBRACKET => depth += 1,
            RPAREN | RBRACKET => depth -= 1,
            COMMA if depth == 0 => {
                items.push((a, i));
                a = i + 1;
            }
            COLON => return None,
            _ => {}
        }
    }
    if a < p.end {
        items.push((a, p.end));
    }
    if items.is_empty() || items.iter().any(|&(x, e)| x >= e || p.glued(x)) {
        return None;
    }
    let mut parts = Vec::with_capacity(items.len());
    for &(x, e) in &items {
        parts.push(method_parts(p, x, e, SECTIONS)?);
    }
    // ABAP-ALIGN-005: a chain with two or more test methods has one item per
    // line, and the test methods are aligned. ABAP-ALIGN-006: the same for a
    // chain of two or more redefinitions and nothing else (as ABAP Cleaner).
    let count = |k: &AlignKey| parts.iter().filter(|m| m.key.as_ref() == Some(k)).count();
    let key = if !alignable {
        None
    } else if count(&AlignKey::ForTesting) >= 2 {
        Some(AlignKey::ForTesting)
    } else if parts.len() >= 2 && count(&AlignKey::Redefinition) == parts.len() {
        Some(AlignKey::Redefinition)
    } else {
        None
    };
    let mut docs = Vec::new();
    let mut rows = Vec::new();
    for (n, (&(x, e), item)) in items.iter().zip(parts).enumerate() {
        let mut end = Vec::new();
        if e < p.end && p.kind(e) == COMMA {
            end.push(p.sp(e));
            end.push(p.t(e));
        }
        if key.is_some() {
            if n + 1 == items.len() {
                end.push(p.period());
            }
            if item.key == key {
                let (name, gap, mut rest) = item.split();
                rest.extend(end);
                rows.push(b::row(name, vec![gap, b::group(rest)]));
            } else {
                let mut line = vec![b::group(vec![item.doc()])];
                line.extend(end);
                rows.push(b::line(line));
            }
            continue;
        }
        let mut d = Vec::new();
        if n > 0 {
            d.push(p.ln(x));
        }
        // Each item decides on its own whether its sections break.
        d.push(b::group(vec![item.doc()]));
        d.extend(end);
        docs.push(b::concat(d));
    }
    p.pos = p.end;
    if let Some(key) = key {
        let rule = if key == AlignKey::ForTesting {
            "ABAP-ALIGN-005"
        } else {
            "ABAP-ALIGN-006"
        };
        p.c.span(start, p.end, rule);
        return Some(Stmt::Doc(b::concat(vec![
            lead,
            p.sp(colon),
            p.t(colon),
            p.sp(colon + 1),
            b::hang(vec![b::align_lines(rows)]),
        ])));
    }
    p.c.span(start, p.end, "ABAP-CHAIN-001");
    Some(Stmt::Doc(b::group(vec![
        lead,
        p.sp(colon),
        p.t(colon),
        p.sp(colon + 1),
        b::hang(docs),
        p.period(),
    ])))
}

/// A method signature in parts: the name, the header words after it
/// (starting with the gap before the first one) and the sections; `key`
/// when the signature is a test method or a redefinition.
struct MethodParts {
    name: Vec<Doc>,
    header: Vec<Doc>,
    sections: Doc,
    key: Option<AlignKey>,
}

impl MethodParts {
    fn doc(self) -> Doc {
        let mut docs = self.name;
        docs.extend(self.header);
        docs.push(self.sections);
        b::concat(docs)
    }

    /// Name, gap before the first header word, and the rest from that word
    /// (a signature with a `key` has header words).
    fn split(self) -> (Vec<Doc>, Doc, Vec<Doc>) {
        let mut header = self.header.into_iter();
        let gap = header.next().unwrap_or_else(b::empty);
        let mut rest: Vec<Doc> = header.collect();
        rest.push(self.sections);
        (self.name, gap, rest)
    }
}

/// One method signature from its name (`from`) to `to` (exclusive), without
/// the leading `METHODS` keyword.
fn method_item(p: &mut P, from: usize, to: usize, sections: &[&str]) -> Option<Doc> {
    method_parts(p, from, to, sections).map(MethodParts::doc)
}

fn method_parts(p: &mut P, from: usize, to: usize, sections: &[&str]) -> Option<MethodParts> {
    let name = from;
    if p.kind(name) != WORD {
        return None;
    }
    let saved_end = p.end;
    p.end = to;
    let result = method_item_inner(p, name, sections);
    p.end = saved_end;
    if result.is_some() {
        let rule = if sections == FORM_SECTIONS {
            "ABAP-FORM-001"
        } else {
            "ABAP-METHODS-001"
        };
        p.c.span(from, to - 1, rule);
    }
    result
}

fn method_item_inner(p: &mut P, name: usize, sections_kw: &[&str]) -> Option<MethodParts> {
    let mut i = name + 1;
    // Header words up to the first section keyword (FOR TESTING, ABSTRACT,
    // RAP handler phrases, FOR EVENT … OF …).
    let header_end = (i..p.end)
        .find(|&k| is_section(p, k, sections_kw))
        .unwrap_or(p.end);
    mark_phrase_keywords(p, i, header_end, METHOD_HEADER_WORDS, METHOD_HEADER_OPERAND);
    // The name ends at the first token not glued to it (`if_intf~meth`).
    let name_stop = name_end(p, name, header_end);
    let mut name_docs = vec![p.t(name)];
    let mut header = Vec::new();
    while i < header_end {
        let docs = if i < name_stop {
            &mut name_docs
        } else {
            &mut header
        };
        docs.push(p.sp(i));
        docs.push(p.t(i));
        i += 1;
    }
    // ABAP-ALIGN-005: `[ABSTRACT] FOR TESTING`, then sections (`RAISING`);
    // ABAP-ALIGN-006: `[FINAL] REDEFINITION` alone.
    let words: Vec<String> = (name_stop..header_end)
        .map(|k| {
            if p.kind(k) == WORD && !p.glued(k) {
                p.upper(k)
            } else {
                String::new()
            }
        })
        .collect();
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let key = match words.as_slice() {
        ["FOR", "TESTING"] | ["ABSTRACT", "FOR", "TESTING"] => Some(AlignKey::ForTesting),
        ["REDEFINITION"] | ["FINAL", "REDEFINITION"] if header_end == p.end => {
            Some(AlignKey::Redefinition)
        }
        _ => None,
    };
    // ABAP-METHODS-001, aligned style: the signature is a table (FORM keeps
    // the compact form in both styles).
    if p.c.style == Style::Aligned && sections_kw == SECTIONS && i < p.end {
        let sections = method_table(p, i)?;
        return Some(MethodParts {
            name: name_docs,
            header,
            sections,
            key,
        });
    }
    let mut sections = Vec::new();
    while i < p.end {
        if !is_section(p, i, sections_kw) {
            return None;
        }
        let kw = p.kw(i);
        let mut j = i + 1;
        let section_end = (j..p.end)
            .find(|&k| is_section(p, k, sections_kw))
            .unwrap_or(p.end);
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
        // `IMPORTING … PREFERRED PARAMETER p`
        for k in i + 1..section_end {
            if p.is(k, "PREFERRED") && p.is(k + 1, "PARAMETER") {
                p.c.keywords.push(k);
                p.c.keywords.push(k + 1);
            }
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
                // RAP handler parameters: `keys FOR ACTION Travel~accept
                // RESULT result`, `REQUEST requested FOR Travel RESULT result`.
                if p.is(ne, "FOR") || p.is(ne, "REQUEST") {
                    mark_phrase_keywords(p, ne, e, RAP_PARAM_WORDS, RAP_PARAM_OPERAND);
                } else if p.is(a, "REQUEST") && p.is(a + 2, "FOR") {
                    mark_phrase_keywords(p, a, e, RAP_PARAM_WORDS, RAP_PARAM_OPERAND);
                }
                d.push(p.sp(ne));
                d.extend(decl_tokens(p, ne, e));
                // OPTIONAL / DEFAULT inside parameters.
            }
            param_docs.push(b::concat(d));
        }
        sections.push(b::concat(vec![p.ln(i), kw, b::indent(param_docs)]));
        i = section_end;
    }
    Some(MethodParts {
        name: name_docs,
        header,
        sections: b::cont(sections),
        key,
    })
}

/// One line of an aligned method signature (ABAP-METHODS-001).
struct SigRow {
    /// Section keyword, on the first line of its section.
    kw: Option<Doc>,
    name: Vec<Doc>,
    ty: Vec<Doc>,
    /// `OPTIONAL` / `DEFAULT …`, with the gap before it.
    opt: Option<(Doc, Vec<Doc>)>,
}

/// `VALUE(x)` / `REFERENCE(x)` or a plain name, `a..ne`.
fn param_name(p: &mut P, a: usize, ne: usize) -> Vec<Doc> {
    let mut d = Vec::new();
    for k in a..ne {
        if k > a {
            d.push(p.sp(k));
        }
        let w = p.upper(k);
        if k == a && (w == "VALUE" || w == "REFERENCE") && p.kind(k + 1) == LPAREN {
            d.push(p.kw(k));
        } else {
            d.push(p.t(k));
        }
    }
    d
}

/// The sections of a method signature from `i` to `p.end` as an aligned
/// table below the method name at the continuation indent: the keywords
/// padded to the longest, the names, `TYPE`, and `OPTIONAL` / `DEFAULT` in
/// columns. A one-line table stays behind the name if it fits.
fn method_table(p: &mut P, mut i: usize) -> Option<Doc> {
    let first_kw = i;
    let mut rows: Vec<SigRow> = Vec::new();
    while i < p.end {
        if !is_section(p, i, SECTIONS) {
            return None;
        }
        let kw = p.kw(i);
        let j = i + 1;
        let section_end = (j..p.end)
            .find(|&k| is_section(p, k, SECTIONS))
            .unwrap_or(p.end);
        for k in j..section_end {
            if p.is(k, "PREFERRED") && p.is(k + 1, "PARAMETER") {
                p.c.keywords.push(k);
                p.c.keywords.push(k + 1);
            }
        }
        if p.is(i, "RAISING") || p.is(i, "EXCEPTIONS") || !param_start(p, j) {
            // A list of exceptions (or content that is not a parameter
            // list) stays on one line.
            if (j..section_end).any(|k| k > j && p.c.commented[k]) {
                return None;
            }
            // RAP handler parameters: `keys FOR ACTION Travel~accept RESULT
            // result`, `REQUEST requested FOR Travel RESULT result`.
            if p.is(i, "IMPORTING") && j < section_end {
                let from = if p.is(j, "REQUEST") && p.is(j + 2, "FOR") {
                    j
                } else {
                    name_end(p, j, section_end)
                };
                mark_phrase_keywords(p, from, section_end, RAP_PARAM_WORDS, RAP_PARAM_OPERAND);
            }
            let mut name = Vec::new();
            for k in j..section_end {
                if k > j {
                    name.push(p.sp(k));
                }
                name.push(p.t(k));
            }
            rows.push(SigRow {
                kw: Some(kw),
                name,
                ty: Vec::new(),
                opt: None,
            });
            i = section_end;
            continue;
        }
        let mut params: Vec<(usize, usize)> = Vec::new();
        let mut s = j;
        for k in j + 1..section_end {
            if param_start(p, k) {
                params.push((s, k));
                s = k;
            }
        }
        params.push((s, section_end));
        let mut kw = Some(kw);
        for (a, e) in params {
            let ne = name_end(p, a, e);
            let name = param_name(p, a, ne);
            let mut ty = Vec::new();
            let mut opt = None;
            if ne < e {
                if p.is(ne, "FOR") || p.is(ne, "REQUEST") {
                    mark_phrase_keywords(p, ne, e, RAP_PARAM_WORDS, RAP_PARAM_OPERAND);
                } else if p.is(a, "REQUEST") && p.is(a + 2, "FOR") {
                    mark_phrase_keywords(p, a, e, RAP_PARAM_WORDS, RAP_PARAM_OPERAND);
                }
                // `OPTIONAL` / `DEFAULT x` (not `WITH DEFAULT KEY`).
                let o = (ne + 1..e).find(|&k| {
                    p.kind(k) == WORD
                        && !p.glued(k)
                        && (p.is(k, "OPTIONAL") || p.is(k, "DEFAULT"))
                        && !p.is(k - 1, "WITH")
                });
                ty.push(p.sp(ne));
                ty.extend(decl_tokens(p, ne, o.unwrap_or(e)));
                if let Some(o) = o {
                    opt = Some((p.sp(o), decl_tokens(p, o, e)));
                }
            }
            rows.push(SigRow {
                kw: kw.take(),
                name,
                ty,
                opt,
            });
        }
        i = section_end;
    }
    // A column for OPTIONAL / DEFAULT if at least 40 % of the parameters
    // have one (ABAP Cleaner's fill ratio).
    let params = rows.iter().filter(|r| !r.ty.is_empty()).count();
    let with_opt = rows.iter().filter(|r| r.opt.is_some()).count();
    let opt_col = with_opt > 0 && with_opt * 100 >= params * 40;
    let mut items = Vec::new();
    // Comment lines do not end the table: all names, types and `OPTIONAL`
    // / `DEFAULT` stay in their columns (ABAP-METHODS-001).
    for r in rows {
        let kw_cell = r.kw.map_or_else(Vec::new, |k| vec![k]);
        // The type cell starts with its own gap; strip it, the name cell
        // brings the separator space.
        let mut ty = r.ty;
        if !ty.is_empty() {
            ty.remove(0);
        }
        let (head, tail) = match r.opt {
            Some((gap, o)) if opt_col => {
                let mut t = vec![gap];
                t.extend(o);
                (ty, t)
            }
            Some((gap, o)) => {
                ty.push(gap);
                ty.extend(o);
                (ty, Vec::new())
            }
            None => (ty, Vec::new()),
        };
        items.push(b::row_cols(vec![kw_cell, r.name], head, tail));
    }
    if items.len() > 1 {
        // A table of two or more lines always starts below the method name.
        p.c.multi_line = true;
        return Some(b::cont(vec![p.hl(first_kw), b::align_broken(items)]));
    }
    let table = b::shared(b::align_broken(items));
    Some(b::choice(
        b::concat(vec![p.sp(first_kw), b::hang(vec![table.clone()])]),
        b::cont(vec![p.hl(first_kw), table]),
    ))
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
    let aligned = p.c.style == Style::Aligned;
    let read = p.is(start, "READ");
    let mut lines: Vec<EmlLine> = Vec::new();
    let mut op_open = false;
    let mut op_has_data = false;
    // Aligned style: the last line is an `ENTITY` line without operation.
    let mut entity_line = false;
    while !p.at_end() {
        let i = p.pos;
        if p.glued(i) || p.kind(i) != WORD {
            return None;
        }
        let w = p.upper(i);
        // Aligned style: `READ … ENTITY x BY \_assoc` keeps the association
        // with the entity (on its line, or in the head of `READ ENTITY`).
        if aligned
            && read
            && w == "BY"
            && !op_open
            && (entity_line || (!plural && lines.is_empty()))
        {
            let mut docs = vec![p.sp(i), p.kw(i)];
            p.pos = i + 1;
            docs.push(p.sp(p.pos));
            docs.push(p.expr()?);
            match lines.last_mut() {
                Some(l) if entity_line => l.content.extend(docs),
                _ => head.extend(docs),
            }
            entity_line = false;
            continue;
        }
        entity_line = false;
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
                entity_line = true;
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
                if aligned {
                    // Aligned style: `WITH …` and `FROM …` on their own line.
                    lines.push(EmlLine {
                        level: op_level,
                        first: i,
                        content: vec![kw, sp, value],
                        data: vec![],
                    });
                } else {
                    let sep = p.ln(i);
                    lines
                        .last_mut()?
                        .data
                        .push((sep, b::concat(vec![kw, sp, value])));
                }
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

// ---- CALL FUNCTION / CALL METHOD ------------------------------------------

const CALL_GROUPS: &[&str] = &[
    "EXPORTING",
    "IMPORTING",
    "CHANGING",
    "TABLES",
    "EXCEPTIONS",
    "RECEIVING",
    "PARAMETER-TABLE",
    "EXCEPTION-TABLE",
];
const CALL_HEADER_WORDS: &[&str] = &[
    "DESTINATION",
    "STARTING",
    "NEW",
    "TASK",
    "IN",
    "UPDATE",
    "BACKGROUND",
    "UNIT",
    "PERFORMING",
    "CALLING",
    "ON",
    "END",
    "OF",
    "AS",
    "SEPARATE",
    "KEEPING",
    "LOGICAL",
    "WORK",
];

fn is_group(p: &P, i: usize) -> bool {
    p.kind(i) == WORD
        && !p.glued(i)
        && CALL_GROUPS.contains(&p.upper(i).as_str())
        // A parameter may be named like a group keyword (`tables = x`).
        && (!p.is(i + 1, "=") || p.glued(i + 1))
}

/// `CALL FUNCTION 'X' …` and `CALL METHOD x->m …` with parameter groups
/// (ABAP-CALL-003): each group keyword starts a line at the continuation
/// indent, its parameters hang behind it with aligned `=` (A1).
fn call_with_groups(p: &mut P) -> Option<Doc> {
    let start = p.pos;
    let mut head = vec![p.kw(start), p.sp(start + 1), p.kw(start + 1)];
    p.pos = start + 2;
    head.push(p.sp(p.pos));
    head.push(p.expr()?);
    // Header words (DESTINATION, STARTING NEW TASK, IN UPDATE TASK, …).
    while !p.at_end() && !is_group(p, p.pos) {
        let i = p.pos;
        head.push(p.sp(i));
        if p.kind(i) == WORD && !p.glued(i) && CALL_HEADER_WORDS.contains(&p.upper(i).as_str()) {
            head.push(p.kw(i));
            p.pos += 1;
        } else {
            head.push(p.expr()?);
        }
    }
    let aligned = p.c.style == Style::Aligned;
    let first_group = p.pos;
    let mut groups = Vec::new();
    // Aligned style: one table for all groups (keyword column, `=` aligned
    // across the groups).
    let mut table: Vec<AlignItem> = Vec::new();
    while !p.at_end() {
        let k = p.pos;
        if !is_group(p, k) {
            return None;
        }
        let kw = p.kw(k);
        p.pos += 1;
        let content = if p.is(k, "PARAMETER-TABLE") || p.is(k, "EXCEPTION-TABLE") {
            let sp = p.sp(p.pos);
            let e = p.expr()?;
            if aligned {
                table.push(b::row_cols(vec![vec![kw.clone()]], vec![e.clone()], vec![]));
            }
            b::concat(vec![sp, e])
        } else {
            let mut rows = Vec::new();
            while !p.at_end() && !is_group(p, p.pos) {
                let name = p.pos;
                // `EXCEPTIONS not_found OTHERS.`: exceptions without a value.
                if p.is(k, "EXCEPTIONS")
                    && p.kind(name) == WORD
                    && !p.glued(name)
                    && !p.is(name + 1, "=")
                {
                    let name_doc = if p.is(name, "OTHERS") {
                        p.kw(name)
                    } else {
                        p.t(name)
                    };
                    rows.push(b::row(vec![name_doc], vec![]));
                    p.pos = name + 1;
                    continue;
                }
                if p.kind(name) != WORD
                    || !p.is(name + 1, "=")
                    || p.glued(name + 1)
                    || p.glued(name)
                {
                    return None;
                }
                if !rows.is_empty() && p.c.commented[name] {
                    rows.push(AlignItem::Break);
                }
                let eq = name + 1;
                p.pos = eq + 1;
                let value = p.expr()?;
                let name_doc = if p.is(name, "OTHERS") {
                    p.kw(name)
                } else {
                    p.t(name)
                };
                rows.push(b::row(
                    vec![name_doc],
                    vec![p.sp(eq), p.t(eq), p.sp(eq + 1), value],
                ));
            }
            if rows.is_empty() {
                return None;
            }
            if aligned {
                let mut first = true;
                for it in &rows {
                    table.push(match it {
                        AlignItem::Row(r) => {
                            let mut r = r.clone();
                            r.lead = vec![if first {
                                vec![kw.clone(), b::space()]
                            } else {
                                Vec::new()
                            }];
                            first = false;
                            AlignItem::Row(r)
                        }
                        // A comment line ends the `=` alignment
                        // (ABAP-ALIGN-001), not the keyword column.
                        AlignItem::Break => AlignItem::Split,
                        other => other.clone(),
                    });
                }
            }
            b::concat(vec![p.sp(k + 1), b::hang(vec![b::align(rows)])])
        };
        groups.push(b::concat(vec![p.ln(k), kw, content]));
    }
    if aligned && !table.is_empty() {
        // ABAP-CALL-003: each group on its own line, even when it would fit.
        return Some(b::group(vec![
            b::cont(head),
            b::cont(vec![p.hl(first_group), b::align_broken(table)]),
            p.period(),
        ]));
    }
    // The header at the continuation indent too, where an end-of-line
    // comment breaks it.
    Some(b::group(vec![b::cont(head), b::cont(groups), p.period()]))
}
