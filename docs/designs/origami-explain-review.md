# origami: line-break explanations and change review

*   **Author:** Nils Lutz
*   **Status:** Approved
*   **Date:** 2026-11-21

--------------------------------------------------------------------------------

## Overview

origami rewrites the layout of ABAP and CDS sources. Today a developer sees
*that* a line was broken, but not *why*, and the only way to look at changes
before they are written is a plain unified diff (`origami --check --diff`).
This design adds three things:

1.  **Line-break explanations in the core.** For a line break that origami
    introduces, the core reports which construct was broken and why: the
    construct did not fit (with the column it would have ended at and the
    limit), a fixed style rule (with its rule ID, only when the attribution is
    exact), or an end-of-line comment. It covers ABAP and the CDS family. The
    formatted output must stay byte-for-byte the same.
2.  **`origami --review`** in the CLI: shows what would change in a readable
    form (colors on terminals, line numbers, changed parts highlighted within
    a line), with one explanation note per broken construct. It writes
    nothing, like `--check`.
3.  **"Preview Changes" in VS Code**: opens VS Code's diff view with a
    snapshot of the document on the left and the formatted text on the right,
    with an action to apply the result.

The explanations are for ABAP developers: plain sentences with style rule
IDs from [docs/STYLE.md](../STYLE.md), no terms from origami's internals.

This design builds on the approved formatter design
([docs/designs/origami.md](origami.md)) and on the brainstorm
brief `concepts_brief.md` (two rounds of independent review). A desktop app
(GPUI) was considered and set aside; see Alternatives Considered.

## The Problem

*   **Line breaks look arbitrary.** origami follows fixed rules, but a
    developer who sees `CALL METHOD` parameters split over six lines in one
    place and kept on one line in another cannot tell which rule decided, or
    whether it was the 120-column limit. Without a reason, layout feels like
    noise, and adoption discussions turn into taste discussions.
*   **The maintainer cannot see the reason either.** When a corpus run shows
    an unexpected break, finding the responsible rule means reading the
    parser and printer code. There is no record of which decision produced a
    given line.
*   **Reviewing changes before writing is hard to read.** `--check --diff`
    prints a unified diff. For layout changes (indentation, joined and split
    lines, re-cased keywords) it is long and hard to scan: whole lines are
    marked as changed when only spacing or case differs, and nothing explains
    the new breaks.
*   **VS Code users have no preview at all.** The extension applies the
    edits directly when the user formats. There is no way to look at the
    result first.

## Goals

*   **G1. Explanations for introduced breaks.** For every line break in the
    output that origami introduced (a break that was not in the input at the
    same position), the core can produce one note per broken construct and
    cause (a construct whose breaks have several causes gets one note per
    cause). Breaks kept from the input get no note.
*   **G2. Accurate content.** Each explanation names:
    *   the cause: *width* (the construct does not fit), *rule* (a style rule
        always breaks here, regardless of width), or *comment* (an end-of-line
        comment forces the break);
    *   for width causes, the column the construct would have ended at and the
        limit (for example "would end at column 134, limit 120"), taken from
        the same measurement the printer used for its decision;
    *   the style rule ID from STYLE.md only when the attribution is exact;
        otherwise the cause alone, never a guessed rule.
*   **G3. No change to formatting.** Formatting output is byte-for-byte
    identical to the output before this feature, checked once against the
    whole test corpus (1,920 ABAP and 227 CDS-family files) at default and
    non-default settings. The rule tests, the statement catalog test and the
    corpus hash run also assert that the result of `explain` equals the
    result of `format`; the stress test, fuzzing and the lexer oracle run
    without explanations.
*   **G4. ABAP and CDS in the first version.**
*   **G5. `origami --review`.** Implies `--check` (writes nothing; exit codes
    as for `--check`), shows for each changed file a readable view of the
    changes with line numbers, changed parts highlighted within lines, colors
    on terminals (respecting `NO_COLOR` / `CLICOLOR_FORCE`), control
    characters from the source escaped, and the explanation notes next to the
    broken lines. Works on paths, stdin, `--lines` and `--changed` (notes only
    for changes that are kept).
*   **G6. `--diff` is unchanged.** It stays a plain unified diff for scripts
    and `git apply`.
*   **G7. VS Code "Preview Changes".** Opens the built-in diff view: a
    snapshot of the document at the time of the command (left), the
    formatted text (right), and an action to apply the result to the
    document. Works for `file:` and ADT `abap:` documents.
*   **G8. Explanation sentences are documented.** Each rule-based sentence
    appears in STYLE.md next to its rule; the code's copy is checked against
    STYLE.md by a test.
*   **G9. No cost when unused.** Formatting without explanations is not
    measurably slower (`cargo xtask bench-check` within its tolerance).

## Non-Goals

*   Explaining anything other than introduced line breaks: re-casing,
    alignment, blank lines, spaces, or why a long line was *not* broken, or
    why an author's break was removed.
*   A desktop app (GPUI or other), an HTML report, or a WASM playground.
*   Explanations inside VS Code (hovers, inline notes); VS Code shows the
    diff only.
*   Changing the output of `--diff`, the JSON diagnostics schema
    (`schemas/diagnostics.v1.json`) or the SARIF output.
*   Explanations in the MCP server or in `--diagnostics agent` in the first
    version.
*   An interactive apply workflow in the CLI (per-file or per-hunk prompts);
    applying stays `origami` without `--check`.
*   New style rules or changed layouts. Breaks without a matching rule are
    explained by their cause; adding rules goes through the normal approval
    gate in STYLE.md.

## Open Questions

*   **Q3 (resolved in Stage 0).** (a) confirmed and fixed: an end-of-line
    comment after the period forced the statement's group to break; (b)
    confirmed and fixed: the fill check ignored the text after the last
    clause, so lines ended at column 121; (c) documented; (d) fixed. Original
    question: **Printer facts to confirm in a spike before implementation.** (a) A
    comment after the period seems to force `CALL FUNCTION` to break; (b) the
    fill check may ignore the statement's period; (c) `--changed` takes the next
    argument as its revision (documented, no behavior change); (d) the human code frame prints
    control characters from the source unescaped. Each is either fixed as a
    separate bug or documented; none changes this design.

Resolved during drafting: Q1 (mechanism: comparison with a second, unlimited
width print), Q2 (rule attributions: a side table of token ranges), Q4
(explanations ship with `--review` in the same release; `cargo xtask
explain` is the development tool; the wording of notes is human output and
not a stable contract), Q5 (VS Code: title "origami: Preview Changes"; on a
structural error no diff opens, the diagnostics go to the Problems panel and
a message says why; timeout 5 seconds). See the Technical Plan.

## Critical User Journeys (CUJs)

### CUJ 1: An ABAP developer asks why a call was split

*   **Given**: a class with a method call whose parameters origami puts on
    separate lines.
*   **When**: the developer runs `origami --review zcl_travel.clas.abap`.
*   **Then**: the view shows the changed lines with line numbers, the call
    on its new lines, and one note next to it, for example "ABAP-CALL-001:
    one parameter per line, because the call would end at column 134 (limit
    120)". Nothing is written; the exit code is 1 because the file would
    change.

### CUJ 2: A team reviews only its own change

*   **Given**: a feature branch in an abapGit repository that has never been
    formatted.
*   **When**: a developer runs `origami --review --changed main`.
*   **Then**: only the changes in lines touched since `main` are shown, with
    notes for the breaks among them; untouched code is not listed.

### CUJ 3: A VS Code user looks before formatting

*   **Given**: an ABAP class open in VS Code, with unsaved edits.
*   **When**: the user runs "origami: Preview Changes".
*   **Then**: a diff view opens with the current text on the left and the
    formatted text on the right. The user chooses "Apply"; the document now
    equals the right side and stays unsaved. If the user edited the document
    after opening the preview, Apply reformats the current text instead of
    overwriting the edits.

### CUJ 4: The maintainer investigates an unexpected break

*   **Given**: a corpus file where a CDS association line is broken
    unexpectedly.
*   **When**: the maintainer runs the explanation for that file.
*   **Then**: the note says whether the break came from width (with the
    measured column), from a rule (with its ID), or from a comment, so the
    responsible rule or printer decision is found without reading code.

### CUJ 5: A script keeps using the plain diff

*   **Given**: a CI job that runs `origami --check --diff | git apply -R
    --check`.
*   **When**: origami with this feature is installed.
*   **Then**: the output of `--diff` is byte-for-byte the same as before; the
    job is unaffected.

## Technical Plan (Architecture)

### Summary of the approach

Explanations are computed after formatting, without changing how origami
formats:

1.  The final formatting pass also records a **side table of rule spans**:
    "tokens i to j form a construct laid out by rule R" (for example a
    method call's parameter list under ABAP-CALL-001). Recording happens
    only when explanations are requested.
2.  The same final pass is **printed a second time with an unlimited line
    width**. Because origami never changes tokens, output token k is input
    token k in both prints, so line breaks can be compared token by token.
3.  Each line break in the normal output is classified:
    *   **kept**: the token also starts a line in the input; no note;
    *   **introduced, width**: the token starts a line in the normal output
        but not in the wide one; the break exists only because of the limit;
    *   **introduced, comment**: the break survives the wide print and the
        token before it carries an end-of-line comment;
    *   **introduced, rule**: the break survives the wide print for another
        reason (a style rule that always breaks).

    Stage 2 finding: an ABAP line comment always ends its line in the input
    too, so "the token before it carries an end-of-line comment" never
    applies to ABAP. What does happen is that an end-of-line comment
    *inside* a construct forces the construct onto several lines. So a break
    that no rule explains (a rule break without a rule ID, or a width break
    where neither N nor M exceeds the limit) is attributed to the first
    end-of-line comment inside its construct, if there is one; only the
    remaining width breaks become `Layout`. In the corpus this moved 43 of 63
    `Layout` notes to comment notes.
4.  Introduced breaks are grouped into **one note per construct and
    cause**. The construct is the innermost rule span that contains the
    break, or else the ABAP statement or CDS layout unit. A construct whose
    breaks have several causes gets one note per cause, in the order rule,
    comment, width. The note names the cause, the rule ID only if a rule span
    supplied it, and for width breaks the measured column.

```text
 input ──► check::run (final pass, explain on) ──► output text + rule spans
                    │                                     │
                    └──► same pass, line width = ∞ ──► wide text
                                                          │
 input tokens ─┬─ output tokens ─┬─ wide tokens ──────────┘
               ▼                 ▼
        line starts per token: input / output / wide
                    │
                    ▼
   classify each output line start: kept | width | comment | rule
                    │
                    ▼
   group by innermost rule span (else statement/unit) ──► notes
                    │                                       │
        ┌───────────┴────────────┐                          │
        ▼                        ▼                          ▼
  origami --review         cargo xtask explain      (VS Code shows the
  (readable view + notes)  (metrics, maintainer)     diff only, no notes)
```

### 1. Shared types (`origami_syntax`) and sentences (`origami_core`)

`origami_syntax` knows no SAP language, so it gets only language-neutral
types in a new module `explain`:

*   `RuleId`: a `&'static str` such as `"ABAP-CALL-001"`.
*   `RuleSpan { first: TokenIndex, last: TokenIndex, rule: RuleId }`: an
    inclusive token range.
*   `Scope { Construct, Line }`.
*   `BreakCause`: `Width { column: u32, limit: u16, scope: Scope }`, `Rule`,
    `Comment { text: String }` (the comment of the first such break),
    `Layout` (no number can be given; see "Columns").
*   `Note { lines: RangeInclusive<u32>, break_tokens: Vec<TokenIndex>, rule:
    Option<RuleId>, cause: BreakCause, text: String }`. `lines` are the
    1-based lines of the construct in the formatted output; `break_tokens`
    are the first tokens of the lines this note explains; `text` is the
    finished sentence.

`LangResult` gets `rule_spans: Vec<RuleSpan>` (empty unless explanations
were requested). This is a minor-version change of `origami_syntax` (0.x),
noted in CHANGELOG. `LangOptions`, the public `Options` and `FormatResult` do
not change.

The **sentence table** (one **rule phrase** and construct noun per rule ID,
plus the cause sentences) lives in `origami_core/src/explain.rs`. Each rule
phrase also appears in STYLE.md as the rule's `Explanation:` line; a test
fails if the two differ. The full table is in the appendix "Note
sentences".

### 2. Recording rule spans (`origami_abap`, `origami_cds`)

Each language crate gets `format_explained(...)` next to `format(...)`: the
same code with span recording switched on. `format` is unchanged.

**Containment rules.** A break before token k is inside span S if
`S.first < k <= S.last`. The innermost span is the one with the fewest
tokens; for equal ranges the span pushed last wins. Each rule's span is
pushed in exactly one function.

*   **ABAP.** The parser context `Ctx` (defined in `stmt.rs`) already keeps a
    list of keyword tokens that is rolled back when a parse attempt fails. It
    gets a second list, `rule_spans`, an `explain` flag, a
    `mark() -> Mark { keywords: usize, spans: usize }` and a
    `reset(Mark)` that rolls back both lists, and `span(first, last, rule)`
    (a no-op unless `explain`). Every place that saves and truncates the
    keyword list (the `saved` / `saved_kw` / `keywords.len()` patterns in
    stmt.rs, statements.rs and layout.rs) uses `mark()` / `reset()`, and so do
    the raw fallbacks that print a sub-span as written after a partly
    successful sub-parse (in stmt.rs around the predicate, constructor-row
    and parenthesis fallbacks, and in statements.rs around the
    `WITH ASSOCIATIONS` / raw-group fallback). Stage 2 finding: at the raw
    fallbacks the keywords of the failed sub-parse were never rolled back
    (they keep their case); rolling them back now could change output, so
    these sites roll back only the spans (`reset_spans`). Spans:

    | Rule | Construct | Pushed in |
    |---|---|---|
    | ABAP-CALL-001 | parameter list `( … )` of a method call, named-parameter and keyword-group layouts only | `call_args` (the caller of `hanging`), when called for a method call |
    | ABAP-CALL-002 | one parameter keyword group (`EXPORTING …`) inside a call | `call_args`, `PARAM_KEYWORDS` branch |
    | ABAP-CALL-003 | `CALL FUNCTION` / `CALL METHOD` statement | `call_with_groups` |
    | ABAP-VALUE-001 | row layout of a constructor | `ctor_body` |
    | ABAP-COND-001 | logical expression | `logical` |
    | ABAP-DECL-001 | declaration or declaration chain | `declaration` |
    | ABAP-CHAIN-001 | chain of another keyword statement | `spec_chain` |
    | ABAP-METHODS-001 / ABAP-FORM-001 | method or FORM signature | `method_item_inner` (FORM through the FORM path) |
    | ABAP-SQL-001 | `SELECT` and `MERGE` (specs with `all_clauses`) | `keyword_statement` |
    | ABAP-SQL-002 | CTE list and each subquery | `with_select`, `subquery` |
    | ABAP-EML-001 | EML statement | `eml` |

    `call_args` gets a parameter that says whether it serves a method call or
    a constructor; constructors (`NEW`, `CONV`, `REF`, …) push no
    ABAP-CALL-001 span. Other keyword statements (`READ TABLE`, `LOOP AT`,
    …) and database DML statements (INSERT, UPDATE, MODIFY, DELETE on
    tables, which use the fill layout rather than the ABAP-SQL-001 layout)
    push no span: their notes name the cause only. This DML gap between
    STYLE.md's ABAP-SQL-001 text and the implementation is recorded in the
    appendix.

*   **CDS.** `G::new(text, stream, lang, explain)`. Grammar helpers collect
    their spans in a local vector and append it to `G.spans` only when they
    return `Some`, so a failed attempt leaves no span. Spans:
    `annotation` (CDS-ANNO-001), `header` (CDS-HEAD-001), `case` expressions
    inside `element` (CDS-ELEM-001), `grant` (DCL-GRANT-001).
    `layout::layout(text, stream, lang, explain) -> (Doc, Vec<RuleSpan>)`
    drains `G.spans` for each unit.

*   **Statement and unit starts.** A break before the first token of an
    ABAP statement or a CDS layout unit (for example two statements that
    shared a line) is attributed to ABAP-STMT-001 or CDS-UNIT-001 without a
    span; the explain pass reads the boundaries from `LangResult.units`.

Spans never touch the layout tree or the printer, so recording cannot change
the output. A break without a span gets a cause-only note.

### 3. The explain pass (`origami_core`)

*   `check::run` returns, besides the result, the final pass's quarantine
    and opaque sets and whether the whole-file fallback was used.
    `check::print_wide(src, lang, opts, identity, &Checked) -> String`
    formats with the same sets and `line_width = u16::MAX`;
    `check::relex` becomes `pub(crate)` and is reused.
*   `format_inner(input, hint, opts, lines, explain: bool) -> Inner {
    result: FormatResult, parts: Option<ExplainParts> }`, where
    `ExplainParts` holds the normalized input and output text (LF, no BOM),
    the language, the `Checked` data and the rule spans. `format_lines` calls
    it with `explain = false`.
*   New public function `explain(input, hint, &Options, lines) ->
    Explained` with `Explained { result: FormatResult, notes: Vec<Note>,
    input_text: String, output_text: String }` (normalized texts, for the
    review view). Formatting and the whole explain computation run inside
    one `catch_unwind`; a panic gives X002 and no output, as for `format`.
    Steps:
    1.  format with `explain = true` (`result` equals what `format_lines`
        returns);
    2.  if the result is an error, unchanged, or the whole-file fallback:
        no notes;
    3.  wide print; the 255-character check does not apply to it;
    4.  relex output and wide print; for each token record whether it starts
        a line in the input, the output and the wide print;
    5.  drop breaks whose token lies in a quarantine unit or an opaque unit of
        the final pass;
    6.  classify the remaining line starts (kept, width, comment, rule) and
        group them into notes (containment rules above; fallback: the ABAP
        statement or CDS layout unit);
    7.  with `lines`: notes are computed on the full output.
        `check::select_lines` returns, besides the mixed text, the kept
        changed blocks as pairs (output line range → mixed line range). A
        note is kept if the output line of each of its break tokens lies in
        a kept block; its lines are mapped through that block. If
        `select_lines` returns `None` (W001): no notes.

### Columns in width notes

The printer decides with a yes/no check from the current position and does
not compute a column. The note therefore measures in the wide print, where
nothing was broken for width:

*   **N** = the column of the last character of the construct's last token in
    the wide print (1-based, inclusive). If N is greater than the limit, the
    note says "would end at column N (limit W)".
*   If N is not greater than the limit (the printer broke the construct
    because of the text that follows it on the same line), **M** = the column
    of the last character of that line in the wide print is used: "the line
    would end at column M (limit W)".
*   If M is not greater than the limit either (possible for layout choices
    that compare variants), the cause is `Layout` and the note gives no
    number: "laid out on several lines to fit the layout rules". `cargo
    xtask explain --corpus` reports the number of these notes.

Columns count characters, as the printer does. Diagnostics count UTF-16
units; the two differ only for characters outside the Basic Multilingual
Plane.

### Wording

Notes are plain sentences for ABAP developers, built from the sentence
table:

*   width, with rule: "ABAP-CALL-001: one parameter per line, because the
    call would end at column 134 (limit 120)";
*   width, no rule: "split because the statement would end at column 131
    (limit 120)";
*   rule: "ABAP-SQL-001: each clause on its own line";
*   comment: "split because of the comment `" check`";
*   layout: "laid out on several lines to fit the layout rules".

Terms from origami's internals (group, fill, doc) never appear.

### 4. `origami --review` (`origami_cli`)

*   New flag `--review`: implies `--check`; clap rejects it together with
    `--diff` or `--write` (exit 3). Exit codes as for `--check`. Works with
    paths, stdin, `--lines` and `--changed`.
*   For each file that would change, the review view goes to **stdout**
    (like `--diff` today); diagnostics stay on stderr in the chosen format.
*   The view is built from the normalized input and output text of
    `Explained` (LF, no BOM); its header says "CRLF" or "BOM" when the file
    has them.
*   View per file: a header with the path and counts; then each change with
    old and new line numbers, removed lines and added lines, and within
    changed lines only the changed characters highlighted (word-level diff:
    `similar`'s `TextDiff::iter_inline_changes`). Up to 2 lines of context.
    Notes appear after the last added line of their construct, indented and
    marked with `↳`.
*   Colors only when stdout is a terminal (`NO_COLOR`, `CLICOLOR_FORCE` as in
    the human diagnostics; one shared `report::color_enabled(stream)`).
    Without colors, changed characters are marked with `[-…-]` and `{+…+}`.
    Control characters from the source (C0 except tab, DEL, C1, and the bidi
    controls U+202A–U+202E and U+2066–U+2069) are shown escaped (`\x1b`,
    `\u{202e}`), in the view and in the human code frame.
*   In stdin mode, `--review` prints the view; the formatted text is not
    printed.
*   docs/CLI.md: the streams rule becomes "stdout carries only formatted
    text (stdin mode), the diff (`--diff`) or the review view (`--review`)".

Example (no colors):

```text
zcl_travel.clas.abap  (1 change, 1 note)
 41 41 │   METHOD validate.
 42    │-    ls_res = zcl_check=>run( iv_id = ls_travel-id iv_date = sy-datum iv_mode = 'X' ).
    42 │+    ls_res = zcl_check=>run( iv_id   = ls_travel-id
    43 │+                             iv_date = sy-datum
    44 │+                             iv_mode = 'X' ).
       │   ↳ ABAP-CALL-001: one parameter per line, because the call would end at column 134 (limit 120)
 43 45 │   ENDMETHOD.
```

### 5. VS Code "Preview Changes" (`editors/vscode`)

*   Command `origami.previewChanges` ("origami: Preview Changes"). It is
    enabled when the active document matches the formatter's document
    selector (language `abap` or `acds`, or a supported file pattern), via a
    context key `origami.formattable` that the extension sets; this covers
    `file:` and ADT `abap:` documents.
*   It takes a snapshot of the document text and version, formats the
    snapshot through the existing CLI call (timeout 5 seconds), and opens
    VS Code's diff view (`vscode.diff`) with two read-only virtual documents
    from a content provider for the scheme `origami-preview`: left the
    snapshot, right the formatted text. Open previews are kept in a map by
    id.
*   "origami: Apply Preview" (title bar of the diff editor and command
    palette) applies the result: if the document version still equals the
    snapshot's, the edits from snapshot to formatted text; if the document
    changed meanwhile, it formats the current text again and applies those
    edits, so later edits are never overwritten. The document stays unsaved
    and the diff tab closes.
*   Messages (all start with "origami:"): nothing to change (no diff); not
    changed because of a structural error (no diff, diagnostics in the
    Problems panel); preview failed, see Output (timeout or failure, no
    diff); the existing version-mismatch message when no binary fits; the
    document is closed or could not be formatted again (Apply makes no
    edit).
*   No explanations in VS Code (non-goal).

### 6. Development tool and checks (`xtask`)

*   `cargo xtask explain <file>` prints the notes for one file.
*   `cargo xtask explain --corpus` runs over the corpus and fails if
    (a) an introduced break belongs to no note or to more than one,
    (b) `explain().result` differs from `format()` for a file,
    (c) a rule ID in a note has no `Explanation:` line in STYLE.md, or
    (d) a break token of a width note lies outside its construct. It
    reports counts per cause and per rule, including `Layout` notes.
*   `cargo xtask corpus run --hashes <file>` writes, sorted, one line per
    corpus file and setting with a 64-bit FNV-1a hash of the formatted output
    (or `ERR <code>` if formatting fails), for the default settings and for
    width 80, indent 4, lower-case keywords. It also checks idempotency at
    the non-default setting and that `explain` gives the same result as
    `format`. The G3 baseline file is `target/g3-baseline.txt`.

### Risks and how the plan handles them

*   **Wrong construct named for nested breaks.** The innermost span is used;
    when the construct would fit, the note switches to the line's column (M)
    instead of claiming the construct is too long. The corpus metric shows
    how often each case occurs.
*   **Spans recorded during a failed parse attempt.** `Ctx::mark()` /
    `reset()` roll spans back together with keywords; a test parses
    statements whose first attempt fails and checks that no span survives.
*   **Cost.** Recording only with `explain`; the wide print and two extra
    lexer runs only in `explain`. Plain formatting does no extra work
    (`xtask bench-check`).
*   **Formatting changes by accident.** Neither the layout tree nor the
    printer changes; the hash comparison (G3) checks it anyway.

## Alternatives Considered

### Alternative A: GPUI desktop app ("origami Preview")

*   **Pros**: one place for before/after, diagnostics, explanations, a
    repository view and trying the three settings; reaches users without
    VS Code.
*   **Cons**: no signing certificates for macOS and Windows exist, so
    Gatekeeper and SmartScreen block or warn, and managed machines in SAP
    shops often refuse unsigned apps; a separately shipped app carries its own
    origami version and can show and write output that the repository's CI
    rejects (the drift the main design rejected in its Alternative 19); a
    settings playground produces a style different from `origami.toml` on
    purpose; text and diff views must be built by hand (Zed's editor crates
    are GPL-3.0, which `deny.toml` excludes); GPUI's API changes often and its
    Windows backend is young.
*   **Reason for Rejection**: Rejected by author for now: the costs above are
    not justified by performance, since a whole repository formats in 0.1 to
    0.2 seconds. To be reconsidered when certificates exist and users ask for
    it.

### Alternative B: Terminal UI

*   **Pros**: runs everywhere the CLI runs, no signing, no GPU.
*   **Cons**: duplicates what the CLI already does in the terminal; does not
    reach Eclipse/ADT users.
*   **Reason for Rejection**: Rejected by author: the terminal is covered by
    the CLI.

### Alternative C: Local web UI served on localhost

*   **Pros**: works on every operating system with a browser.
*   **Cons**: opens a local port, with the security questions that brings; a
    static file would avoid them.
*   **Reason for Rejection**: Rejected by author during the brainstorm.

### Alternative D: Static HTML report (`origami report`) and WASM playground

*   **Pros**: no signing, no installation; the report comes from the
    repository's pinned CLI and cannot drift; usable as a CI artifact; a WASM
    build would allow pasting code and trying settings in the browser.
*   **Cons**: a second output channel to maintain; the WASM variant needs a
    wasm32 build with a byte-identity test against the native build, and
    `catch_unwind` does not work on wasm32.
*   **Reason for Rejection**: Not chosen by author for this scope; the
    explanation in the core and the two previews cover the needs named in the
    problem statement. Recommended by the round-1 reviewers; kept as a future
    improvement.

### Alternative E: Explaining every line break

*   **Pros**: complete.
*   **Cons**: undefined (the printer writes newlines in several places,
    including blank lines and the final newline); mostly noise, because every
    ABAP statement starts on a new line and CDS keeps many breaks from the
    input; many breaks have no style rule to cite.
*   **Reason for Rejection**: Rejected by author after review: one note per
    broken construct, only for breaks origami introduces.

### Alternative F: Colored, annotated `--diff`

*   **Pros**: no new flag.
*   **Cons**: scripts, pipes and `git apply` read `--diff`; any change breaks
    them.
*   **Reason for Rejection**: Rejected by author: `--diff` stays a plain
    unified diff; the readable view is `--review`.

### Alternative G: Record decisions inside the printer

*   **Pros**: the exact cause and the exact number the printer compared,
    including which variant of a trial layout was chosen.
*   **Cons**: changes in the printer's hottest paths; the printer is cloned
    for trial layouts and called recursively for fill, and the record must
    survive both; any slip can move a decision unnoticed.
*   **Reason for Rejection**: Rejected by author: the comparison with a
    second, unlimited-width print leaves the printer unchanged.

### Alternative H: Re-compute the printer's measurement from the layout tree

*   **Pros**: printer unchanged; one note per group, more exact than the
    wide-print comparison.
*   **Cons**: re-implements the printer's measurement; where the two differ
    (trial layouts, alignment groups) the note is wrong.
*   **Reason for Rejection**: Rejected by author in favor of the wide-print
    comparison.

### Alternative I: Rule IDs in a wrapper node of the layout tree

*   **Pros**: attribution follows the structure that forms the construct.
*   **Cons**: the node is not transparent everywhere: splitting clauses
    (`Doc::into_parts`) and alignment (`align_group`) would treat it
    differently and change the output; every printer function must handle it.
*   **Reason for Rejection**: Rejected by author: a side table of token
    ranges leaves the layout tree unchanged.

### Alternative J: Rule IDs as a field on existing nodes

*   **Pros**: no new node kind; attribution follows the structure.
*   **Cons**: changes the layout tree's types in every crate and makes nodes
    larger; the attribution must still be mapped to tokens; a group used in
    several trial layouts may need different rules.
*   **Reason for Rejection**: Rejected by author in favor of the side table.

### Alternative K: Documentation only (`origami --check --diff | delta`)

*   **Pros**: no code; readable side-by-side diffs with existing tools.
*   **Cons**: no explanations; needs an extra tool installed.
*   **Reason for Rejection**: Not chosen as the solution; the recipe is
    added to the documentation as a zero-cost option.

--------------------------------------------------------------------------------

## Non-Functional Requirements

*   **Formatting unchanged (G3).** No change to the layout tree, the printer,
    `Options`, `FormatResult`, the JSON schema or `--diff`. Checked by the
    corpus hash comparison and the existing rule, corpus, oracle, stress and
    fuzz runs.
*   **Performance (G9).** Plain formatting does no extra work: span
    recording runs only in `format_explained`. `explain` costs one more
    print and two more lexer runs per file (expected under 2x the format
    time of a file). Checked by running `cargo bench -p origami_core` on the
    G3 baseline and after Stage 4 (median of each benchmark within 5
    percent), then `cargo xtask bench-check`.
*   **Security.** The review view and the human code frame escape control
    characters from source files, so a crafted file cannot send terminal
    escape sequences. The VS Code preview uses read-only virtual documents
    and never writes files; Apply edits the open document only.
*   **Compatibility.** `--review` is a new flag; existing flags, exit codes
    and outputs keep their meaning, including `--changed [REF]`. The note
    wording is human output and may change between releases.
    `origami_syntax` gets a minor-version bump (0.x) for
    `LangResult.rule_spans`.

### Additional Resources

*   [docs/designs/origami.md](origami.md): the formatter design
    this builds on (D6 version check, invariants, Alternative 19).
*   [docs/STYLE.md](../STYLE.md): rule IDs cited in notes.
*   [docs/CLI.md](../CLI.md): CLI contract, streams, exit codes.
*   `concepts_brief.md`, `egm_brainstorm_raw.md`,
    `egm_brainstorm_raw_round2.md`: brainstorm brief and the two review
    rounds.

### Future Improvements

*   **Explanations in MCP and `--diagnostics agent`,** once the wording has
    settled.
*   **Explanations in VS Code** as hovers on the right side of the preview.
*   **"Why was my break removed?"** and other layout changes (re-casing,
    alignment, blank lines).
*   **Exact rule IDs for keyword statements** (`READ TABLE`, `LOOP AT`, …):
    needs a new STYLE rule through the approval gate.
*   **HTML report / WASM playground** (Alternative D).
*   **Desktop app** (Alternative A) when certificates exist and users ask.

--------------------------------------------------------------------------------

## Detailed Implementation Plan

Work happens in five stages. Each stage ends green, as defined in the
Verification Plan; from Stage 2 on, "green" includes the G3 hash comparison
against the G3 baseline.

*   **Stage 0: spike, test gap, baseline.** Items 1–5.
*   **Stage 1: VS Code "Preview Changes"** (no core or CLI change). Items
    6–11.
*   **Stage 2: explain infrastructure.** Types, span recording mechanism,
    explain pass, xtask, and the tests that need no rule spans. Items 12–23
    except the parts marked Stage 3.
*   **Stage 3: rule spans and sentences.** Spans at the code sites, the
    sentence table, the `Explanation:` lines in STYLE.md (approval gate),
    span-dependent tests and `notes.txt` fixtures. Parts of items 15, 16, 18,
    20–22, 27.
*   **Stage 4: `--review` in the CLI and documentation.** Items 24–30.

The **G3 baseline** is the commit at the end of Stage 0, after any
re-recording caused by a Stage 0 fix.

### Target Files to Create/Modify

#### 1. `xtask/src/corpus.rs`, `xtask/src/main.rs` (Modify) — Stage 0

*   **Type / Scope**: Rust, developer tooling.
*   **Proposed Changes**: `corpus::run(bless: bool, hashes:
    Option<PathBuf>)`; main.rs gets the arm `["corpus", "run", "--hashes",
    path]`. For each corpus file and each setting (default; width 80, indent
    4, lower-case keywords) write `<hash> <setting> <path>`, sorted, with a
    64-bit FNV-1a hash written in xtask (no new dependency); a file that fails
    at a setting is written as `ERR <code>`. Also check idempotency at the
    non-default setting. From Stage 2 on, also assert that `explain` returns
    the same result as `format`. Record `target/g3-baseline.txt` at the end
    of Stage 0.

#### 2. `crates/origami_core/tests/rules.rs` (Modify) — Stage 0

*   **Type / Scope**: Rust test.
*   **Proposed Changes**: `CURRENT_MILESTONE = 4`, so the fixtures of the M3
    and M4 rules are checked (today they are skipped). Fix any fixture that
    fails; a fixture fix that reveals a formatter change is handled like a
    Stage 0 fix (item 5).

#### 3. `crates/origami_cli/src/report/mod.rs`, `crates/origami_cli/src/report/human.rs` (Modify) — Stage 0

*   **Type / Scope**: Rust, CLI output.
*   **Proposed Changes**: mod.rs: `pub mod review;` (Stage 4),
    `pub(crate) fn escape_controls(&str) -> Cow<str>` (C0 except tab, DEL,
    C1, bidi controls U+202A–U+202E and U+2066–U+2069 become `\xNN` or
    `\u{NNNN}`) and `pub(crate) fn color_enabled(stream: Stream) -> bool`
    (moved from human.rs, with a stdout/stderr parameter). human.rs: the code
    frame uses `escape_controls`; the caret position is computed on the
    escaped prefix.

#### 4. `crates/origami_cli/src/args.rs`, `docs/CLI.md` (Modify) — Stage 0

*   **Type / Scope**: Rust CLI arguments and docs.
*   **Proposed Changes**: no behavior change for `--changed`. Its help text
    and CLI.md say that `--changed` takes the next argument as the revision
    unless it starts with `-`; to give paths, write `--changed=REF path…` or
    put the paths first. CLI test for this behavior (item 26).

#### 5. Spike: `crates/origami_layout/src/printer.rs`, `crates/origami_abap/src/statements.rs` (Investigate) — Stage 0

*   **Type / Scope**: tests first, fixes only if needed.
*   **Proposed Changes**: write tests that show the behavior of (a) a
    comment after the period of a `CALL FUNCTION` statement
    (`call_with_groups`, and the forced-break check in printer.rs) and (b) the
    fill check in printer.rs and the statement's trailing period. If a test
    shows a defect, fix it in a separate commit; such a fix may change
    output, gets a CHANGELOG entry with its rule ID, and is part of the G3
    baseline. If not, record the finding in this document.

#### 6. `editors/vscode/src/previewUri.ts` (Create) — Stage 1

*   **Type / Scope**: TypeScript, no `vscode` import.
*   **Proposed Changes**: `encode({ id, side, name }) -> string` and
    `decode(string)` for URIs `origami-preview:/<name>?side=before|after&id=<n>`.

#### 7. `editors/vscode/src/preview.ts` (Create) — Stage 1

*   **Type / Scope**: TypeScript.
*   **Proposed Changes**: `PreviewProvider` (`TextDocumentContentProvider`
    for `origami-preview`) with a `Map<number, Preview>` (`Preview { id,
    source: Uri, version, before, after }`). `previewChanges(editor)`: takes
    the snapshot, calls `formatText` with a 5-second timeout, then shows a
    message (nothing to change; structural error, with diagnostics through
    `reportOutcome`; preview failed) or opens `vscode.diff(before, after,
    "<name>: origami preview")`. `applyPreview(uri?)`: uses `uri` from the
    title bar or the active diff; applies `computeEdits(before, after)` if the
    version is unchanged, else formats the current text and applies those
    edits; closes the diff tab; messages for a closed document or a failed
    re-format. Maintains the context key `origami.previewActive` from
    `onDidChangeActiveTextEditor` and tab changes.

#### 8. `editors/vscode/src/format.ts` (Modify) — Stage 1

*   **Type / Scope**: TypeScript.
*   **Proposed Changes**: split `formatDocument` into `formatText(text,
    document, resolved, token, log, ranges, timeoutMs)`, which returns the
    outcome with the formatted text, and a thin `formatDocument` wrapper that
    keeps today's behavior (2-second timeout, edits against the document).

#### 9. `editors/vscode/src/extension.ts` (Modify) — Stage 1

*   **Type / Scope**: TypeScript.
*   **Proposed Changes**: extract the diagnostics and status-bar code into
    `reportOutcome(document, outcome, timeoutMs)` (the timeout text uses the
    given value) and use it for formatting and preview; register the content
    provider and the commands `origami.previewChanges` and
    `origami.applyPreview`; maintain the context key `origami.formattable`
    (active document matches `SELECTOR` via `vscode.languages.match`).

#### 10. `editors/vscode/package.json`, `editors/vscode/tsconfig.test.json` (Modify) — Stage 1

*   **Type / Scope**: extension manifest, test config.
*   **Proposed Changes**: contribute "origami: Preview Changes" and "origami:
    Apply Preview"; `menus.editor/title`: Apply with
    `when: resourceScheme == origami-preview`, `group: navigation`, icon
    `$(check)`; `menus.commandPalette`: Preview `when: origami.formattable`,
    Apply `when: origami.previewActive`. tsconfig.test.json includes
    `src/previewUri.ts`.

#### 11. `editors/vscode/test/e2e/suite.ts`, `editors/vscode/test/previewUri.test.ts` (Modify / Create), `editors/vscode/README.md` (Modify) — Stage 1

*   **Type / Scope**: tests and docs.
*   **Proposed Changes**: e2e: the preview opens a diff whose right side
    equals the formatted text; Apply makes the document equal to it; Apply
    after an edit keeps the edit; a structural error opens no diff and sets a
    diagnostic; a formatted file shows "nothing to change". Unit: URI
    round trip. README: Preview Changes and Apply.

#### 12. `crates/origami_syntax/src/explain.rs` (Create), `crates/origami_syntax/src/lib.rs`, `crates/origami_syntax/src/lang.rs`, `crates/origami_syntax/Cargo.toml` (Modify) — Stage 2

*   **Type / Scope**: Rust, shared types.
*   **Proposed Changes**: the types of Technical Plan section 1 (`RuleId`,
    `RuleSpan`, `Scope`, `BreakCause`, `Note`), re-exported from lib.rs;
    `LangResult.rule_spans` (empty by default, also in the error
    constructor); minor-version bump.

#### 13. `crates/origami_abap/src/stmt.rs` (Modify) — Stage 2 (mechanism), Stage 3 (spans)

*   **Type / Scope**: Rust.
*   **Proposed Changes**: Stage 2: `Ctx` gets `rule_spans`, `explain`,
    `Mark`, `mark()`, `reset()`, `span()`; the keyword save/truncate patterns
    and the raw fallbacks use `mark()` / `reset()`. Stage 3: spans in
    `call_args` (ABAP-CALL-001 for method calls, ABAP-CALL-002 per keyword
    group; new parameter method call vs. constructor), `ctor_body`
    (ABAP-VALUE-001), `logical` (ABAP-COND-001).

#### 14. `crates/origami_abap/src/statements.rs` (Modify) — Stage 2 (mechanism), Stage 3 (spans)

*   **Type / Scope**: Rust.
*   **Proposed Changes**: Stage 2: `mark()` / `reset()` at the save/truncate
    patterns and raw fallbacks. Stage 3: spans in `declaration`
    (ABAP-DECL-001), `spec_chain` (ABAP-CHAIN-001), `method_item_inner`
    (ABAP-METHODS-001, ABAP-FORM-001), `call_with_groups` (ABAP-CALL-003),
    `keyword_statement` for specs with `all_clauses` (ABAP-SQL-001),
    `with_select` and `subquery` (ABAP-SQL-002), `eml` (ABAP-EML-001).

#### 15. `crates/origami_abap/src/layout.rs`, `crates/origami_abap/src/lib.rs` (Modify) — Stage 2

*   **Type / Scope**: Rust.
*   **Proposed Changes**: `layout::layout` takes `explain: bool` and returns
    the spans in `Laid`; lib.rs adds `format_explained(...)` (same as
    `format` with `explain = true`) and fills `LangResult.rule_spans`.

#### 16. `crates/origami_cds/src/grammar.rs`, `crates/origami_cds/src/layout.rs`, `crates/origami_cds/src/lib.rs` (Modify) — Stage 2 (mechanism), Stage 3 (spans)

*   **Type / Scope**: Rust, CDS layout.
*   **Proposed Changes**: Stage 2: `G::new(text, stream, lang, explain)`,
    `G.spans`, helpers append local spans only on success;
    `layout::layout(text, stream, lang, explain) -> (Doc, Vec<RuleSpan>)`;
    lib.rs adds `format_explained(...)`. Stage 3: spans in `annotation`
    (CDS-ANNO-001), `header` (CDS-HEAD-001), `case` expressions inside
    `element` (CDS-ELEM-001), `grant` (DCL-GRANT-001).

#### 17. `crates/origami_core/src/check.rs` (Modify) — Stage 2

*   **Type / Scope**: Rust, core.
*   **Proposed Changes**: `Checked` carries the final quarantine and opaque
    sets and `whole_file_fallback`; `print_wide(src, lang, opts, identity,
    &Checked)`; `relex` becomes `pub(crate)`; `select_lines` also returns the
    kept changed blocks (output line range → mixed line range).

#### 18. `crates/origami_core/src/explain.rs` (Create), `crates/origami_core/src/lib.rs` (Modify) — Stage 2, sentence table Stage 3

*   **Type / Scope**: Rust, core.
*   **Proposed Changes**: Stage 2: `format_inner(..., explain)` returning
    `Inner`; `pub fn explain(...) -> Explained`; the steps of Technical Plan
    section 3 inside one `catch_unwind`; the cause-only sentences and the
    ABAP-STMT-001 / CDS-UNIT-001 rule phrases. Stage 3: the full sentence
    table from the appendix.

#### 19. `crates/origami_core/tests/explain.rs`, `crates/origami_core/tests/explain/` (Create) — Stage 2 and 3

*   **Type / Scope**: Rust tests and checked-in samples.
*   **Proposed Changes**: samples (about 6 files: class with calls,
    SELECT/CTE, EML, report with FORM, CDS view, DCL), each with
    `expected.notes`. Stage 2 tests: `explain().result` equals
    `format_lines()` for every sample and rule fixture; a statement sharing a
    line gets an ABAP-STMT-001 note; an end-of-line comment gets a comment
    note; kept breaks get no note; quarantine units and the whole-file
    fallback give no notes; `--lines` filtering and mapping; a failed parse
    attempt leaves no span. Stage 3 tests: a too-long call gets one
    ABAP-CALL-001 width note with N > 120; a CALL FUNCTION statement with a
    keyword group gets CALL-003/CALL-002 notes per cause; a CDS header clause
    gets CDS-HEAD-001; every rule phrase equals the `Explanation:` line of
    its rule in STYLE.md.

#### 20. `crates/origami_core/tests/rules.rs`, `crates/origami_core/tests/catalog.rs`, `crates/origami_core/tests/rules/<ID>/notes.txt` (Modify / Create) — Stage 2 and 3

*   **Type / Scope**: Rust tests and fixtures.
*   **Proposed Changes**: Stage 2: rules.rs and catalog.rs also assert
    `explain(..).result == format(..)`. Stage 3: if a rule directory has
    `notes.txt`, the notes for `input.*` must equal it (one line per note:
    `<first line>: <text>`); added for each rule with a span.

#### 21. `xtask/src/explain.rs` (Create), `xtask/src/main.rs` (Modify) — Stage 2

*   **Type / Scope**: Rust, developer tooling.
*   **Proposed Changes**: `cargo xtask explain <file>` and `--corpus` with
    the checks (a)–(d) of Technical Plan section 6; exit 1 on a failed check.

#### 22. `docs/STYLE.md` (Modify) — Stage 3

*   **Type / Scope**: docs.
*   **Proposed Changes**: an `Explanation: <rule phrase>` line directly after
    the `Milestone:` line of each rule in the appendix; the "How rules work"
    Format bullet mentions it; a short section on how notes are built. The
    new lines are a rule change and go through the approval gate.

#### 23. `Cargo.toml` (workspace) (Modify) — Stage 4

*   **Type / Scope**: build configuration.
*   **Proposed Changes**: `similar = { version = "3.2", features =
    ["inline"] }`.

#### 24. `crates/origami_cli/src/report/review.rs` (Create) — Stage 4

*   **Type / Scope**: Rust, CLI output.
*   **Proposed Changes**: `write(path, &Explained, color, out)` renders the
    view of Technical Plan section 4: header (path, counts, CRLF/BOM),
    changes with 2 lines of context, old/new line numbers, word-level
    highlighting, `[-…-]` / `{+…+}` without colors, notes after the last added
    line of their construct, control characters escaped.

#### 25. `crates/origami_cli/src/args.rs`, `crates/origami_cli/src/run.rs` (Modify) — Stage 4

*   **Type / Scope**: Rust.
*   **Proposed Changes**: args.rs: `--review` (help: "Show the changes
    origami would make, with notes on introduced line breaks; writes
    nothing"), `conflicts_with_all = ["diff", "write"]`; `Args::check_mode()`
    is true for `--check` or `--review`. run.rs: every read of `args.check`
    (stdin output, diff, write decision, exit code, `Run.check`) uses
    `check_mode()`; with `--review`, stdin and path mode call `explain` and
    write the review view to stdout for changed files.

#### 26. `crates/origami_cli/tests/cli.rs` (Modify) — Stage 0 and 4

*   **Type / Scope**: Rust tests.
*   **Proposed Changes**: Stage 0: `--changed REF path` takes REF as the
    revision (documented behavior); the code frame escapes control
    characters. Stage 4: `--review` on a changed file prints the view with
    line numbers and a note and exits 1; on a formatted file prints nothing
    and exits 0; with a structural error exits 2 without a view; never
    modifies a file; rejected with `--diff` and `--write` (exit 3);
    `--review --changed=…` shows only changed lines; no escape sequences
    without a terminal; control characters in the source are escaped.

#### 27. `docs/CLI.md`, `docs/ci.md`, `README.md`, `docs/adoption.md`, `CHANGELOG.md` (Modify) — Stage 4 (CHANGELOG also Stage 0)

*   **Type / Scope**: docs.
*   **Proposed Changes**: CLI.md: `--review` in usage, the streams rule, the
    stdin rule for `--review`, the review view; README: `--review` and VS Code
    Preview Changes; adoption.md: evaluate with `origami --review` and the
    `origami --check --diff | delta` recipe (Alternative K); CHANGELOG:
    entries for each stage, Stage 0 output changes with their rule IDs, the
    `origami_syntax` minor-version bump.

#### 28. `.github/workflows/ci.yml` (Modify) — Stage 2

*   **Type / Scope**: CI.
*   **Proposed Changes**: the corpus job runs `cargo xtask explain --corpus`
    after `cargo xtask corpus run`.

#### 29. `docs/designs/origami-explain-review.md` (Modify) — after Stage 4

*   **Type / Scope**: docs.
*   **Proposed Changes**: status "Approved"; record the spike results
    (item 5).

### Verification Plan

*   **Compilation/Build**: `cargo build --workspace`; `npm run build` in
    `editors/vscode`.
*   **Unit Tests**: `cargo test --workspace`; `cargo test -p origami_core
    --features fault-injection`; `npm test` in `editors/vscode`.
*   **Static Analysis & Linting**: `cargo fmt --all --check`; `cargo clippy
    --workspace --all-targets -- -D warnings`; `npx eslint .` and `npx tsc
    --noEmit` in `editors/vscode`.
*   **Test Coverage Requirement**: `cargo llvm-cov --workspace --exclude
    xtask --features origami_core/fault-injection --fail-under-lines 85` (as
    in CI).
*   **E2E/Integration**: `npm run test:e2e` (VS Code preview tests);
    `cargo xtask corpus run` (clean and idempotent); `cargo xtask oracle`
    (0 mismatches); `cargo xtask stress 20000` with 4 seeds (0 failures);
    `cargo xtask explain --corpus` (checks (a)–(d) pass; `Layout` count
    reported).
*   **Formatting unchanged (G3)**: `cargo xtask corpus run --hashes
    target/g3-current.txt`, compared with `target/g3-baseline.txt`; identical
    at the end of Stages 2, 3 and 4.
*   **Performance (G9)**: `cargo bench -p origami_core` on the G3 baseline
    and after Stage 4, medians within 5 percent; then `cargo xtask
    bench-check`.

### Appendix: Note sentences

| Rule | Rule phrase (`Explanation:` line) | Construct noun |
|---|---|---|
| ABAP-STMT-001 | each statement on its own line | statement |
| ABAP-CALL-001 | one parameter per line | call |
| ABAP-CALL-002 | each parameter group on its own line | call |
| ABAP-CALL-003 | each parameter group on its own line | call |
| ABAP-VALUE-001 | one row per line | constructor expression |
| ABAP-COND-001 | condition split before AND, OR | condition |
| ABAP-DECL-001 | one declaration per line | declaration |
| ABAP-CHAIN-001 | one item per line | chain |
| ABAP-METHODS-001 | each section and parameter on its own line | method signature |
| ABAP-FORM-001 | each section and parameter on its own line | FORM signature |
| ABAP-SQL-001 | each clause on its own line | statement |
| ABAP-SQL-002 | each common table expression on its own line | query |
| ABAP-EML-001 | each entity and operation on its own line | statement |
| CDS-UNIT-001 | each annotation and element on its own line | element |
| CDS-ANNO-001 | annotation values on their own lines | annotation |
| CDS-HEAD-001 | each clause of the header on its own line | header |
| CDS-ELEM-001 | each WHEN on its own line | case expression |
| DCL-GRANT-001 | condition split before AND, OR | access condition |

Cause sentences (`{…}` filled from the note):

*   width, construct measured: "{RULE}: {phrase}, because the {noun} would
    end at column {N} (limit {W})";
*   width, line measured: "{RULE}: {phrase}, because the line would end at
    column {M} (limit {W})";
*   width without a rule: "split because the {noun} would end at column {N}
    (limit {W})", noun "statement" (ABAP) or "element" (CDS);
*   rule: "{RULE}: {phrase}";
*   rule without a span: "line break required by the layout of this
    {noun}";
*   comment: "split because of the comment `{text}`";
*   layout: "laid out on several lines to fit the layout rules".

Known gap: STYLE.md's ABAP-SQL-001 also names database INSERT, UPDATE,
MODIFY and DELETE, but these statements use the fill layout; their notes
name the cause only until the implementation or the rule text is aligned.

--------------------------------------------------------------------------------

*Drafted with EGM*
