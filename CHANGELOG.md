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
