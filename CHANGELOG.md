# Changelog

All notable changes to origami are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Within a style
edition, every output change is listed with its STYLE rule ID.

## [Unreleased]

### Added

- M1 (ABAP): printer with groups, hanging indentation, fallback choice,
  alignment groups and fill-style clauses; ADT-style block indentation; one
  statement per line; blank-line rules; statement parsers for assignments,
  calls, constructor expressions, logical expressions, declarations and
  chains (A3), method signatures, ABAP SQL `SELECT` and about 50 keyword
  statements; keyword upper case (STYLE ABAP-*, batch 1).
- ABAP EML (`READ`/`MODIFY ENTITIES`, `COMMIT ENTITIES`, `GET PERMISSIONS`,
  `SET LOCKS`): STYLE ABAP-EML-001.
- ABAP: `CALL FUNCTION`/`CALL METHOD` parameter groups (ABAP-CALL-003), `METHODS:`/`CLASS-METHODS:` chains, trailing pragmas,
  selectors after constructor expressions, `CREATE DATA`, `FIND`,
  `REPLACE`, `AUTHORITY-CHECK`, `CALL TRANSFORMATION`, `UPDATE`,
  `RAISE SHORTDUMP`.
- CDS: annotation values laid out like JSON when they do not fit
  (CDS-ANNO-001); DDLS and BDEF headers one clause per line
  (CDS-HEAD-001).
- Robustness: `cargo xtask stress` (mutation testing on stable Rust, in CI);
  80,000 mutated inputs without failures after these fixes: a `*` token is
  never printed in column 1; block comments inside a unit keep their bytes;
  call arguments with an unhandled keyword group fall back instead of
  dropping tokens; `message = …` and other assignments to variables named
  like statements are assignments; `TYPE any` / `TYPE line` stay type names.
- Fixes (spike for the explain design, Stage 0):
  - ABAP-WIDTH-001: the last clause of a filled keyword statement is checked
    together with the rest of its line (the period), so such lines no
    longer end at column 121; 42 corpus lines were over the limit.
  - ABAP-COMMENT-002: an end-of-line comment after the statement's period no
    longer forces the statement to break (`TABLES: a, b. " c` stays on one
    line).
  - The human code frame escapes control and bidi characters from the
    source.
  - The rule tests now check all milestones (M3/M4 fixtures were skipped).
  - `--changed` documents that it takes the next argument as the revision.
  - `cargo xtask corpus run --hashes FILE` for byte-for-byte output checks.
- Output: new human format (per-file blocks with symbols, code frames for
  errors, compact W003, summary with counts and run time, next-step hint;
  colors on terminals, `NO_COLOR` / `CLICOLOR_FORCE`) and a new
  `--diagnostics agent` format for AI agents (one line per finding with a
  fix hint, a final `NEXT:` line); agent output is the default when
  `CLAUDECODE` is set; `ORIGAMI_DIAGNOSTICS` sets the default.
- CI: GitHub Action (`action.yml`: downloads the release binary, checks the
  checksum, optional SARIF upload to code scanning) and GitLab / Azure
  Pipelines templates (`docs/ci.md`).
- MCP server: `origami --mcp` with the tools `format` and `format_files`
  (check or write, optionally only changed lines).
- Partial formatting: `--lines START:END`, `--changed [REF]` (git), core
  `format_lines`; VS Code *Format Selection* and format-on-save for modified
  lines only. The kept changes pass the self-check; otherwise the file stays
  as written (W001).
- VS Code: "origami: Preview Changes" opens the built-in diff view (the
  document as it is, the formatted text); "origami: Apply Preview" applies
  it and reformats instead if the document was edited meanwhile.
- Line-break explanations (core): `origami_core::explain` formats like
  `format_lines` and returns one note per construct and cause for the line
  breaks origami introduced (width with the measured column, a style rule,
  or an end-of-line comment); `cargo xtask explain FILE` and
  `cargo xtask explain --corpus` (checked in CI). Formatting output is
  unchanged (corpus hashes identical to the Stage 0 baseline).
- `origami_syntax`: new `explain` module (`RuleSpan`, `Note`, `BreakCause`,
  `Scope`, `RuleId`); `LangResult` has a new field `rule_spans` (code that
  builds `LangResult` with a struct literal must add it). The ABAP and CDS
  crates have `format_explained`.
- Dynpro flow logic (`*.screen_<nr>.abap`) is formatted: `PROCESS` blocks
  indented like event blocks, `CHAIN … ENDCHAIN`, table-control loops,
  `MODULE` calls, `FIELD`.
- Statement catalog up to SAP_BASIS 8.16: all 4,576 example statements and
  333 CDS sources from abaplint's tests (`corpus/catalog/`, MIT) run in a
  test; no internal errors, no false structural errors, idempotent; 2
  statements without grammar (both contrived). New: ABAP SQL
  `MERGE INTO` (8.16), `WITH +cte … ` incl. `WITH ASSOCIATIONS`/`HIERARCHY`
  (ABAP-SQL-002), `OPEN CURSOR … FOR SELECT`, `DELETE … ORDER BY`,
  SQL functions and path expressions; EML `OPERATIONS`; `RAISE RESUMABLE`,
  `WAIT FOR … UNTIL`, `CALL FUNCTION … EXCEPTIONS OTHERS`, `CALL METHOD OF`
  (OLE), about 80 rare and obsolete statements; `&`, `BIT-NOT`, unary signs,
  chained assignments, qualified constructor types, leading pragmas.
- Keywords inside constructor expressions (`FOR … IN`, `BASE`, `MAPPING`,
  `EXCEPT`, `DISCARDING DUPLICATES`, `WHEN … THEN`, `LET`, `INIT`/`NEXT`,
  `IS [NOT] INITIAL`, `IS NULL`, …) and in RAP handler and event
  declarations (`FOR VALIDATE ON SAVE`, `keys FOR ACTION … RESULT result`,
  `FOR EVENT x OF y`) are re-cased.
- Fixes: `AT fg … ENDAT` in extract loops is a block (was E004); CDS string
  escapes `\'` (was E001); a CDS header never breaks inside `{ … }`.
- Robustness (longer fuzzing): a new line over 255 characters that remains
  after quarantine leaves the whole file as written (INVARIANTS 2); an
  opaque unit starting with `*` never lands in column 1; an unclosed pragma
  parameter (`##X[ `) is not part of the pragma; CDS: a one-line block is
  part of its unit only right after the previous token, and it ends the
  unit (fixes wrong indentation after `define authorization context … { … }`
  and keeps one blank line between BDEF statements). One finding on
  binary garbage with nested braces is open (`fuzz/found/open-*`).
- Classic ABAP (M4 decision: full grammar): event blocks indented
  (ABAP-EVENT-001), structures across `DATA`/`TYPES` statements
  (ABAP-DECL-002), `FORM` signatures like `METHODS` (ABAP-FORM-001);
  `REPORT`, `PARAMETERS`, `SELECT-OPTIONS`, `SELECTION-SCREEN`, `PERFORM`,
  `FUNCTION`, `WRITE` (also `WRITE /: a, b.`), `RAISE`, `CONTROLS`, list,
  memory, dataset and cursor statements. Chains may have keywords before the
  colon and items glued to their comma. Obsolete `SELECT … ENDSELECT`
  without `INTO` is a loop. Dynpro flow logic (`*.screen_<nr>.abap`) is
  skipped. Corpus: two classic repositories added (abap2xlsx demos, ABAP
  SDK for Azure); ABAP fallback 1.6% → 0.2%.
- Fuzzing (cargo-fuzz, first local run): fixed four findings, kept as
  regression tests in `fuzz/found/`: line-break style could flip between
  runs with `\r\r\n`; a vertical tab ending a comment was dropped; a
  `/* */` comment between tokens on one line was dropped in unknown units
  (caught by the self-check). Two further idempotency findings on binary
  garbage input were fixed later (see Robustness).
- `ASSIGN structure TO <fs>`: a first operand named like a keyword stays a
  name (found by comparing with ABAP Cleaner output).
- ABAP: chains of keyword statements (ABAP-CHAIN-001); `SHIFT`,
  `MOVE-CORRESPONDING`, `CONVERT DATE/TIME STAMP/UTCLONG`, `WAIT UP TO`,
  `TEST-SEAM`/`TEST-INJECTION` and their ends. ABAP fallback 1.3% → 0.9%.
- Performance comparison with ABAP Cleaner 1.29.2 on 1,652 ABAP files
  (`docs/benchmark-abap-cleaner.md`).
- VS Code extension: ESLint (typescript-eslint strict) and end-to-end tests
  in a real VS Code instance (`npm run test:e2e`: activation, ABAP and CDS
  formatting, structural error as diagnostic, idempotency); both in CI.
- CDS select lists: `case` expressions laid out by grammar (CDS-ELEM-001); DCL `grant select on … where …` compact (DCL-GRANT-001).
- Release workflow (`release.yml`): 7 targets, optional signing,
  checksums, third-party notices, GitHub release, crates.io, VSIX per
  platform.
- CLI benchmark (`scripts/bench-cli.sh`, bash 3.2 compatible): p95 per
  file 1.5–2.0 ms including process start, 1,000 files 107 ms.
- CDS headers: a clause that does not fit breaks before `on`, `and`, `or`;
  clauses that fit stay on one line.
- CI: coverage measured workspace-wide (87%, gate 85%); tests for
  `--check --diff`, write mode and token comparison; paths found by
  walking `.` are shown without `./`.
- Write mode no longer forces each file to disk (fsync): formatting the RAP
  reference project (346 files) takes 0.06 s instead of 0.9 s.
- Printer unit tests; `docs/adoption.md` (team adoption guide).
- VS Code extension (`editors/vscode`): formatter for `abap`/`acds` and
  abapGit files, binary resolution with `required_version`, minimal edits,
  Problems panel, untrusted-workspace handling.
- CDS family: structural layout (STYLE CDS-*).
- M0 walking skeleton:
  - lossless lexers for ABAP and the CDS family;
  - structural checks (E001–E005), with the block-pair table and the
    `SELECT`/`WITH` loop rule;
  - verbatim regions for AMDP, `EXEC SQL` and macros;
  - language detection by file name and content;
  - self-check with quarantine (W001/W002) and X001/X002;
  - identity printer;
  - CLI with stdin mode, `--check`, `--diff`, JSON/SARIF diagnostics,
    `origami.toml` and `--version --json`;
  - corpus pipeline (`cargo xtask corpus`), statement frequencies, abaplint
    oracle and benchmark checks.
