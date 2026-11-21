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
