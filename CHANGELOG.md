# Changelog

All notable changes to origami are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Within a style
edition, every output change is listed with its STYLE rule ID.

## [Unreleased]

### Added

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
