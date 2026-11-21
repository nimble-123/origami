# origami: a layout-only formatter for ABAP and the CDS family

*   **Author:** Nils Lutz
*   **Status:** Proposed
*   **Date:** 2026-11-21
*   **Product name:** origami (CLI `origami`; crates.io and npm package `abap-origami`; internal crates `origami_*`; all checked free on crates.io and npm on 2026-11-21, trademark search still open)

--------------------------------------------------------------------------------

## Overview

origami is a code formatter for SAP ABAP development that runs on a developer machine, in CI and in AI agent sandboxes. It needs no SAP system. It formats six source languages:

| Language | abapGit object types | What it contains |
|---|---|---|
| ABAP | CLAS, INTF, PROG, FUGR | Classes, interfaces, programs, function groups |
| CDS DDL | DDLS | Data definitions: view entities, projections, custom and abstract entities, table functions, hierarchies, extends |
| CDS DCL | DCLS | Access controls (roles) |
| CDS DDLX | DDLX | Metadata extensions (UI annotations) |
| RAP BDEF | BDEF | Behavior definitions |
| SRVD | SRVD | Service definitions |

The formatter changes layout only. It may change:

*   the width of existing gaps between tokens;
*   line breaks and indentation;
*   the letter case of ABAP keywords.

It never adds or removes a gap between two tokens, never re-cases identifiers, and never rewrites statements. After every run it checks its own output against the input and refuses to write if the two differ in anything but layout.

The style is fixed, in the spirit of Prettier and rustfmt: the formatter recomputes line breaks from the syntax and a 120-character width, regardless of how the author broke the lines. Teams can change only three settings: ABAP keyword case, line width and indent.

It ships as:

*   a Rust command-line tool for files and stdin/stdout, built for pre-commit hooks, CI and agent loops;
*   a VS Code extension that formats abapGit files on disk and the virtual `abap:` documents of SAP's "ADT for VS Code".

## The Problem

ABAP development is moving off the SAP stack. Developers keep code in git with abapGit, edit in VS Code with SAP's ADT for VS Code, and let AI agents write and change code. None of today's tools gives these workflows a formatter that is offline, deterministic, layout-only and covers the RAP languages.

What exists today and why it falls short:

1.  **The ADT pretty printer (Eclipse, Shift+F1)** only runs in Eclipse or through a connection to an SAP system.
    *   It handles ABAP keyword case and indentation, but does not break long lines.
    *   A CLI, a pre-commit hook or an agent sandbox without SAP access cannot call it.
    *   ADT for VS Code registers formatting through its language server. We assume it needs a system connection too, but this is not yet verified (spike S2).
2.  **ABAP Cleaner** (SAP, Apache-2.0) has a CLI and a VS Code extension.
    *   It runs on the JVM, so each cold CLI call pays the JVM start-up. The VS Code extension hides this with a Java daemon; hooks and agents cannot.
    *   About half of its rules rewrite code (inline declarations, replacing obsolete statements). A layout-only run needs a hand-maintained profile.
    *   It covers ABAP and CDS views, but not BDEF, SRVD, DCL or DDLX.
    *   Its VS Code extension registers for `file` and `vscode-vfs` documents, not for ADT's `abap:` documents.
3.  **abaplint** (MIT) has an open ABAP parser and a few auto-fixes (keyword case, indentation). It does not break lines, and does not parse BDEF or SRVD.
4.  **Nothing formats BDEF, SRVD, DCL or DDLX** outside an SAP system.

The result:

*   Every developer and every agent produces a different layout.
*   Code reviews and diffs carry layout noise.
*   Agents waste tokens re-reading layout-only diffs.
*   Pre-commit hooks cannot enforce any style, because nothing can run there fast and offline across all six languages.

**Proposed solution:** one Rust core that formats all six languages with one fixed style. It proves on every run that only layout changed, and runs as a native CLI and inside VS Code.

## Goals

Each goal has a measurable check. "Corpus" means the pinned test corpus defined in the Technical Plan: public abapGit repositories plus SAP sample repositories with RAP content.

1.  **G1 – Meaning is never changed.**
    *   The token sequence, token adjacency (whether two tokens touch), comment and literal text, and comment-to-line attachment of the output are equal to the input's.
    *   Check: zero violations on the corpus. The runtime self-check runs on every format and blocks the write on a violation.
2.  **G2 – Output is idempotent.** Formatting already formatted output changes nothing. Check: 100% on the corpus, including files that use the fallback.
3.  **G3 – Six languages covered.** ABAP, CDS DDL, DCL, DDLX, BDEF and SRVD are formatted in the abapGit file patterns and as ADT `abap`/`acds` documents. Check: per-language fallback rate on the corpus is reported in CI, and does not rise between releases.
4.  **G4 – Fast enough to run on every save, commit and agent step.** Reference machines: an Apple Silicon laptop and a GitHub-hosted `ubuntu-latest` runner (4 vCPU for public repositories).

    | Measure | Target |
    |---|---|
    | In-process format of a typical file (p50 size per language) | under 10 ms |
    | CLI end-to-end, one file, including process start (p95) | under 50 ms |
    | CLI over 1,000 typical files | under 1 s |
    | Worst case: one 50,000-line file | under 500 ms, no crash |

5.  **G5 – Syntax the formatter does not know is not a failure.**
    *   A statement that is lexically and structurally sound, but unknown to the grammar, is formatted generically and reported as warning W003 (W004 when the nesting-depth limit is hit).
    *   Only structural errors leave a file untouched.
    *   Check: zero structural errors on corpus files that activate in an SAP system.
6.  **G6 – A contract agents and CI can rely on.**
    *   stdin/stdout mode, `--check`, documented exit codes, versioned JSON diagnostics with file, line and column, and a `changed` flag.
    *   Check: the contract is covered by tests and frozen at v1.0.
7.  **G7 – One fixed style, three settings.**
    *   ABAP keyword case (default UPPER), line width (default 120) and indent (default 2).
    *   CDS-family keywords stay lower case. Identifiers are never re-cased.
    *   Each style rule is written down in a STYLE document with a rule ID and examples before it is implemented.
8.  **G8 – Works in VS Code for both sources.**
    *   abapGit files on disk and ADT for VS Code `abap:` documents can be formatted with "Format Document" and format-on-save.
    *   The extension returns minimal text edits.

## Non-Goals

1.  **Semantic rewrites.** No inline declarations, no replacing obsolete statements, no reordering. That is ABAP Cleaner's job.
2.  **Matching other formatters.** The output does not try to match the Eclipse pretty printer or ABAP Cleaner.
3.  **Syntax checking.** The formatter decides only whether it can format safely. Whether code is valid is decided by the SAP system or abaplint.
4.  **Re-casing identifiers.** In any language. In CDS, alias case becomes the OData property name.
5.  **Upper-casing CDS-family keywords.**
6.  **Formatting embedded languages.** SQLScript in AMDP bodies, `EXEC SQL … ENDEXEC` and macro bodies pass through unchanged.
7.  **Formatting abapGit XML metadata.** abapGit re-serializes these files itself.
8.  **Range formatting, format-on-type and a language server** in v1.
9.  **Formatting files with structural errors.** Partial formatting is not attempted.
10. **Classic ABAP grammar** (reports, selection screens, FORM routines) before the M4 decision. Until then it goes through the fallback.

## Open Questions

Questions 1 to 3 are experiments. They are scheduled as spikes in milestone M0, each with a decision it feeds.

1.  **S1 – Round-trip through SAP.**
    *   Experiment: format a file, then write it into an SAP system through ADT for VS Code and through an abapGit pull. Activate it, serialize it back and diff.
    *   Decision it feeds: if any path rewrites layout, the CI `--check` gate produces false failures. The product is then positioned as an authoring tool, not a CI gate.
2.  **S2 – ADT for VS Code behavior.**
    *   Does the `acds` language ID cover DDLS, DDLX, DCLS, BDEF and SRVD?
    *   Do `abap:` URIs encode the object type?
    *   Can a third-party formatter edit `abap:` documents, locked or not?
    *   Does ADT's own formatter need a system connection?
3.  **S3 – Pilot workflow.**
    *   Who adopts first, and are they system-first (abapGit pushes from SAP) or git-first?
    *   Are agents and CI the primary users, or editor users?
4.  **Programs and classic function groups.** Full grammar, fallback only, or out of v1? ABAP Cloud has no executable programs. Decided at M4.
5.  **Validation system for BDEF and SRVD.** Who provides an SAP system (for example an ABAP Platform trial) to activate formatted sources as an oracle, and under which license terms?
6.  **Project license.** Dual MIT/Apache-2.0 is compatible with both references.
7.  **Code-signing certificates.** Who owns the Apple Developer ID and the Windows signing certificate that D6 requires?

## Critical User Journeys (CUJs)

### CUJ 1: Format on save in ADT for VS Code

*   **Given**:
    *   A developer has a RAP behavior definition open in ADT for VS Code, as a virtual `abap:` document with language ID `acds`.
    *   origami is set as `editor.defaultFormatter` for `abap` and `acds`, and format-on-save is on.
*   **When**: The developer saves.
*   **Then**:
    *   The extension detects BDEF from the content, formats it and applies minimal text edits.
    *   Cursor position and undo history are kept.
    *   If nothing changed, no edit is applied.
    *   If the source has a structural error, the document is not changed, and a diagnostic with line and column appears in the Problems panel.

### CUJ 2: Pre-commit hook in an abapGit repository

*   **Given**: A git-first abapGit repository with a `origami.toml` that pins the style edition, and a pre-commit hook that runs `origami --check` on staged files.
*   **When**: A developer commits a changed class and a changed CDS view.
*   **Then**:
    *   Both files are checked in well under one second.
    *   If either would change, the commit is blocked with exit code 1 and the file names.
    *   `origami --write` fixes them.
    *   Files the formatter does not handle (XML, `.ddls.baseinfo`) are skipped without error.

### CUJ 3: AI agent loop on ADT objects

*   **Given**: An agent edits ABAP objects through the ADT MCP server, which returns source text, not files.
*   **When**: The agent pipes the new source into `origami --stdin-filepath zcl_foo.clas.abap --diagnostics=json`.
*   **Then**:
    *   The formatted source comes back on stdout.
    *   A JSON result on stderr says `changed: true|false`, lists diagnostics with stable codes, and gives an exit code.
    *   On a structural error, stdout is empty and the exit code is 2, so the agent does not write an empty object back.
    *   With `changed: false` the agent skips the MCP write.

### CUJ 4: CI gate on a pull request

*   **Given**: A CI job runs `origami --check --diagnostics=json` over the repository, with the binary version pinned by the repo config.
*   **When**: A pull request contains a file formatted by a different tool.
*   **Then**: The job fails with exit code 1 and a diff summary. The result does not depend on thread count or file order.

### CUJ 5: New SAP syntax the formatter does not know

*   **Given**: A class uses an ABAP statement added in a recent SAP release, which the grammar does not cover yet.
*   **When**: The file is formatted.
*   **Then**:
    *   Every other statement is formatted normally.
    *   The unknown statement is re-indented but keeps its own line breaks and letter case.
    *   Warning W003 names the statement's line.
    *   The exit code is 0, unless `--strict` is set.

## Technical Plan (Architecture)

### Decisions this plan rests on

| ID | Decision | Source |
|---|---|---|
| D1 | Rust core, native CLI, VS Code extension | Brief |
| D2 | Layout only; runtime self-check on every run | Brief + review |
| D3 | Full reflow from the syntax tree (Prettier style) | Brief |
| D4 | Lossless syntax tree with a hand-written recursive-descent parser per language | Author, draft step 3 |
| D5 | ABAP grammar written by hand from the SAP keyword documentation; abaplint used only as reference and test source | Author, draft step 3 |
| D6 | VS Code runs the native binary, one process per format request | Author, draft step 3 |
| D7 | Cross-line alignment only in a fixed list of three cases | Author, draft step 3 |
| D8 | AMDP, `EXEC SQL` and macro bodies are verbatim | Brief + review |

### Components

```text
                 ┌──────────────────────────┐      ┌───────────────────────────┐
                 │ origami (CLI)            │      │ VS Code extension (TS)    │
                 │ files, stdin, --check,   │      │ resolves binary, spawns   │
                 │ JSON diagnostics, -j     │      │ CLI via stdin, applies    │
                 └────────────┬─────────────┘      │ minimal TextEdits         │
                              │                    └─────────────┬─────────────┘
                              │ calls                            │ spawns
                              ▼                                  ▼
                 ┌───────────────────────────────────────────────────────────┐
                 │ origami_core  format(bytes, hint, options) -> FormatResult│
                 │ detect language → language crate → self-check and         │
                 │ quarantine → result + diagnostics                         │
                 └───────┬───────────────────┬───────────────────┬───────────┘
                         ▼                   ▼                   ▼
                 ┌────────────────┐  ┌────────────────┐  ┌────────────────┐
                 │ origami_abap   │  │ origami_cds    │  │ origami_layout │
                 │ lexer, parser, │  │ lexer; DDLS,   │  │ Doc IR, align  │
                 │ layout rules   │  │ DDLX, DCL,     │  │ groups, printer│
                 │                │  │ BDEF, SRVD     │  │ (no language   │
                 │                │  │ parsers+rules  │  │  knowledge)    │
                 └───────┬────────┘  └───────┬────────┘  └────────────────┘
                         └─────────┬─────────┘
                                   ▼
                         ┌────────────────────┐
                         │ origami_syntax     │
                         │ tokens, trivia,    │
                         │ shared types,      │
                         │ token comparison   │
                         └────────────────────┘
```

Dependency rules, enforced by the crate graph:

*   `origami_layout` depends on no language crate.
*   The language crates depend only on `origami_syntax` and `origami_layout`. Types they share with the core (options, units, results) live in `origami_syntax`.
*   `origami_core` is the only crate the CLI uses.
*   The core performs no file, network or environment access and starts no threads. Parallelism over files lives in the CLI.

### Pipeline

1.  **Decode.**
    *   Input must be UTF-8; anything else is structural error E005.
    *   A byte-order mark, the line-ending style (LF or CRLF, decided by the first line break) and the presence of a final newline are recorded and restored on output. Internally every CRLF becomes LF; mixed line endings are written back in the recorded style.
    *   Tabs inside code are treated as whitespace. Tabs inside literals and comments are kept.
2.  **Detect the language.** In this order:
    *   an explicit `--language`;
    *   the file name, using the table below;
    *   for CDS-family text, the first significant keywords after leading annotations and comments: `define view entity`, `define root view entity`, `define transient view entity`, `define view`, `define table entity`, `define table function`, `define scalar function`, `define external entity`, `define hierarchy`, `define custom entity`, `define abstract entity`, `define type` (CDS simple type), `extend view entity` → DDLS; `annotate entity|view` → DDLX; `define role|accesspolicy|aspect|pfcg_mapping` → DCL; `managed|unmanaged|abstract|projection|interface` followed by `implementation`, `extension` or `;`, or `extend behavior for` → BDEF; `define service`, `extend service` → SRVD.
    *   In path mode, a file name that matches no pattern is error E011. In stdin mode, and for the ADT language ID `acds`, an unmatched name falls back to content sniffing.
    *   If sniffing finds nothing, the result is error E010 "language not detected", and the input is left untouched.
3.  **Lex (lossless).** Every byte of the input belongs to exactly one token or one piece of trivia (whitespace, newline, comment). Each token records its kind, its text, its byte span and a flag `glued` that is true when no whitespace separates it from the previous token. The `glued` flag is never changed by the printer.
    *   **ABAP lexer modes:**
        *   normal code;
        *   string templates `|…{ expr }…|`, nested, with escapes `\|`, `\{`, `\}`, `\\`;
        *   character literals `'…'` and backquote literals;
        *   verbatim regions.
    *   **ABAP comment kinds** are distinct trivia types:
        *   full-line `*` (column 1);
        *   `"` comments;
        *   ABAP Doc `"!`;
        *   pseudo-comment `"#EC`.

        Each comment also records its placement: end-of-line when a token precedes it on the same line, own-line otherwise.
    *   **Pragmas** `##NAME[...]` are tokens.
    *   **CDS-family lexer:**
        *   `//` and `/* */` comments in all five languages; `--` comments in DDLS, DDLX and DCL only;
        *   string literals;
        *   numbers;
        *   annotations `@` and `@<`;
        *   `$projection`, `$parameters`, `$session`;
        *   enum values `#VALUE`.
4.  **Structural pass.** This pass decides the only hard errors. It checks:
    *   every literal, template and block comment is closed;
    *   `()`, `[]`, `{}` are balanced;
    *   in ABAP, every statement ends with `.`, and every opener in the block-pair table has its matching closer, in order.

    In the CDS family a missing `;` is not a structural error; the parser's fallback handles it.

    The block-pair table is data, with one code predicate for the loop forms of `SELECT` and `WITH`. Openers match only at the start of a statement, and some forms of an opener have no closer (for example `CLASS … DEFINITION DEFERRED` or `AT SELECTION-SCREEN`). The full table is in item 43 of the implementation plan.

    A failure here produces one of the errors E001–E005 with line and column. The file is left untouched.
5.  **Parse.** One hand-written recursive-descent parser per language builds a lossless syntax tree using the `rowan` crate.
    *   **ABAP:** statements are dispatched on their leading keyword(s).
    *   **Unknown statements:** when no parser function recognizes a statement, or a function fails part-way, the parser does not abort. It wraps the tokens up to the terminating `.` in an `UNKNOWN_STATEMENT` node and continues. In the CDS family the same happens up to the next `;`, `,` in a select list, or closing brace, whichever comes first. Each unknown node produces warning W003 with its line.
    *   **Keyword marking:** only tokens the parser consumed as keywords of a recognized statement are marked `keyword`. Only marked tokens may be re-cased. Words inside unknown nodes are never re-cased.
    *   **Depth limit:** nesting depth is limited to 200. A deeper statement becomes an unknown node with warning W004, so a deeply nested generated file cannot overflow the stack.
    *   **Units:** the language crate also returns a list of units, the pieces the self-check can quarantine. In ABAP a unit is one statement or one whole chain. In the CDS family it is one annotation, one list element, one `;`-terminated statement, or one header.
6.  **Layout.** Each node kind has a layout function that turns the node into a Doc IR, in the style of Wadler's "prettier printer" (as used by Prettier, Biome and Ruff). The IR elements:

    | Element | Meaning |
    |---|---|
    | `Token(i)` | the original token *i*, with its attached comments; the printer copies its text, re-casing it only if it is marked `keyword` |
    | `Space` | exactly one space; only emitted where `glued` is false |
    | `SoftLine` | a space if the group fits, a line break otherwise |
    | `HardLine` | always a line break |
    | `BlankLine` | an empty line; consecutive ones collapse to one |
    | `Group` | content that is laid out flat if it fits in the remaining width, broken otherwise; a group that contains a forced break is always broken |
    | `Indent` | adds one indent level to breaks inside it |
    | `AlignGroup` | rows whose separator column is aligned (D7) |
    | `Unknown(range)` | original text of an unknown statement, re-indented, with runs of spaces shrunk to one |
    | `Verbatim(range)` | original text of a quarantined unit, re-indented only |
    | `Opaque(range)` | original bytes, never re-indented: verbatim regions (D8) and the last-level W002 fallback |

    Gaps, line breaks and comments can only be created through a builder module. It emits nothing where the following token is glued, and emits every comment together with the token it is attached to. Rules every layout function must follow:
    *   A gap between two tokens exists in the output only if `glued` is false. A `glued` pair is never separated, not even by a line break.
    *   Continuation lines of a statement are always indented by at least one level. A wrapped line can therefore never start in column 1, so it can never begin with `*` there.
    *   An end-of-line comment forces a `HardLine` after it, and stays after the same token it followed in the input. An own-line comment stays before the token it preceded, at that token's indent.
    *   A pseudo-comment `"#EC` stays on the line of the token it followed.
    *   An ABAP Doc block `"!` stays directly above the statement it preceded.
    *   A full-line `*` comment is printed in column 1, unchanged.
    *   Blank lines between statements are kept, capped at one. Blank lines inside a statement are removed.
    *   Literals and string templates are atomic. They are never split.
    *   **Unknown nodes:** keep their own line breaks and the relative indentation of their lines. The whole node is shifted to the current block indent, and runs of spaces between non-glued tokens on one line shrink to one space. Full-line `*` comments, the inside of block comments and multi-line literals are never shifted, and no shifted continuation line may land in column 1.
7.  **Alignment (D7).** `AlignGroup` is used in exactly three places:
    *   **A1:** parameter assignments in a broken parameter list of a method call (`EXPORTING`, `IMPORTING`, `CHANGING`, `RECEIVING`, `EXCEPTIONS`, and keyword-less lists `meth( a = 1 b = 2 )`). The `=` signs are aligned.
    *   **A2:** component assignments in a broken constructor expression (`VALUE`, `NEW`, `CORRESPONDING`, `CONV` with components). The `=` signs are aligned.
    *   **A3:** names in a broken `DATA:`, `TYPES:`, `CONSTANTS:` or `CLASS-DATA:` chain. The `TYPE`/`LIKE` keyword is aligned.

    A group covers contiguous rows only and ends at a blank line or a comment line. If aligning would push any row past the line width, or an anchor is glued to the token before it, the group falls back to single spaces. Adding a fourth case is a style change and follows the stability policy below.
8.  **Print.** The printer walks the IR and decides flat or broken for each `Group`. The fits check stops at the end of the group, at a forced break, or when the remaining width runs out, so run time is O(n × w) for line width w: linear in the output size for a fixed width.
    *   Width is counted in Unicode scalar values.
    *   ABAP's hard limit of 255 characters per line is enforced by the self-check (step 9), not by the printer.
9.  **Self-check.** The output is lexed again with the same lexer and compared with the input. Equal means:
    *   the same token kinds and texts, except case on tokens marked `keyword`;
    *   the same `glued` flags;
    *   the same comment texts, each attached to the same token and of the same kind;
    *   byte-equal verbatim regions;
    *   no ABAP line over 255 characters, unless that line was already over 255 in the input.

    **On a mismatch or a new line over 255 characters:**
    1.  The core finds the units that contain the mismatching tokens or the overlong lines, and asks the language crate to print them again as `Verbatim`. This is quarantine, reported as warning W001 (mismatch) or W002 (length).
    2.  It runs the self-check again. A unit that is still over 255 characters is printed `Opaque`, as in the input.
    3.  If the check still fails, the result is internal error X001 (exit code 70), no output is written, and the diagnostic includes enough context to reproduce the failure.
10. **Result.** `FormatResult { output: Option<String>, changed: bool, language, diagnostics: Vec<Diagnostic>, stats }`. `output` is `None` on any error.

### Verbatim regions (D8)

*   **AMDP:** the body between `METHOD m BY DATABASE PROCEDURE|FUNCTION … .` and `ENDMETHOD.`.
*   **Native SQL:** the text between `EXEC SQL` and `ENDEXEC`.
*   **Macros:** the body between `DEFINE name.` and `END-OF-DEFINITION.`.

The opening and closing statements are formatted normally. The body runs from directly after the opener's period to the closer keyword. It is printed `Opaque`: copied byte for byte and not re-indented, because SQLScript can contain multi-line string literals.

The AMDP body end is found by a small SQLScript-aware scanner that skips `"…"` identifiers, `'…'` strings, `--` and `/* */` comments, and stops at the `ENDMETHOD` keyword at statement start.

The region type is an interface (`VerbatimRegion { kind, range }`), so a SQLScript formatter can be added later without changing the ABAP parser.

### Supported inputs

| File pattern (abapGit) | Language | v1 status |
|---|---|---|
| `*.clas.abap`, `*.clas.locals_def.abap`, `*.clas.locals_imp.abap`, `*.clas.testclasses.abap`, `*.clas.macros.abap` | ABAP | Full grammar (M1) |
| `*.intf.abap` | ABAP | Full grammar (M1) |
| `*.prog.abap`, `*.fugr.*.abap`, `*.type.abap` | ABAP | Fallback until the M4 decision |
| `*.ddls.asddls`, `*.drty.acds` (CDS simple type) | CDS DDL | Full grammar (M2) |
| `*.ddlx.asddlxs` | CDS DDLX | Full grammar (M2) |
| `*.dcls.asdcls` | CDS DCL | Full grammar (M2) |
| `*.bdef.asbdef` | BDEF | Full grammar (M3) |
| `*.srvd.srvdsrv` | SRVD | Full grammar (M3) |
| `*.xml`, `*.ddls.baseinfo`, all others | none | Skipped silently in directory mode; error E011 if named explicitly |

*   **Namespaced objects:** abapGit encodes the `/` in object names as `#` (for example `#dmo#cl_flight.clas.abap`). The pattern match ignores the name part.
*   **ADT documents:** the extension passes the language ID (`abap` or `acds`) as a hint, and content detection decides within the CDS family.

### CLI contract

```text
origami [OPTIONS] [PATHS...]

  PATHS               files or directories; directories are walked, honoring .gitignore
                      and .origamiignore; with no PATHS and no --stdin, the current directory
  --stdin             read one source from stdin, write the result to stdout
  --stdin-filepath P  name used for language detection and config lookup; implies --stdin
  --language L        abap | ddls | ddlx | dcl | bdef | srvd | cds (CDS family, detected from content)
  --write             rewrite files in place (default for PATHS)
  --check             write nothing; exit 1 if any file would change
  --diff              with --check: print a unified diff of the changes to stdout
  --strict            exit 5 if any warning was emitted
  --preview           apply style rules that are still in preview
  --diagnostics F     human (default) | json | sarif; always written to stderr
  --max-diagnostics N cap the number of diagnostics per run (default 200)
  --config FILE       use only this config file, no discovery
  --no-config         ignore all config files
  --line-width N, --indent-width N, --keyword-case upper|lower
  -j N                worker threads for directory mode (default: CPU count)
  --version [--json] [--config FILE]
                      print version and style edition; with --config, also whether this
                      binary satisfies the file's required_version
```

Streams: stdout carries only formatted text (stdin mode) or the diff (`--diff`). Diagnostics in every format go to stderr.

Exit codes. In a multi-file run, every file is processed, and the highest code in this list wins: 70 > 4 > 3 > 2 > 5 > 1 > 0. Configuration and usage errors (E020, E021) abort the run before any file is processed.

| Code | Meaning |
|---|---|
| 0 | success; nothing to report |
| 1 | `--check`: at least one file would change |
| 2 | at least one file has a structural error and was left untouched |
| 3 | usage or configuration error |
| 4 | I/O error (unreadable file, failed write) |
| 5 | `--strict`: warnings were emitted. This includes W003, so `--strict` fails on every grammar gap until coverage is complete (decided by author: gaps must stay visible) |
| 70 | internal error (self-check failure after quarantine, or a caught panic) |

In stdin mode:

*   success means file status `formatted` or `unchanged`, even when the exit code is 5; the formatted text then goes to stdout;
*   with `--check`, stdout stays empty, and the exit code is 1 if the input would change;
*   on any error, stdout stays empty;
*   diagnostics always go to stderr.

Files are written atomically (temporary file, then rename), keep their permissions, and are not touched at all if their content did not change. Output does not depend on thread count or directory order.

JSON diagnostics, schema version 1, one object per run:

```json
{
  "schema": 1,
  "version": "1.0.0",
  "style_edition": "2026",
  "truncated": false,
  "files": [
    {
      "path": "src/zcl_foo.clas.abap",
      "language": "abap",
      "changed": true,
      "status": "formatted",
      "diagnostics": [
        {
          "code": "W003",
          "severity": "warning",
          "message": "Statement not recognized; formatted generically",
          "range": {
            "start": { "line": 42, "column": 5, "offset": 1290 },
            "end":   { "line": 44, "column": 18, "offset": 1377 }
          }
        }
      ]
    }
  ]
}
```

`line` is 1-based. `column` is 1-based and counted in UTF-16 code units, which is what VS Code uses. `offset` is the 0-based byte offset. `status` is one of `formatted`, `unchanged`, `error`, `skipped`. `truncated` is true when `--max-diagnostics` cut the list.

Diagnostic codes (complete for v1). In the Scope column, "file" means the code applies to one file, and "run" means it applies to the whole run.

| Code | Severity | Trigger | Exit | Scope | Run continues |
|---|---|---|---|---|---|
| E001 | error | unterminated literal, template or block comment | 2 | file | yes |
| E002 | error | unbalanced bracket | 2 | file | yes |
| E003 | error | ABAP statement without terminating period | 2 | file | yes |
| E004 | error | block-pair mismatch | 2 | file | yes |
| E005 | error | input is not valid UTF-8 | 2 | file | yes |
| E010 | error | language not detected | 2 | file | yes |
| E011 | error | file named explicitly but not a supported type | 3 | file | yes |
| E020 | error | invalid config file, invalid value, or `required_version` mismatch | 3 | run | no |
| E021 | error | invalid command-line usage | 3 | run | no |
| E030 | error | file cannot be read | 4 | file | yes |
| E031 | error | file cannot be written | 4 | file | yes |
| W001 | warning | unit quarantined by the self-check | 0 (5 with `--strict`) | file | yes |
| W002 | warning | unit left as written because of the 255-character limit | 0 / 5 | file | yes |
| W003 | warning | unknown statement formatted generically | 0 / 5 | file | yes |
| W004 | warning | nesting-depth limit reached | 0 / 5 | file | yes |
| W101 | warning | unknown config key | 0 / 5 | run | yes |
| X001 | internal | self-check failed after quarantine | 70 | file | yes |
| X002 | internal | panic caught | 70 | file | yes |

Codes are stable once released. `I`-codes are reserved for future notes that need no action.

### Configuration

*   **File:** `origami.toml`. The CLI looks for it starting in the file's directory and walking up. It stops at the first file found, or after checking the directory that contains `.git`. In stdin mode without `--stdin-filepath`, the search starts at the current working directory.
*   **Unknown keys:** produce warning W101, not an error.
*   **Ignored on purpose:** `.editorconfig` and VS Code's `tabSize`/`insertSpaces`, so that every tool produces the same output.

```toml
style_edition    = "2026"     # which version of the style rules to apply
required_version = ">=1.0, <2" # semver range; the CLI exits 3 if it does not match

line_width   = 120   # 40..=255
indent_width = 2     # 1..=8
preview      = false # apply style rules that are still in preview

[abap]
keyword_case = "upper"   # "upper" | "lower"
```

*   **Precedence:** command-line flags win over the config file, and the config file wins over the defaults.
*   **Line width:** values above 255 are rejected with E020 (exit code 3).

### Style specification and stability

*   **STYLE document:** `docs/STYLE.md` is the normative style. Each rule has:
    *   a stable ID, such as `ABAP-CALL-003` or `CDS-ANNO-001`;
    *   a statement of the rule;
    *   before/after examples.
*   **Rule tests:** every rule has at least one snapshot test whose name carries the rule ID.
*   **Writing rules:** a rule is written into STYLE.md, and approved by the maintainer, before its layout function is merged.
*   **Style editions:** each edition is named after its year, starting with `2026`.
    *   Within one edition, output changes only to fix bugs, and every such change is listed in the changelog with its rule ID.
    *   New rules first appear behind `--preview`.
    *   A new edition can change output freely. Repositories move to it by changing `style_edition`.
*   **Corpus diffs:** each release publishes the diff its output produces on the test corpus, compared to the previous release.

### VS Code extension (D6)

*   **Packaging:** one VSIX per platform (`darwin-x64`, `darwin-arm64`, `linux-x64`, `linux-arm64`, `win32-x64`, `win32-arm64`, `alpine-x64`). Each contains the matching `origami` binary, signed on macOS and Windows; Linux binaries ship with checksums only.
*   **Binary resolution,** in this order:
    1.  The setting `origami.path`. It is machine-scoped, so a workspace cannot set it.
    2.  For documents inside a folder with `origami.toml`, the first `origami` on `PATH` that reports `satisfies_required_version: true` for that file.
    3.  The bundled binary, checked the same way.

    The binary evaluates `required_version` itself, so the extension needs no TOML or semver code. If none satisfies it, the extension does not format and shows one status-bar warning.
*   **Untrusted workspaces:** in a workspace without VS Code Workspace Trust, steps 1 and 2 are skipped and only the bundled binary is used.
*   **Registration:** a `DocumentFormattingEditProvider` for:
    *   the language IDs `abap` and `acds` on any URI scheme, which covers ADT's `abap:`;
    *   the abapGit file patterns from the supported-inputs table on the `file` scheme.
*   **Per request:**
    1.  Spawn `origami --stdin --stdin-filepath <name> --diagnostics=json [--language <hint>]` with a 2-second timeout.
    2.  Write the document text to stdin.
    3.  On exit code 0 or 5, compute a minimal diff between old and new text and return it as `TextEdit`s. If `changed` is false, return no edits.
    4.  On any other exit code, return no edits.
*   **Language hint:** `abap` documents get `--language abap`, `acds` documents get `--language cds`.
*   **File name for `abap:` documents:** the extension builds the file name from the URI, based on the object name and type if spike S2 shows the URI carries them.
*   **Diagnostics:** shown in a `DiagnosticCollection` named `origami`, including warnings from successful runs, never as pop-up messages on save.
*   **Coexistence with ADT:** the README documents setting `editor.defaultFormatter` for `[abap]` and `[acds]`, because ADT registers its own formatter.

### Test strategy

*   **Snapshot tests:** one directory per STYLE rule, with input and expected output. Each run checks that the output matches, that formatting the expected output again yields the same text, and that the self-check passes.
*   **Corpus test:** `corpus/manifest.toml` lists repositories with a pinned commit SHA and an SPDX license. CI fetches them and checks for every file that:
    *   no structural error occurs;
    *   the self-check passes;
    *   formatting twice yields the same output.

    CI also reports the fallback rate per language. Candidate sources:
    *   abapGit;
    *   SAP-samples/abap-platform-refscen-flight (RAP: BDEF, SRVD, DCL, DDLX);
    *   SAP-samples/abap-cheat-sheets;
    *   open-abap;
    *   ajson;
    *   abap2xlsx.
*   **Independent oracle for ABAP:** CI runs the lexer of a pinned abaplint version over input and output, and compares the token sequences, with words compared case-insensitively. This catches errors that our own lexer would hide from our own self-check.
*   **Fault injection:** a test-only feature of the core forces a self-check mismatch or a panic, so quarantine (W001), X001 and X002 are tested.
*   **Fuzzing:** `cargo-fuzz` targets call `format` on mutated corpus files. Invariants: no X001 or X002, and formatted output formats to itself.
*   **SAP-system oracle (M3):** the corpus repositories are imported into an SAP system and activated, before and after formatting. Per object, the activation status and the set of message IDs must be equal; message positions are ignored.
*   **Benchmarks:** `criterion` for in-process time per language; `hyperfine` for CLI end-to-end time. A CI task compares both to G4 and fails if a target is missed by more than 20%.

### Milestones and exit criteria

| Milestone | Content | Exit criteria |
|---|---|---|
| **M0** | STYLE.md skeleton; invariants; CLI contract; config; both lexers; structural pass; identity printer (output = input); self-check; corpus pipeline; spikes S1–S3 | Identity run over the whole corpus: zero self-check failures; spike results written into this document |
| **M1** | ABAP parser and layout for classes and interfaces, statement kinds added in order of corpus frequency; alignment A1–A3; verbatim regions | Fallback rate below 5% of statements in the modern-ABAP part of the corpus; G1, G2, G4 met for ABAP |
| **M2** | CDS DDL, DDLX, DCL parsers and layout; shared annotation and expression grammar | Fallback rate below 5% of statements on corpus DDLS/DDLX/DCL; G1, G2 met |
| **M3** | BDEF and SRVD parsers and layout; SAP-system oracle | Activation results unchanged for all corpus BDEF/SRVD; fallback rate below 5% |
| **M4** | Decision on programs and function groups; VS Code extension; signed release pipeline; contracts frozen as v1.0 | CUJ 1–5 pass end to end |

## Alternatives Considered

Where the author chose between options during drafting without stating a reason, the reason given is the trade-off the author accepted when choosing. It is marked "trade-off accepted".

### Product scope

#### Alternative 1: Use ABAP Cleaner (layout-only profile) and abaplint for ABAP; build only a CDS-family formatter

*   **Pros**:
    *   Most ABAP layout work already exists and is maintained by SAP.
    *   The new code would cover only the real gap: BDEF, SRVD, DCL and DDLX.
*   **Cons**:
    *   ABAP Cleaner runs on the JVM, so every CLI, hook or agent call pays a cold start.
    *   Its rules mix layout with semantic rewrites, so a layout-only profile must be maintained by hand.
    *   Two tools give two styles and two configurations.
    *   ABAP Cleaner's VS Code extension does not register for ADT's `abap:` documents.
*   **Reason for Rejection**: Rejected by author: no single fixed style across ABAP and the CDS family, and the JVM cold start on every non-editor call.

#### Alternative 2: Evaluate existing tools for two weeks before writing code ("Stage 0")

*   **Pros**: Could show that most of the ABAP need is already met, before months of grammar work.
*   **Cons**: Delays the start; the author's reasons against Alternative 1 hold regardless of the outcome.
*   **Reason for Rejection**: Rejected by author for the same reasons as Alternative 1.
    *   Residual risk accepted: part of M1 may duplicate ABAP Cleaner's layout rules.
    *   Spikes S1–S3 in M0 still test the workflow assumptions.

#### Alternative 3: CDS family first, ABAP last

*   **Pros**:
    *   It closes the gap no other tool covers first.
    *   The CDS grammars are smaller, which makes them a cheaper proving ground for the pipeline.
*   **Cons**: ABAP is most of daily editing, so the main value arrives last.
*   **Reason for Rejection**: Rejected by author: ABAP is most of daily editing and should arrive in M1.

#### Alternative 4: Semantic cleanup rules (as in ABAP Cleaner)

*   **Pros**: More value per run, for example modern syntax and removed obsolete statements.
*   **Cons**:
    *   Each rule can change behavior, so each needs proof of equivalence.
    *   It needs type information that an offline tool does not have.
    *   The self-check could no longer compare token streams.
*   **Reason for Rejection**: Rejected by author: the tool formats only. A semantics-free guarantee is the product.

#### Alternative 5: Output compatible with the Eclipse pretty printer or ABAP Cleaner

*   **Pros**: Mixed teams would not flip-flop the layout when someone presses Shift+F1 in Eclipse.
*   **Cons**:
    *   The style would be bound to tools the project does not control.
    *   The pretty printer does not break long lines, so full reflow would be impossible.
*   **Reason for Rejection**: Rejected by author: the team adopts one fixed style. The CI `--check` gate is the single source of truth.

#### Alternative 6: Format abapGit XML metadata

*   **Pros**: Every file in the repository would be covered.
*   **Cons**: abapGit re-serializes these files in its own fixed format on every export, so any formatting is undone and shows up as diff noise.
*   **Reason for Rejection**: Rejected by author: it produces churn and has no benefit.

### Error and case handling

#### Alternative 7: Reject valid code that the grammar does not know

*   **Pros**: A simpler model, because every unrecognized statement is an error.
*   **Cons**: After each SAP syntax release, valid code fails pre-commit hooks until the grammar catches up. Teams then switch the hook off.
*   **Reason for Rejection**: Rejected by author: replaced by the generic fallback with warning W003.

#### Alternative 8: Partially format files with structural errors

*   **Pros**: Format-on-save keeps working while code is half-written.
*   **Cons**: In ABAP, one missing period moves the boundaries of every following statement, so "partial" formatting can corrupt code far from the error.
*   **Reason for Rejection**: Rejected by author: files with structural errors are left untouched.

#### Alternative 9: Upper-case keywords in CDS, DCL, DDLX, BDEF and SRVD as well

*   **Pros**: One rule for all six languages.
*   **Cons**: Every SAP template, documentation example and RAP code base uses lower case in these languages.
*   **Reason for Rejection**: Rejected by author: upper case applies to ABAP only.

#### Alternative 10: Re-case identifiers

*   **Pros**: More uniform appearance.
*   **Cons**: In CDS, alias case becomes the OData property name, so re-casing changes a public API. In ABAP, a word list cannot tell keywords from identifiers, because ABAP has no reserved words.
*   **Reason for Rejection**: Rejected by author: identifiers are never re-cased.

### Layout

#### Alternative 11: Keep the author's line breaks (gofmt/Black style), or split only overlong lines

*   **Pros**:
    *   Smaller first diff on existing code.
    *   Idempotency is easier to guarantee.
    *   The tool depends less on the grammar.
*   **Cons**: The output is not canonical. Two authors who break the same statement differently keep different layouts.
*   **Reason for Rejection**: Rejected by author: output must be canonical regardless of the author's line breaks.
    *   Cost accepted: a large first diff, which the one-time mass format and `.git-blame-ignore-revs` absorb.
    *   Cost accepted: harder idempotency, which the corpus double-format test checks.

#### Alternative 12: No cross-line alignment (L1)

*   **Pros**:
    *   The simplest printer.
    *   Renaming one parameter changes one line, which gives the smallest diffs for review and for agents.
*   **Cons**: It looks unfamiliar to ABAP developers used to ABAP Cleaner's alignment.
*   **Reason for Rejection**: Rejected by author (trade-off accepted: familiar layout in three cases over minimal diffs).

#### Alternative 13: Align everywhere a "key = value" shape appears (L3)

*   **Pros**: The most uniform look across all six languages.
*   **Cons**:
    *   The largest diffs and the most special cases.
    *   This much alignment is unusual in CDS code.
*   **Reason for Rejection**: Rejected by author (trade-off accepted: limited diff noise over full uniformity).

### Architecture

#### Alternative 14: Layered structure with layout rules as data tables (parser option B)

*   **Pros**:
    *   Fallback is the normal path.
    *   New SAP syntax costs a table row, not parser code.
*   **Cons**:
    *   The table language tends to grow into an informal parser.
    *   Layout quality for nested expressions (`VALUE`, `COND`, `REDUCE`) is limited.
*   **Reason for Rejection**: Rejected by author (trade-off accepted: layout quality for nested expressions over lower grammar upkeep).

#### Alternative 15: Token stream with penalty-based line breaking (clang-format style, parser option C)

*   **Pros**: Needs the least grammar, and even unknown syntax gets line breaks.
*   **Cons**:
    *   Penalty tuning is hard, and changes have non-local effects.
    *   Idempotency is harder.
    *   Results are hard to predict, which conflicts with a written STYLE specification.
*   **Reason for Rejection**: Rejected by author (trade-off accepted: predictable, specifiable output over minimal grammar).

#### Alternative 16: Generate the ABAP grammar from abaplint's combinator definitions (A2)

*   **Pros**:
    *   Full statement coverage early.
    *   Updates arrive with each abaplint release.
*   **Cons**:
    *   The generator is a project of its own.
    *   The tree shape follows abaplint's recognition needs, not layout needs.
    *   It is tied to abaplint internals that can change without notice.
*   **Reason for Rejection**: Rejected by author (trade-off accepted: independence and a layout-shaped tree over early coverage). The months of hand-written grammar are mitigated by the fallback and by adding statement kinds in order of corpus frequency.

#### Alternative 17: Hybrid grammar — about 40 hand-written statement kinds plus a keyword table from abaplint (A3)

*   **Pros**: A fast start, with hand work spent only where layout matters.
*   **Cons**: Two mechanisms, whose boundary must be maintained. Table-driven statements get only generic layout.
*   **Reason for Rejection**: Rejected by author (trade-off accepted: one uniform mechanism over a faster start).

#### Alternative 18: TypeScript, reusing abaplint's parser

*   **Pros**: Years of grammar work available immediately.
*   **Cons**:
    *   It needs a Node.js runtime in CI and agent sandboxes.
    *   Node start-up costs tens of milliseconds per CLI call.
    *   Formatting would depend on abaplint's tree shape.
*   **Reason for Rejection**: Rejected by author: Rust gives one static binary with no runtime and the lowest per-call start-up cost (D1).

### Editor integration

#### Alternative 19: Core compiled to WASM inside the extension (V2)

*   **Pros**:
    *   One package for all platforms.
    *   No code signing and no process start.
    *   Works in vscode.dev and SAP Business Application Studio.
*   **Cons**:
    *   A second build target that must produce identical output.
    *   The extension's core version can drift from the CLI version pinned by the repository.
*   **Reason for Rejection**: Rejected by author (trade-off accepted: one identical binary for CLI and editor, and simple version pinning, over cross-platform packaging and browser support).

#### Alternative 20: Long-lived native daemon (V3)

*   **Pros**: No process start per format request.
*   **Cons**:
    *   The same packaging and signing work as V1.
    *   Process lifecycle management on top: crashes, restarts and version changes.
    *   The gain is small, because a Rust process starts in a few milliseconds.
*   **Reason for Rejection**: Rejected by author (trade-off accepted: no lifecycle management over saving a few milliseconds per format).

#### Alternative 21: Language server (LSP)

*   **Pros**: Editor-agnostic, and range formatting would come for free later.
*   **Cons**: A protocol and state that v1 does not need. Formatting is a single request/response.
*   **Reason for Rejection**: Rejected by author: out of scope for v1 (Non-Goal 8).

### Not decided, does not block v1

*   **Contributing upstream instead of, or as well as, building:** adding the `abap:` scheme to ABAP Cleaner's VS Code extension, or BDEF/SRVD grammar to abaplint. The author has not decided. Neither changes the v1 design.
*   **tree-sitter-abap or Topiary as the parser or formatter base.** Their error recovery conflicts with "untouched on error", and their grammar quality is unknown. This conflicts with D4 and D5, so it would only be reconsidered if hand-written grammar progress in M1 stalls.

--------------------------------------------------------------------------------

## Non-Functional Requirements

*   **Security & Privacy**:
    *   The core and the CLI make no network calls and send no telemetry. Source code never leaves the machine.
    *   The VS Code setting `origami.path` is machine-scoped, so a cloned repository cannot point the extension at a binary of its choice. In workspaces without Workspace Trust, only the bundled binary runs.
    *   Release binaries are signed (Apple Developer ID with notarization; Windows Authenticode) and published with SHA-256 checksums.
    *   Dependencies are checked by `cargo deny` against a license allowlist and the RustSec advisory database on every CI run.
*   **Performance & Latency**:
    *   The G4 targets apply, measured on the two reference machines (an Apple Silicon laptop and a GitHub-hosted `ubuntu-latest` runner).
    *   CI fails if a benchmark misses its target by more than 20%.
    *   The printer runs in O(n × w) for line width w, which is linear in output size for a fixed width, and parser depth is limited to 200, so no input can cause quadratic time or a stack overflow.
*   **Rollout & Rollback**:
    *   **Style editions and version pinning:** `required_version` and `style_edition` in `origami.toml` pin the output per repository. Rolling back means pinning the previous version, after which output returns to the previous edition's.
    *   **First adoption:** `docs/adoption.md` describes a one-time mass format as a separate commit, listed in `.git-blame-ignore-revs`. It also recommends `editor.formatOnSaveMode: "modifications"` for code that lives in an SAP system, so that saving one method does not reformat a whole class that would then need a transport.
*   **Reliability**:
    *   Panics in the core are caught at the `format` boundary and reported as X002. No file is written on any error.
    *   Writes are atomic.

--------------------------------------------------------------------------------

### Additional Resources

*   `concepts_brief.md`: brainstorm brief with the review findings of seven lenses (2026-11-21).
*   SAP ABAP Keyword Documentation (latest ABAP Cloud and S/4HANA 2023 editions): the grammar source for D5.
*   abaplint (MIT), `@abaplint/core`: reference for statement coverage and test cases, and the pinned lexer used as an independent oracle.
*   ABAP Cleaner (Apache-2.0): reference for layout conventions ABAP developers are used to (alignment cases A1–A3).
*   Philip Wadler, "A prettier printer" (2003), and the formatter crates of Biome and Ruff (MIT): references for the Doc IR and printer.
*   `rowan` crate (MIT/Apache-2.0): lossless syntax tree library used by rust-analyzer.
*   SAP-samples/abap-platform-refscen-flight and SAP-samples/abap-cheat-sheets: modern ABAP and RAP corpus sources.

### Future Improvements

*   **SQLScript formatter** plugged into the `VerbatimRegion` interface for AMDP bodies.
*   **WASM build** of the core for vscode.dev and SAP Business Application Studio.
*   **Range formatting and a language server**, once v1 contracts are frozen.
*   **More distribution channels:** Homebrew, an npm platform package (`abap-origami`), a PyPI wheel via maturin for pre-commit users without Rust.
*   **`origami check --validate`**, which delegates validity to abaplint or to an SAP system through ADT, for agent loops that want one call.
*   **Upstream contributions** (ABAP Cleaner's VS Code selector for `abap:`, BDEF/SRVD grammar for abaplint).
*   **Classic ABAP grammar** (reports, selection screens, FORM routines), depending on the M4 decision.

--------------------------------------------------------------------------------

## Implementation notes (M1, 2026-11-21)

Deviations from the plan above, recorded during implementation:

* **ABAP parsing and layout are combined.** Each statement parser returns its
  Doc directly (`crates/origami_abap/src/stmt.rs`, `statements.rs`) instead of
  building a `rowan` tree first. Trivia and comments live in the token stream,
  so a separate tree was not needed. `rowan` is not used.
* **Keyword statements are table-driven.** About 50 statements are described
  by a `Spec` (lead keywords, clause keywords, other keywords, operand mode).
  Words in keyword positions are re-cased; everything else is parsed as an
  operand or logical expression. A statement a parser does not fully
  understand falls back to the generic layout (W003).
* **Raw sub-spans.** Inside a recognized statement, a bracketed part the parser
  does not understand (for example `VALUE #( FOR … )`) keeps its own line
  breaks and moves with the statement.
* **Fill clauses.** A broken keyword statement breaks only before clauses that
  do not fit (`APPEND VALUE #( … ) TO itab` keeps `TO itab` on the last line);
  ABAP SQL breaks before every clause (ABAP-SQL-001).
* **CDS family: structural layout instead of grammar (M2/M3 phase 1).** Units
  (annotations, elements, `;` statements, braces), block indentation and `key`
  alignment; inside a unit the author's line breaks are kept. The fallback
  rate reported for the CDS family therefore measures nothing yet; grammar-
  based layout of headers and expressions is future work.
* **Comments:** trailing blanks after a line comment are whitespace, not
  comment text.

## Detailed Implementation Plan

All paths are relative to the repository root. The repository is a Cargo workspace plus three Node packages: the VS Code extension and two CI tools (`tools/abaplint-oracle`, `tools/sap-activation-check`).

Fixed technical choices:

*   **Toolchain:** Rust edition 2024. The toolchain is pinned in `rust-toolchain.toml` to the stable release current at the start of M0. MSRV = that version. Fuzzing uses nightly (item 79).
*   **Node:** Node 22 for every Node package.
*   **Extension tooling:** TypeScript 5, bundled with `esbuild`, tested with `@vscode/test-electron`. Minimal edits are computed with `diff` (jsdiff).
*   **Release profile:** `lto = "fat"`, `codegen-units = 1`, `panic = "unwind"` (required so the core can catch panics).
*   **Dependency versions:** each dependency is taken at its latest release on the M0 start date and recorded in `Cargo.lock` / `package-lock.json`. `[workspace.dependencies]` states `major.minor`.

Dependency matrix:

| Manifest | Runtime dependencies | Dev dependencies |
|---|---|---|
| `origami_syntax` | `rowan`, `serde`, `thiserror` | — |
| `origami_layout` | `origami_syntax` | — |
| `origami_abap`, `origami_cds` | `origami_syntax`, `origami_layout`, `rowan` | — |
| `origami_core` | the four crates above, `serde` | `criterion` |
| `abap-origami` (CLI) | `origami_core`, `clap`, `serde`, `serde_json`, `toml`, `semver`, `ignore`, `rayon`, `tempfile`, `similar` | `assert_cmd`, `jsonschema` |
| `xtask` | `origami_core`, `serde`, `serde_json`, `toml` | — |
| `fuzz` (own workspace) | `libfuzzer-sys`, `origami_core` (path) | — |
| `editors/vscode` | `diff` | `typescript`, `esbuild`, `@types/vscode`, `@types/node`, `@types/mocha`, `mocha`, `@vscode/test-electron`, `eslint`, `typescript-eslint`, `@vscode/vsce`, `ovsx` |
| `tools/abaplint-oracle` | `@abaplint/core` (exact version pin) | — |
| `tools/sap-activation-check` | none (Node 22 built-in `fetch`) | — |

Each milestone is one or more pull requests. Each file's milestone is given in brackets.

### Target Files to Create/Modify

#### Workspace root

##### 1. `Cargo.toml` (Create) [M0]

*   **Type / Scope**: Cargo workspace manifest.
*   **Proposed Changes**:
    *   **Members:** `crates/origami_syntax`, `crates/origami_layout`, `crates/origami_abap`, `crates/origami_cds`, `crates/origami_core`, `crates/origami_cli`, `xtask`. The `fuzz/` directory is excluded.
    *   **`[workspace.dependencies]`:** pins every crate in the dependency matrix.
    *   **Lints:** `[workspace.lints.clippy] all = "deny"` and `[workspace.lints.rust] unsafe_code = "forbid"`. Every member declares `[lints] workspace = true`.
    *   **`[profile.release]`:** as above.
*   **Why**: one place for versions and lints. origami's own code needs no `unsafe`, and `forbid` keeps it that way.

##### 2. `rust-toolchain.toml` (Create) [M0]

*   **Type / Scope**: Toolchain pin.
*   **Proposed Changes**: `channel = "<stable version at M0 start>"`, components `rustfmt`, `clippy`, `llvm-tools-preview`.
*   **Why**: the same compiler and lint results on every machine.

##### 3. `.cargo/config.toml` (Create) [M0]

*   **Type / Scope**: Cargo alias.
*   **Proposed Changes**: `[alias] xtask = "run --package xtask --"`.
*   **Why**: makes `cargo xtask …` work (items 76, 81, 83).

##### 4. `Cargo.lock` (Create, generated, committed) [M0]

*   **Type / Scope**: Lockfile.
*   **Why**: reproducible builds; records the versions chosen under the version rule.

##### 5. `deny.toml` (Create) [M0]

*   **Type / Scope**: `cargo-deny` configuration.
*   **Proposed Changes**:
    *   License allowlist: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Zlib.
    *   Deny RustSec advisories.
    *   Deny duplicate major versions of `rowan` and `serde`.
    *   `fuzz/` is a separate workspace and outside `cargo deny` scope, because `libfuzzer-sys` includes the NCSA license.
*   **Why**: Security NFR and the license allowlist.

##### 6. `about.toml`, `about.hbs` (Create) [M0]

*   **Type / Scope**: `cargo about` configuration and template.
*   **Proposed Changes**: `about.toml` accepts the same licenses as `deny.toml`. `about.hbs` renders one section per license, with the crates using it.
*   **Why**: generates `THIRD_PARTY_NOTICES.md` (item 8).

##### 7. `LICENSE-MIT`, `LICENSE-APACHE` (Create) [M0]

*   **Type / Scope**: Project license.
*   **Proposed Changes**: The dual MIT/Apache-2.0 texts. Every `Cargo.toml` and `package.json` declares `MIT OR Apache-2.0`.
*   **Why**: compatible with abaplint (MIT) and ABAP Cleaner (Apache-2.0). This is subject to Open Question 6: if the author picks another license, only these files and the license fields change.

##### 8. `THIRD_PARTY_NOTICES.md` (Create) [M0, regenerated per release]

*   **Type / Scope**: Attribution file.
*   **Proposed Changes**:
    *   A generated part, produced by `cargo about generate about.hbs`.
    *   A hand-written part that lists the reference versions of abaplint and ABAP Cleaner consulted. It states: "No source code was copied. Test inputs taken from abaplint are listed below with their license."
*   **Why**: license compliance for shipped binaries.

##### 9. `README.md` (Create) [M0, extended each milestone]

*   **Type / Scope**: User documentation.
*   **Proposed Changes**:
    *   What origami does and does not do.
    *   Install: GitHub Releases, `cargo install abap-origami`, VS Code Marketplace / Open VSX.
    *   CLI quick start; pre-commit setup; CI setup.
    *   The supported-inputs table and links to `docs/`.
*   **Why**: first entry point for users and agents.

##### 10. `CHANGELOG.md` (Create) [M0]

*   **Type / Scope**: Release notes.
*   **Proposed Changes**: Keep-a-Changelog format. Every output change within a style edition is listed with its rule ID.
*   **Why**: required by the stability policy.

##### 11. `.gitignore` (Create) [M0]

*   **Type / Scope**: VCS hygiene.
*   **Proposed Changes**: `target/`, `corpus/repos/`, `editors/vscode/node_modules/`, `editors/vscode/out/`, `editors/vscode/bin/`, `tools/*/node_modules/`, `fuzz/artifacts/`, `fuzz/corpus/`, `fuzz/target/`.
*   **Why**: keep the fetched corpus, build output and fuzz output out of git.

##### 12. `.pre-commit-hooks.yaml` (Create) [M0]

*   **Type / Scope**: pre-commit framework hook definition.
*   **Proposed Changes**:
    *   Hook id `origami`, with `entry: origami --check`, `language: system`, `files: \.(abap|asddls|asddlxs|asdcls|asbdef|srvdsrv)$` and `pass_filenames: true`.
    *   Hook id `origami-fix`, the same except `entry: origami --write`.
*   **Why**: CUJ 2 without hand-written hook scripts.

##### 13. `.git-blame-ignore-revs` (Create) [M0]

*   **Type / Scope**: Template for adopters.
*   **Proposed Changes**: An empty file with a comment that explains how to list the commit of the one-time mass format. `docs/adoption.md` tells adopters to copy it.
*   **Why**: supports the first-adoption step of the Rollout NFR.

#### Documentation

##### 14. `docs/STYLE.md` (Create) [M0 skeleton, filled per milestone]

*   **Type / Scope**: Normative style specification.
*   **Proposed Changes**:
    *   **Sections:** General (`GEN-…`: width, indent, blank lines, line endings, comments, glued tokens), ABAP, CDS shared (annotations, expressions), DDLS, DDLX, DCL, BDEF, SRVD.
    *   **Rule format:** each rule has the heading `### <ID>: <title>`. The ID matches `^(GEN|ABAP|CDS|DDLS|DDLX|DCL|BDEF|SRVD)-[A-Z]+-\d{3}$`. Each rule then has:
        *   a `Milestone: M<n>` line;
        *   a one-paragraph statement of the rule;
        *   a before/after example.
    *   **Content at M0:** the general rules only.
    *   **Approval gate:**
        *   Each STYLE rule enters this file through its own pull request.
        *   The maintainer, Nils Lutz, approves that pull request before any layout pull request that cites the rule ID is merged.
        *   A layout pull request that cites a rule ID not yet on `main` fails review.
    *   **Rule batches:**
        *   The first pull request of M1 adds the rule set for the statement kinds that `cargo xtask freq` ranks highest, including the alignment rules `ABAP-ALIGN-001..003` (A1–A3).
        *   M2 and M3 start the same way for their languages.
*   **Why**: G7, and an owner for every output decision.

##### 15. `docs/INVARIANTS.md` (Create) [M0]

*   **Type / Scope**: Normative safety specification.
*   **Proposed Changes**: Each of the following comes with a reference to its test:
    *   the self-check equality definition;
    *   the `glued` rule;
    *   the comment placement and anchor rules (item 25);
    *   the re-indent rules for `Unknown`/`Verbatim`/`Opaque` (item 32);
    *   the 255-character rule;
    *   the structural error list (E001–E005).
*   **Why**: G1. Reviewers and contributors check changes against one document.

##### 16. `docs/CLI.md` (Create) [M0]

*   **Type / Scope**: CLI contract.
*   **Proposed Changes**:
    *   The full flag reference, including `--preview` and `--language cds`.
    *   The exit-code table and its aggregation order.
    *   The stream routing rules.
    *   stdin behavior.
    *   The config file format, discovery and precedence.
    *   The complete diagnostic code table (Technical Plan).
    *   The `--version --json` schema (item 67).
*   **Why**: G6. Agents and CI scripts are written against this file. It is frozen at v1.0.

##### 17. `schemas/diagnostics.v1.json` (Create) [M0]

*   **Type / Scope**: JSON Schema (draft 2020-12) for `--diagnostics=json` output.
*   **Proposed Changes**:
    *   Defines the object shown in the Technical Plan, including the top-level boolean `truncated`.
    *   `schema` is the constant `1`. `status` and `severity` are enums.
    *   `column` is documented as UTF-16 code units.
*   **Why**: lets agents and the extension validate output, and lets CLI tests assert conformance.

##### 18. `docs/adoption.md` (Create) [M4]

*   **Type / Scope**: Team adoption guide.
*   **Proposed Changes**:
    *   One-time mass format, with `.git-blame-ignore-revs` (copy item 13).
    *   Pre-commit and CI setup with `required_version`.
    *   VS Code `editor.defaultFormatter` for `[abap]` and `[acds]`.
    *   `editor.formatOnSaveMode: "modifications"` for system-first projects.
    *   Switching off other formatters (Eclipse pretty printer on save, ABAP Cleaner on save).
    *   Agent usage through stdin together with the ADT MCP server.
*   **Why**: the Rollout NFR, and the risk that teammates using other formatters undo the style.

##### 19. `docs/spikes/S1-roundtrip.md`, `docs/spikes/S2-adt-vscode.md`, `docs/spikes/S3-pilot.md` (Create) [M0]

*   **Type / Scope**: Spike reports.
*   **Proposed Changes**: Each report records the procedure, the systems and versions used, the observed results and the decision taken.
    *   S1 decides whether origami is positioned as a CI gate or as an authoring tool.
    *   S2 decides URI-based language hints and how `abap:` edits are handled.
    *   S3 names the pilot team and its workflow.
*   **Why**: Open Questions 1–3 are experiments.

##### 20. `docs/designs/origami.md` (Modify) [M0]

*   **Type / Scope**: This design.
*   **Proposed Changes**: The spike results are written into Open Questions 1–3, and the items that depend on them (90) are amended.
*   **Why**: keeps the design the single record of decisions.

#### Crate `origami_syntax` (shared primitives and types)

##### 21. `crates/origami_syntax/Cargo.toml` (Create) [M0]

*   **Type / Scope**: Library crate manifest (see the dependency matrix).

##### 22. `crates/origami_syntax/src/lib.rs` (Create) [M0]

*   **Type / Scope**: Public API.
*   **Proposed Changes**: Declares and re-exports `source`, `token`, `trivia`, `diagnostic`, `lang`, `compare`. Contains no language knowledge.

##### 23. `crates/origami_syntax/src/source.rs` (Create) [M0]

*   **Type / Scope**: Input decoding and position mapping.
*   **Proposed Changes**:
    *   `SourceText::decode(&[u8]) -> Result<SourceText, Diagnostic>`:
        *   Validates UTF-8. Invalid input gives E005.
        *   Strips and records the BOM.
        *   Converts every CRLF to LF. A lone CR is kept as an ordinary character.
        *   Records the line-break style of the first line break, and whether a final newline is present.
    *   Spans refer to the normalized text.
    *   `fn encode(&self, out: &str) -> Vec<u8>` writes every LF in the recorded style, restores the BOM and the final-newline state. Mixed line endings therefore become uniform, which is a whitespace-only change.
    *   `LineIndex` maps a byte offset to `(line, utf16_column)`.
*   **Why**: Pipeline step 1 and diagnostic positions.

##### 24. `crates/origami_syntax/src/token.rs` (Create) [M0]

*   **Type / Scope**: Token model.
*   **Proposed Changes**:
    *   `struct TokenIndex(u32)`.
    *   `struct Token { kind: u16, span: Range<u32>, glued: bool, keyword: bool }`.
    *   `struct TokenStream { tokens: Vec<Token>, trivia: Vec<Trivia> }`.
    *   `kind` values are defined per language.
    *   `keyword` starts false and is set only by a parser.
    *   The virtual end-of-file anchor is `TokenIndex(tokens.len())`.
*   **Why**: the `glued` and `keyword` flags are the basis of G1 and of keyword casing.

##### 25. `crates/origami_syntax/src/trivia.rs` (Create) [M0]

*   **Type / Scope**: Whitespace and comment model.
*   **Proposed Changes**:
    *   **Types:**
        *   `struct TriviaIndex(u32)`.
        *   `enum TriviaKind { Whitespace, Newline, Comment { syntax: CommentSyntax, placement: Placement } }`.
        *   `enum CommentSyntax { AbapStar, AbapQuote, AbapDoc, AbapPseudo, CdsLine, CdsBlock }`. `CdsLine` covers `//` and `--`.
        *   `enum Placement { OwnLine, EndOfLine }`. Placement is EndOfLine when a token precedes the comment on the same line, OwnLine otherwise.
        *   `struct Trivia { kind, span, anchor: TokenIndex, position: Leading | Trailing }`.
    *   **`attach(tokens, trivia)`:**
        *   An EndOfLine comment is Trailing on the preceding token.
        *   An OwnLine comment is Leading on the next token. When there is no next token, it is Leading on the end-of-file anchor.
*   **Why**: the comment rules and the self-check comparison.

##### 26. `crates/origami_syntax/src/diagnostic.rs` (Create) [M0]

*   **Type / Scope**: Diagnostics model.
*   **Proposed Changes**:
    *   `struct Diagnostic { code: Code, severity: Severity, message: String, range: Range<u32> }`.
    *   `enum Severity { Info, Warning, Error, Internal }`.
    *   `enum Code`: one variant per code in the Technical Plan code table, each with its fixed string (`"E001"`), its severity and its exit code.
*   **Why**: one stable code list shared by all crates.

##### 27. `crates/origami_syntax/src/lang.rs` (Create) [M0]

*   **Type / Scope**: Types shared between the core and the language crates.
*   **Proposed Changes**:

    | Type | Definition |
    |---|---|
    | `Language` | `{ Abap, Ddls, Ddlx, Dcl, Bdef, Srvd }` |
    | `Case` | `{ Upper, Lower }` |
    | `Edition` | `{ E2026 }` |
    | `LangOptions` | `{ line_width: u16, indent_width: u8, abap_keyword_case: Case, preview: bool }` |
    | `UnitId` | `u32` |
    | `Unit` | `{ id: UnitId, tokens: Range<TokenIndex> }` |
    | `Stats` | `{ statements: u32, unknown: u32, depth_limited: u32, quarantined: u32 }` |
    | `LangResult` | `{ output: String, input_tokens: TokenStream, units: Vec<Unit>, diagnostics: Vec<Diagnostic>, stats: Stats }` |

    *   **Units:**
        *   An ABAP unit is one statement from its first token to its terminating `.`. A whole chain (`DATA: a, b.`) is one unit.
        *   A CDS unit is the smallest of: one annotation with its value, one select-list or field-list element, one `;`-terminated statement, or a header (the tokens before the first `{` of a definition).
*   **Why**: puts these types where both the core and the language crates can use them, without a dependency cycle.

##### 28. `crates/origami_syntax/src/compare.rs` (Create) [M0]

*   **Type / Scope**: Token-stream equality for the self-check.
*   **Proposed Changes**:
    *   `fn equivalent(input: &TokenStream, input_src: &str, output: &TokenStream, output_src: &str) -> Result<(), Vec<Mismatch>>`.
    *   It implements the equality rules in `docs/INVARIANTS.md`.
    *   Keyword flags come from `input`, because output tokens are always re-lexed with `keyword = false`.
    *   `struct Mismatch { input_index: TokenIndex, kind: MismatchKind }`, where `MismatchKind` is one of `{ Text, Glued, CommentText, CommentAnchor, CommentKind, Opaque, Count }`. For `Count`, `input_index` is the first position where the two streams differ.
*   **Why**: Pipeline step 9, independent of any language crate.

##### 29. `crates/origami_syntax/tests/source.rs`, `tests/trivia.rs`, `tests/compare.rs` (Create) [M0]

*   **Type / Scope**: Unit tests.
*   **Proposed Changes**:
    *   `source.rs`: BOM present or absent, CRLF, LF, mixed endings, lone CR, final newline present or absent, invalid UTF-8 (E005), UTF-16 column mapping with umlauts and emoji.
    *   `trivia.rs`: every placement and anchor rule, including the end-of-file anchor.
    *   `compare.rs`: one case per `MismatchKind`, and the keyword-case exemption.

#### Crate `origami_layout` (Doc IR and printer)

##### 30. `crates/origami_layout/Cargo.toml` (Create) [M0]

*   **Type / Scope**: Library crate manifest. Depends only on `origami_syntax`.

##### 31. `crates/origami_layout/src/lib.rs` (Create) [M0]

*   **Type / Scope**: Public API.
*   **Proposed Changes**: Re-exports `Doc`, `builder`, `print` and `PrintOptions`. The IR variants stay private.

##### 32. `crates/origami_layout/src/ir.rs` (Create) [M0]

*   **Type / Scope**: Doc IR.
*   **Proposed Changes**: `pub struct Doc(Inner)`, where `Inner` is a private enum:

    | Variant | Meaning |
    |---|---|
    | `Token(TokenIndex)` | the original token |
    | `Space` | one space |
    | `SoftLine` | a space if the group fits, a line break otherwise |
    | `HardLine` | always a line break |
    | `BlankLine` | an empty line; consecutive ones collapse to one |
    | `Group(Vec<Doc>)` | flat if it fits, broken otherwise |
    | `Indent(Vec<Doc>)` | adds one indent level to breaks inside it |
    | `AlignGroup(Vec<AlignItem>)` | aligned rows (D7) |
    | `Comment(TriviaIndex)` | a comment, emitted only by `token(i)` |
    | `Unknown(Range<u32>)` | re-indent, and shrink runs of spaces between non-glued tokens on one line to one space; used for `UNKNOWN_STATEMENT` |
    | `Verbatim(Range<u32>)` | re-indent only; used for quarantined units |
    | `Opaque(Range<u32>)` | byte-exact, never re-indented; used for D8 bodies and the second-level W002 fallback |
    | `Concat(Vec<Doc>)` | a sequence |

    *   `AlignItem` is `Row(AlignRow) | Comment(TriviaIndex) | Blank`, where `AlignRow { head: Vec<Doc>, anchor: TokenIndex, tail: Vec<Doc> }`.
    *   **Range boundaries for `Unknown`/`Verbatim`/`Opaque`:**
        *   A range runs from the first byte of the unit's first token to the last byte of its last token.
        *   It includes trailing EndOfLine comments on the last line.
        *   It excludes leading indentation.
*   **Why**: the layout vocabulary, with no way to build a gap that bypasses the builder.

##### 33. `crates/origami_layout/src/builder.rs` (Create) [M0]

*   **Type / Scope**: The only way to build a `Doc`.
*   **Proposed Changes**:
    *   Helpers: `token(i)`, `sep(i)`, `soft(i)`, `hard(i)`, `blank_line()`, `group(..)`, `indent(..)`, `align(..)`, `unknown(range)`, `verbatim(range)`, `opaque(range)`.
    *   `sep(i)`, `soft(i)` and `hard(i)` take the index of the token after the gap. They emit nothing when that token is glued, and `debug_assert!` fails in tests.
    *   `token(i)` emits, in order:
        1.  the leading trivia attached to token *i*, each comment followed by `HardLine`;
        2.  the token;
        3.  its trailing comments. An EndOfLine comment forces a `HardLine` after it.
    *   Layout functions never emit comments themselves.
*   **Why**: enforces the `glued` rule and comment emission by construction. Layout code cannot add a gap or drop a comment by mistake.

##### 34. `crates/origami_layout/src/printer.rs` (Create) [M0 identity mode, M1 full]

*   **Type / Scope**: Printer.
*   **Proposed Changes**:
    *   **Signature:** `pub struct PrintOptions { width: u16, indent: u8, keyword_case: Option<Case>, identity: bool }` and `pub fn print(doc: &Doc, src: &SourceText, tokens: &TokenStream, opts: &PrintOptions) -> String`.
    *   **Identity mode:** when `identity` is true, the printer returns the normalized source text unchanged.
    *   **Break propagation:** a pre-pass marks every `Group` that contains a `HardLine`, a `BlankLine` or a comment at any depth. Marked groups are always broken.
    *   **Fits check:** scans the group's flat content until the group ends, the remaining width becomes negative, or a forced break is reached. Run time is O(n × w) for line width w, which is linear in n for a fixed width. Width is counted in Unicode scalar values.
    *   **Keyword case:** tokens with `keyword = true` are re-cased per `keyword_case`. `None` leaves every token as written; the CDS crate uses `None`.
    *   **Comment rules:**
        *   `AbapStar` comments are printed in column 1, unchanged.
        *   Other OwnLine comments are printed at the indent of the token they lead.
        *   For a multi-line `CdsBlock`, only the first line is re-indented. Interior lines stay byte for byte.
    *   **Continuation indent:** continuation lines inside a broken group are indented by at least one level.
    *   **Re-indent rules for `Unknown` and `Verbatim`:**
        *   shift = target indent − original indent of the first line;
        *   the shift applies to every line except `AbapStar` lines, lines inside a block comment, and lines inside a multi-line literal or template;
        *   after the shift, a line that is not a comment line and is not the first line is clamped to an indent of at least `indent`;
        *   a tab in leading indentation counts as `indent` columns, and shifted lines are written with spaces.
    *   **`Opaque` ranges** are copied byte for byte.
    *   **`AlignGroup`** is printed only in broken context, with rows separated by `HardLine`.
*   **Why**: Pipeline step 8.

##### 35. `crates/origami_layout/src/align.rs` (Create) [M1]

*   **Type / Scope**: AlignGroup resolution.
*   **Proposed Changes**:
    *   A `Comment` or `Blank` item ends one alignment segment and starts the next.
    *   Within a segment, the anchor of each row is padded to the widest `head`.
    *   A segment is printed with single spaces instead when either of these holds:
        *   any padded row would exceed the width;
        *   any anchor token is glued to the token before it.
*   **Why**: D7 and STYLE rules `ABAP-ALIGN-001..003`.

##### 36. `crates/origami_layout/tests/printer.rs` (Create) [M0 identity, M1 full]

*   **Type / Scope**: Unit tests.
*   **Proposed Changes**:
    *   [M0] Identity mode returns the input.
    *   [M1] Fits and breaks, break propagation, nested groups, indentation, `BlankLine` collapsing, the glued rule (the builder emits nothing for glued tokens), each comment rule, each re-indent rule including the clamp, each alignment segment rule.
    *   [M1] Linear-time assertion: `time(50k-line doc) / time(10k-line doc) < 7`.

#### Crate `origami_abap`

##### 37. `crates/origami_abap/Cargo.toml` (Create) [M0]

*   **Type / Scope**: Library crate manifest (see the dependency matrix).

##### 38. `crates/origami_abap/src/kinds.rs` (Create) [M0, extended M1]

*   **Type / Scope**: `SyntaxKind` enum for ABAP and the `rowan::Language` implementation.
*   **Proposed Changes**:
    *   Token kinds: word, literal, template part, operator, `.`, `:`, `,`, brackets, pragma, opaque body, and every trivia kind.
    *   Node kinds: one per statement kind, plus `UNKNOWN_STATEMENT` and expression kinds.

##### 39. `crates/origami_abap/src/lexer.rs` (Create) [M0]

*   **Type / Scope**: Lossless ABAP lexer.
*   **Proposed Changes**:
    *   **Signature:** `pub fn lex(src: &SourceText) -> Result<TokenStream, Diagnostic>`, with the modes from the Technical Plan.
    *   **Token boundaries:** ABAP requires whitespace around arithmetic and comparison operators. The lexer therefore splits only at:
        *   whitespace;
        *   the characters `. , : ( ) [ ] { } ' \` | "`;
        *   the selectors `->`, `=>`, `~`, when they stand between two non-blank characters (for example `obj->m`, `meth( )->m`);

        The component selector `-` is not split: `ls_struc-comp` and hyphenated keywords such as `END-OF-DEFINITION` stay one word, and the M1 parser splits component paths. (Changed during M0: splitting `-` would have broken hyphenated keywords and gives the self-check no extra safety.)
        *   a leading `@`.
    *   **Words:** a word is any other run of characters. That includes `/NS/NAME`, `%cid`, `%key`, `%control`, `<fs>`, `!name`, `#`, `*` and `$`.
    *   **Comments:** a `*` in column 1 is an `AbapStar` comment. `"!` is `AbapDoc`, `"#EC` is `AbapPseudo`, and any other `"` is `AbapQuote`.
    *   **Pragmas:** `##NAME` and `##NAME[...]` are pragma tokens.
    *   **Flags:** it computes the `glued` flag for every token.
    *   **Errors:** E001 for unterminated literals and templates.
    *   **Verbatim bodies:** calls `verbatim::scan_body` (item 41) after an opener statement.
*   **Why**: Pipeline step 3.

##### 40. `crates/origami_abap/src/lexer/template.rs` (Create) [M0]

*   **Type / Scope**: String-template sub-lexer.
*   **Proposed Changes**: Handles nested `|…{ expr }…|`, the escapes `\|`, `\{`, `\}`, `\\`, and embedded expressions spanning lines. Literal parts are atomic tokens.

##### 41. `crates/origami_abap/src/verbatim.rs` (Create) [M0]

*   **Type / Scope**: Verbatim-region scanning (D8).
*   **Proposed Changes**:
    *   **Two-phase lexing:**
        1.  The lexer lexes normally.
        2.  When it emits the `.` that ends a statement whose first tokens are `METHOD <name> BY DATABASE PROCEDURE|FUNCTION|GRAPH WORKSPACE`, `EXEC SQL`, or `DEFINE <name>`, it calls `pub fn scan_body(src: &str, offset_after_period: usize, kind: RegionKind) -> Result<usize, Diagnostic>`.
        3.  `scan_body` returns the offset of the first byte of the closer keyword (`ENDMETHOD`, `ENDEXEC`, `END-OF-DEFINITION`) at statement start.
        4.  The body is the bytes from directly after the opener's `.` up to that offset. It includes the newline after the `.` and the closer line's leading indentation, and is emitted as one opaque token.
        5.  The closer is then lexed normally.
    *   **Scanner:** for AMDP, the scanner skips `"…"` identifiers, `'…'` strings, `--` comments and `/* */` comments. For `EXEC SQL` and `DEFINE`, a plain scan for the closer at statement start is used.
    *   **Comments:** a commented-out opener never matches, because matching runs on tokens.
    *   **Errors:** a missing closer gives E004.

##### 42. `crates/origami_abap/src/structure.rs` (Create) [M0]

*   **Type / Scope**: Structural pass.
*   **Proposed Changes**:
    *   Splits the token stream into statements at `.`.
    *   A `,` is a chain separator only when the statement contains a `:` before it and the comma is at bracket depth 0.
    *   Checks bracket balance (E002), a final terminator (E003) and block pairs (E004) via `block_pairs.rs`.
    *   Returns a `StatementList` with the block depth of each statement, and the `Unit` list.
    *   An E004 found on a corpus file is a table bug and is fixed in `block_pairs.rs`.
*   **Why**: Pipeline step 4. Together with E001 and E005, this pass is the only source of structural errors after decoding and language detection.

##### 43. `crates/origami_abap/src/block_pairs.rs` (Create) [M0]

*   **Type / Scope**: Block-pair data and the `SELECT` loop predicate.
*   **Proposed Changes**:
    *   **Matching rule:** an opener matches only at the first word(s) of a statement. A `SELECT` inside brackets, after `FOR`, or inside `WITH` is not a statement start.
    *   `const BLOCK_PAIRS: &[BlockPair]` holds:

        | Opener (statement starts with) | Closer | Intermediates | Not an opener when |
        |---|---|---|---|
        | `CLASS … DEFINITION` / `CLASS … IMPLEMENTATION` | `ENDCLASS` | — | the statement contains `DEFERRED`, `LOAD` or `LOCAL FRIENDS` |
        | `INTERFACE` | `ENDINTERFACE` | — | the statement contains `DEFERRED` or `LOAD` |
        | `METHOD` | `ENDMETHOD` | — | — |
        | `FUNCTION` | `ENDFUNCTION` | — | — |
        | `FORM` | `ENDFORM` | — | — |
        | `MODULE` | `ENDMODULE` | — | — |
        | `IF` | `ENDIF` | `ELSEIF`, `ELSE` | — |
        | `CASE` | `ENDCASE` | `WHEN` (incl. `WHEN OTHERS`, `WHEN TYPE`) | — |
        | `LOOP` | `ENDLOOP` | — | — |
        | `DO` | `ENDDO` | — | — |
        | `WHILE` | `ENDWHILE` | — | — |
        | `TRY` | `ENDTRY` | `CATCH`, `CLEANUP` | — |
        | `CATCH SYSTEM-EXCEPTIONS` | `ENDCATCH` | — | — (a plain `CATCH` is a `TRY` intermediate) |
        | `AT FIRST`, `AT LAST`, `AT NEW`, `AT END OF` | `ENDAT` | — | any other `AT …` (event blocks such as `AT SELECTION-SCREEN`, `AT LINE-SELECTION`, `AT USER-COMMAND`, `AT PFnn` have no closer) |
        | `SELECT` | `ENDSELECT` | — | `select_is_loop` is false |
        | `WITH` | `ENDWITH` | — | `select_is_loop` of the main query is false |
        | `PROVIDE` | `ENDPROVIDE` | — | — |
        | `ON CHANGE OF` | `ENDON` | — | — |
        | `TEST-SEAM` | `END-TEST-SEAM` | — | — |
        | `TEST-INJECTION` | `END-TEST-INJECTION` | — | — |
        | `ENHANCEMENT` | `ENDENHANCEMENT` | — | — |
        | `ENHANCEMENT-SECTION` | `END-ENHANCEMENT-SECTION` | — | — |
        | `DEFINE` | `END-OF-DEFINITION` | — | — |
        | `EXEC SQL` | `ENDEXEC` | — | — |

    *   **Loop predicate:** `pub fn select_is_loop(stmt: &[Token], src: &str) -> bool` returns true when all of these hold:
        *   the statement has no `SINGLE`;
        *   one of these is true:
            *   the target after `INTO` or `APPENDING` is not `TABLE`, and the select list is not aggregate-only without `GROUP BY`;
            *   the target is `TABLE` and `PACKAGE SIZE` is present.
    *   **Data vs code:** every row is data except the `SELECT`/`WITH` predicate, which is code.
*   **Why**: indentation and E004 depend on it. A new row is a one-line change.

##### 44. `crates/origami_abap/src/parser/mod.rs` (Create) [M0 skeleton, M1 full]

*   **Type / Scope**: Parser entry, dispatch and recovery.
*   **Proposed Changes**:
    *   **Signature:** `fn parse(stream: &mut TokenStream, statements: &StatementList) -> (SyntaxNode, Vec<Diagnostic>)`.
    *   **Rowan tree:** contains every token and every trivia piece, so rowan text offsets equal source offsets. The `TokenIndex` of a rowan token is found by binary search over token span starts.
    *   **Dispatch:** on the first one to three words of each statement.
    *   **Recovery:** on failure, it wraps the statement in `UNKNOWN_STATEMENT`, emits W003 and continues at the next statement.
    *   **Depth limit:** nesting beyond depth 200 turns the statement into an unknown statement and emits W004.
    *   **Keywords:** only tokens it consumes as keywords get `keyword = true`.
    *   **M0 state:** every statement is `UNKNOWN_STATEMENT`. No syntax tree is built yet, so the `rowan` dependency arrives with the first M1 parser. In identity mode W003 is not emitted, because nothing is formatted; the corpus report counts every statement as fallback instead.
    *   **M1 order:** statement kinds are added in order of corpus frequency from `cargo xtask freq`.
*   **Why**: D4, D5 and the fallback design.

##### 45. `crates/origami_abap/src/parser/{class.rs, declarations.rs, control.rs, expressions.rs, calls.rs, itab.rs, sql.rs, eml.rs, misc.rs}` (Create) [M1]

*   **Type / Scope**: Statement parsers grouped by area, written by hand from the SAP keyword documentation.
*   **Proposed Changes**:

    | File | Covers |
    |---|---|
    | `class.rs` | `CLASS … DEFINITION`/`IMPLEMENTATION` (incl. `DEFERRED`, `LOCAL FRIENDS`), sections, `METHODS`, `CLASS-METHODS`, `INTERFACES`, `ALIASES`, `EVENTS`, `INTERFACE` |
    | `declarations.rs` | `DATA`, `TYPES`, `CONSTANTS`, `CLASS-DATA`, `STATICS`, `FIELD-SYMBOLS`, `BEGIN OF`/`END OF`, inline declarations |
    | `control.rs` | `IF`/`ELSEIF`/`ELSE`, `CASE`/`WHEN`, `LOOP AT` (with `GROUP BY`), `DO`, `WHILE`, `TRY`/`CATCH`/`CLEANUP`, `RETURN`, `EXIT`, `CONTINUE`, `CHECK` |
    | `expressions.rs` | operands, arithmetic, logical and relational expressions, method-call chains incl. functional calls, constructor expressions (`VALUE`, `NEW`, `CONV`, `CORRESPONDING`, `COND`, `SWITCH`, `REDUCE`, `FILTER`, `REF`, `CAST`, `EXACT`), table expressions, string templates |
    | `calls.rs` | standalone method calls, `CALL METHOD`, `CALL FUNCTION`, `RAISE EXCEPTION`, `RAISE` (RAP and events), `MESSAGE` |
    | `itab.rs` | `APPEND`, `INSERT`, `READ TABLE`, `MODIFY`, `DELETE`, `SORT`, `COLLECT`, `CLEAR`, `FREE` |
    | `sql.rs` | ABAP SQL `SELECT` (incl. `WITH`, joins, `UNION`), `INSERT`, `UPDATE`, `MODIFY`, `DELETE` on database tables |
    | `eml.rs` | `READ ENTITIES`, `READ ENTITY`, `MODIFY ENTITIES`, `MODIFY ENTITY`, `COMMIT ENTITIES`, `ROLLBACK ENTITIES`, `GET PERMISSIONS`, `SET LOCKS`, `RAISE ENTITY EVENT` |
    | `misc.rs` | every remaining statement kind (for example `ASSIGN`, `UNASSIGN`, `CONCATENATE`, `SPLIT`, `FIND`, `REPLACE`, `CONDENSE`, `GET TIME STAMP`) |

*   **Why**: M1 content, split by area so each file is reviewable.

##### 46. `crates/origami_abap/src/layout/{mod.rs, params.rs, class.rs, declarations.rs, control.rs, expressions.rs, calls.rs, itab.rs, sql.rs, eml.rs, misc.rs}` (Create) [M0 `mod.rs`, M1 others]

*   **Type / Scope**: Layout functions.
*   **Proposed Changes**:
    *   **`mod.rs`:**
        *   Walks the tree and emits block indentation from the statement depth.
        *   Emits `blank_line()` where the input had at least one blank line between statements.
        *   Maps `UNKNOWN_STATEMENT` to `unknown(range)` and the AMDP, `EXEC SQL` and macro bodies to `opaque(range)`.
        *   Maps each unit in the quarantine set to `verbatim(range)`. A unit that `check.rs` marks for the second-level W002 fallback is mapped to `opaque(range)`.
    *   **`params.rs`:** `param_list()` lays out every parameter list and emits alignment A1. It is used by `calls.rs` and `expressions.rs`.
    *   **Other modules:** turn their node kinds into Doc IR following the STYLE rules whose IDs are cited in doc comments. A2 is emitted in `expressions.rs`, A3 in `declarations.rs`.

##### 47. `crates/origami_abap/src/lib.rs` (Create) [M0]

*   **Type / Scope**: Crate API.
*   **Proposed Changes**:
    *   `pub fn lex(src: &SourceText) -> Result<TokenStream, Diagnostic>` re-exports the lexer.
    *   `pub fn format(src: &SourceText, opts: &LangOptions, quarantine: &BTreeSet<UnitId>, opaque: &BTreeSet<UnitId>, identity: bool) -> LangResult` runs lex → structure → parse → layout → print.
    *   It builds `PrintOptions` with `keyword_case = Some(opts.abap_keyword_case)` and the given `identity` flag.
    *   It performs no self-check; that is the core's job.

##### 48. `crates/origami_abap/tests/lexer.rs` (Create) [M0]

*   **Type / Scope**: Lexer unit tests.
*   **Proposed Changes**:
    *   **Glued flags:** `a-b` vs `a - b`, `lv+off(len)`, `meth( a )`, `DATA(x)`, `@lv`, `itab[ 1 ]`, `'text'(001)`, `\_assoc`.
    *   **Word forms:** `/DMO/CL_FLIGHT`, `%cid`, `%key`, `%control`, `<fs>`, `!name`, `obj->m( )`, `cl=>m( )`, `intf~m`.
    *   **Other forms:** every comment kind, pragmas, nested templates, every verbatim region kind including a commented-out opener.
    *   **Errors:** every E001 case.

#### Crate `origami_cds`

##### 49. `crates/origami_cds/Cargo.toml` (Create) [M0]

*   **Type / Scope**: Library crate manifest (see the dependency matrix).

##### 50. `crates/origami_cds/src/kinds.rs` (Create) [M0, extended M2/M3]

*   **Type / Scope**: `SyntaxKind` enum for the CDS family. It has node kinds per language plus `UNKNOWN_ELEMENT`.

##### 51. `crates/origami_cds/src/lexer.rs` (Create) [M0]

*   **Type / Scope**: Lossless CDS-family lexer.
*   **Proposed Changes**:
    *   **Signature:** `pub fn lex(src: &SourceText, lang: Language) -> Result<TokenStream, Diagnostic>`.
    *   **Comments:** `//` and `/* */` in all five languages. `--` is a line comment in DDLS, DDLX and DCL only. This is one flag per language, verified by the corpus run at M2/M3 and by the activation check (item 78).
    *   **Identifiers:** `[A-Za-z_][A-Za-z0-9_]*` with optional namespace prefixes `/NS/`. A `$` followed by an identifier is one token (`$projection`, `$parameters`, `$session`, `$extension`). A `#` followed by an identifier is an enum value. `.` is a separate token between path steps.
    *   **Other tokens:** strings, numbers, and the annotation starts `@` and `@<`.
    *   **Flags:** it computes the `glued` flag for every token.
    *   **Errors:** E001 for unterminated strings and block comments.

##### 52. `crates/origami_cds/src/structure.rs` (Create) [M0]

*   **Type / Scope**: Structural pass.
*   **Proposed Changes**:
    *   Bracket and brace balance (E002).
    *   The CDS family raises no E003. A missing `;` is not a structural error, so the parser's fallback handles it.
    *   Returns the `Unit` list.

##### 53. `crates/origami_cds/src/parser/{mod.rs, annotations.rs, expressions.rs, ddls.rs, ddlx.rs, dcl.rs, bdef.rs, srvd.rs}` (Create) [M0 `mod.rs`; M2 annotations, expressions, ddls, ddlx, dcl; M3 bdef, srvd]

*   **Type / Scope**: Hand-written parsers.
*   **Proposed Changes**:

    | File | Covers |
    |---|---|
    | `mod.rs` | recovery to the next `;`, select-list `,`, or closing brace (W003); depth limit (W004); keyword marking; same rowan and `&mut TokenStream` rules as item 44 |
    | `annotations.rs` | shared annotation grammar: dotted names, literals, enum values, arrays, objects, `@<` post-annotations |
    | `expressions.rs` | `case`, `cast`, arithmetic, comparisons, functions, path expressions with filters and cardinality |
    | `ddls.rs` | view entity (root, transient, projection), legacy `define view` with `@AbapCatalog.sqlViewName`, custom and abstract entity, table entity, table function, scalar function, external entity, hierarchy, `extend view entity`, associations, compositions, parameters |
    | `ddlx.rs` | `annotate entity` / `annotate view` |
    | `dcl.rs` | `define role`, `grant select on … where`, conditions, `define aspect`, `define pfcg_mapping`, `define accesspolicy` |
    | `bdef.rs` | header (managed / unmanaged / abstract / projection / interface, `strict ( n )`, `with draft`), behavior blocks, field and operation lists, actions, determinations, validations, mappings, extensions |
    | `srvd.rs` | `define service`, `extend service`, `expose … as …` |

##### 54. `crates/origami_cds/src/layout/{mod.rs, annotations.rs, expressions.rs, ddls.rs, ddlx.rs, dcl.rs, bdef.rs, srvd.rs}` (Create) [same milestones as item 53]

*   **Type / Scope**: Layout functions per parser module.
*   **Proposed Changes**:
    *   The CDS printer never changes letter case: `PrintOptions.keyword_case = None`.
    *   No alignment (D7 is ABAP-only).
    *   `mod.rs` handles units, quarantine and blank lines as in item 46.

##### 55. `crates/origami_cds/src/lib.rs` (Create) [M0]

*   **Type / Scope**: Crate API.
*   **Proposed Changes**:
    *   Declares the modules `kinds`, `lexer`, `structure`, `parser`, `layout`.
    *   Exports `lex` (item 51) and `pub fn format(src: &SourceText, lang: Language, opts: &LangOptions, quarantine: &BTreeSet<UnitId>, opaque: &BTreeSet<UnitId>, identity: bool) -> LangResult`.

##### 56. `crates/origami_cds/tests/lexer.rs` (Create) [M0]

*   **Type / Scope**: Lexer unit tests.
*   **Proposed Changes**:
    *   Each comment form per language, including `--` being rejected as a comment in BDEF and SRVD.
    *   Namespaced names, `$` names, enum values, `@<`.
    *   `_Assoc.field` paths and filters.
    *   Every E001 case.

#### Crate `origami_core`

##### 57. `crates/origami_core/Cargo.toml` (Create) [M0]

*   **Type / Scope**: Library crate manifest.
*   **Proposed Changes**:
    *   Dependencies as in the dependency matrix.
    *   `[features] fault-injection = []`.
    *   `[[bench]] name = "format"`, `harness = false`.

##### 58. `crates/origami_core/src/lib.rs` (Create) [M0]

*   **Type / Scope**: The `format` facade.
*   **Proposed Changes**:
    *   **Signature:** `pub fn format(input: &[u8], hint: LanguageHint, opts: &Options) -> FormatResult`.
    *   **Flow:** decode → detect → language crate → self-check and quarantine (`check.rs`) → encode, all inside `std::panic::catch_unwind`. A caught panic becomes X002, with its message taken from the panic payload.
    *   **Identity switch:** `const IDENTITY_MODE: bool = true;` is passed as `identity` to the language crates. The first M1 pull request sets it to false.
*   **Why**: the single entry point for the CLI, the fuzzer and the benchmarks.

##### 59. `crates/origami_core/src/options.rs` (Create) [M0]

*   **Type / Scope**: Options and validation.
*   **Proposed Changes**:
    *   `pub struct Options { lang: LangOptions, style_edition: Edition, #[cfg(feature = "fault-injection")] fault: Option<Fault> }`.
    *   `Options::validate()` rejects `line_width` outside 40..=255, `indent_width` outside 1..=8, and unknown editions. A rejection gives E020.
    *   `enum Fault { DropSpaceOnce, DropSpaceAlways, Panic }`, only with the `fault-injection` feature.

##### 60. `crates/origami_core/src/detect.rs` (Create) [M0]

*   **Type / Scope**: Language detection.
*   **Proposed Changes**:
    *   `pub enum LanguageHint { None, Path(String), LanguageId(String), Explicit(Language), CdsFamily }`.
    *   The file-pattern table from "Supported inputs".
    *   Content sniffing on the first significant keywords, using the list in the Technical Plan step 2.
    *   **Path mode:** an unmatched name gives E011.
    *   **stdin mode and the `acds` language ID:** an unmatched name falls back to sniffing. If sniffing fails, the result is E010.

##### 61. `crates/origami_core/src/check.rs` (Create) [M0]

*   **Type / Scope**: Self-check, quarantine and W002 ownership.
*   **Proposed Changes**:
    1.  `r = lang::format(src, opts, ∅, ∅)`.
    2.  Re-lex `r.output` with the same crate's `lex`, then run `equivalent(r.input_tokens, …)`.
    3.  Collect the units that contain each mismatch's `input_index`. Also collect, for ABAP, the units of output lines over 255 characters that were not over 255 in the input.
    4.  If the set is non-empty, call `format(…, quarantine = set, ∅)`, emit W001 for mismatch units and W002 for length units, then re-check.
    5.  Units still over 255 characters are re-printed with `opaque = those units`.
    6.  If the comparison still fails, return X001 with the first mismatch.
    *   With the `fault-injection` feature, the faults are applied here between print and check.
*   **Why**: Pipeline step 9. This is the only owner of W001, W002 and X001.

##### 62. `crates/origami_core/src/result.rs` (Create) [M0]

*   **Type / Scope**: Result types.
*   **Proposed Changes**: `FormatResult { output: Option<String>, changed: bool, language: Option<Language>, status: Status, diagnostics, stats: Stats }` and `enum Status { Formatted, Unchanged, Error, Skipped }`, all serializable with `serde`.

##### 63. `crates/origami_core/benches/format.rs` (Create) [M0 harness, M1+ data]

*   **Type / Scope**: `criterion` benchmarks.
*   **Proposed Changes**:
    *   Per language, one benchmark on the corpus file at the median line count, read from `corpus/repos/` after `cargo xtask corpus fetch`.
    *   One benchmark on a generated 50,000-line ABAP file.

##### 64. `crates/origami_core/tests/rules.rs` and `crates/origami_core/tests/rules/<ID>/{input.<name>, expected.<name>}` (Create) [M0 harness, cases per milestone]

*   **Type / Scope**: The single snapshot harness for all STYLE rules.
*   **Proposed Changes**:
    *   **Inputs:**
        *   It reads the rule IDs and `Milestone:` lines from `docs/STYLE.md` using the heading format in item 14.
        *   `<name>` is a full abapGit-style file name (`input.clas.abap`, `input.ddls.asddls`), which decides the language.
    *   **Milestone gate:** it skips rules whose milestone is greater than `const CURRENT_MILESTONE: u8`.
    *   **Checks:**
        *   for each remaining rule, `format(input) == expected` and `format(expected) == expected`, with no W001 or X001;
        *   every rule has a directory;
        *   every rule directory has a STYLE entry.

##### 65. `crates/origami_core/tests/detect.rs`, `tests/options.rs`, `tests/safety.rs` (Create) [M0]

*   **Type / Scope**: Core tests.
*   **Proposed Changes**:
    *   `detect.rs`: every file pattern and every sniffing keyword, plus E010, E011 and the stdin fallback.
    *   `options.rs`: every validation bound.
    *   `safety.rs`, with `--features fault-injection`:
        *   `DropSpaceOnce` gives W001 and correct output;
        *   `DropSpaceAlways` gives X001 and no output;
        *   `Panic` gives X002 and no output.

#### Crate `origami_cli` (published as `abap-origami`, binary `origami`)

##### 66. `crates/origami_cli/Cargo.toml` (Create) [M0]

*   **Type / Scope**: Binary crate manifest.
*   **Proposed Changes**: `name = "abap-origami"`, `[[bin]] name = "origami"`, with the dependencies from the dependency matrix.

##### 67. `crates/origami_cli/src/main.rs`, `crates/origami_cli/src/args.rs` (Create) [M0]

*   **Type / Scope**: Entry point and arguments.
*   **Proposed Changes**:
    *   **`args.rs`:**
        *   `clap` derive definitions for every flag in the CLI contract, with `disable_version_flag = true`.
        *   `--version` and `--version --json` are implemented by hand.
        *   `--stdin-filepath` implies `--stdin`.
    *   **`main.rs`, panic hook:** installs `std::panic::set_hook` with a hook that prints nothing and stores `info.location()` in a thread-local. The location is appended to any X002 message.
    *   **`main.rs`, dispatch:** dispatches to stdin or path mode, and exits with the aggregated code.
    *   **`--version --json` output:** `{"version":"1.0.0","style_edition":"2026","json_schema":1,"required_version":">=1.0, <2" | null,"satisfies_required_version":true | false | null}`. With `--config <path>`, the binary evaluates that file's `required_version` against its own version.

##### 68. `crates/origami_cli/src/config.rs` (Create) [M0]

*   **Type / Scope**: Config discovery and merge.
*   **Proposed Changes**:
    *   **`--config FILE`** uses only that file, with no discovery. **`--no-config`** uses defaults and flags only.
    *   **Discovery:**
        *   It checks each directory from the file's directory upwards, including the directory that contains `.git`, and stops after it.
        *   In stdin mode without `--stdin-filepath`, it starts at the current working directory.
        *   Results are cached per directory.
    *   **Keys:** `style_edition`, `required_version`, `line_width`, `indent_width`, `preview`, `[abap] keyword_case`. Unknown keys give W101.
    *   **Merge order:** defaults, then file, then flags.
    *   **Errors:** a parse error, an invalid value or a `required_version` mismatch gives E020, and the run aborts with exit 3 before any file is processed.

##### 69. `crates/origami_cli/src/walk.rs` (Create) [M0]

*   **Type / Scope**: File discovery.
*   **Proposed Changes**:
    *   `ignore::WalkBuilder` with these settings:
        *   `hidden(true)`
        *   `git_ignore(true)`
        *   `git_exclude(true)`
        *   `git_global(false)`
        *   `require_git(false)`
        *   `parents(true)`
        *   `add_custom_ignore_filename(".origamiignore")`
        *   `follow_links(false)`
    *   Results are filtered by the supported-inputs table and sorted by path.

##### 70. `crates/origami_cli/src/run.rs` (Create) [M0]

*   **Type / Scope**: Execution.
*   **Proposed Changes**:
    *   **Files:** processes files in parallel with `rayon` (`-j`), and collects results in path order.
    *   **Modes:** `--check` never writes. `--write`, the default for paths, writes only changed files.
    *   **Exit code:** aggregated with the precedence 70 > 4 > 3 > 2 > 5 > 1 > 0, using each code's exit value from the code table.
    *   **stdin success:** file status `formatted` or `unchanged`, even when the exit code is 5. On success, the text goes to stdout. With `--check --stdin`, stdout stays empty, and the exit code is 1 if the input would change.

##### 71. `crates/origami_cli/src/io.rs` (Create) [M0]

*   **Type / Scope**: Atomic writes.
*   **Proposed Changes**:
    *   Writes to a `tempfile::NamedTempFile` in the same directory, copies the permissions, then calls `persist`.
    *   An unchanged file is never opened for writing.
    *   A read failure gives E030; a write failure gives E031.

##### 72. `crates/origami_cli/src/report/{mod.rs, human.rs, json.rs, sarif.rs, diff.rs}` (Create) [M0]

*   **Type / Scope**: Output formats and routing.
*   **Proposed Changes**:
    *   **Routing:** diagnostics in every format go to stderr. stdout carries only formatted text (stdin mode) or the diff (`--diff`).
    *   `mod.rs` selects the formatter.
    *   `human.rs` prints `path:line:col: severity[code]: message`.
    *   `json.rs` conforms to `schemas/diagnostics.v1.json`.
    *   `sarif.rs` produces SARIF 2.1.0, with one rule per code.
    *   `diff.rs` produces a unified diff via `similar`.
    *   `--max-diagnostics` caps the diagnostics per run and sets the top-level `truncated` to true.

##### 73. `crates/origami_cli/tests/cli.rs` (Create) [M0]

*   **Type / Scope**: CLI integration tests with `assert_cmd`.
*   **Proposed Changes**:
    *   **Exit codes:** every exit code, and aggregation over mixed files.
    *   **stdin mode:** success, error (empty stdout), `changed`, `--check --stdin`, `--stdin-filepath` without `--stdin`.
    *   **Writes:** `--check` writes nothing; unchanged files keep their modification time; CRLF, BOM and permissions are preserved.
    *   **Config:** discovery and precedence, `--config`, `--no-config`, a `required_version` mismatch (E020), an unknown key (W101).
    *   **Output:** JSON validated against the schema with `jsonschema`; stream routing; `--version --json`.
    *   **Determinism:** identical output for `-j 1` and `-j 8`.

#### Test infrastructure

##### 74. `corpus/manifest.toml` (Create) [M0]

*   **Type / Scope**: Corpus definition.
*   **Proposed Changes**: Each entry has `url`, `commit`, `spdx`, `include = ["src/**"]` and `modern`. The `commit` is the default-branch HEAD on the M0 start date.

    | Repository | `modern` |
    |---|---|
    | `github.com/abapGit/abapGit` | false |
    | `github.com/SAP-samples/abap-platform-refscen-flight` | true |
    | `github.com/SAP-samples/abap-cheat-sheets` | true |
    | `github.com/open-abap/open-abap` | false |
    | `github.com/sbcgua/ajson` | false |
    | `github.com/abap2xlsx/abap2xlsx` | false |

    *   The `modern` flag selects the files counted for the M1 exit criterion.
    *   The sources are fetched to `corpus/repos/` and never committed.

##### 75. `corpus/baseline.json` (Create) [M0]

*   **Type / Scope**: Committed corpus baseline.
*   **Proposed Changes**: Per language, the fallback rate and the statement counts. It is updated only by `cargo xtask corpus bless`, in the pull request that changes it.
*   **Why**: the reference for regression checks.

##### 76. `xtask/Cargo.toml`, `xtask/src/{main.rs, corpus.rs, freq.rs, oracle.rs, bench_check.rs, release_diff.rs}` (Create) [M0]

*   **Type / Scope**: Developer task runner (`cargo xtask …`, `publish = false`).
*   **Proposed Changes**:

    | Task | What it does |
    |---|---|
    | `corpus fetch` | clones the pinned commits |
    | `corpus run` | formats every file twice in memory, then asserts: no E-codes on corpus files that activate, no W001/X-codes, and idempotency. It computes the fallback rate per language, as statements with W003 or W004 divided by all statements, separately for `modern` files. It fails when a rate is higher than in `corpus/baseline.json`. |
    | `corpus bless` | rewrites `corpus/baseline.json` |
    | `freq` | counts statements by their leading keywords across the corpus, giving the M1 implementation order |
    | `oracle` | runs `tools/abaplint-oracle` on the input and output of every ABAP corpus file, and compares type plus text per token. Word tokens outside strings, templates and comments are compared case-insensitively. |
    | `bench-check` | reads the criterion median from `target/criterion/<id>/new/estimates.json` as the p50, and the p95 of a `hyperfine --export-json` file with 50 runs; compares both to G4 with a 20% tolerance |
    | `release-diff` | downloads the previous release binary with `gh release download`, formats the corpus with it and with the current build, and writes the diff for the release notes |

##### 77. `tools/abaplint-oracle/{package.json, package-lock.json, lex.mjs}` (Create) [M0]

*   **Type / Scope**: Independent lexer oracle (Node.js, CI only).
*   **Proposed Changes**:
    *   `package.json` pins `@abaplint/core` to an exact version.
    *   `lex.mjs` reads file paths from stdin and prints NDJSON, one line per file: `{path, tokens:[{type, text, row, col}]}`.
    *   abaplint's punctuation and operator token types encode adjacent whitespace (for example `WParenLeft`); word tokens do not.
*   **Why**: catches lexer bugs that our own self-check would share. This is not shipped.

##### 78. `tools/sap-activation-check/{package.json, package-lock.json, check.mjs}` (Create) [M3]

*   **Type / Scope**: SAP-system oracle (Node.js, semi-automated).
*   **Proposed Changes**: The procedure runs once per corpus repository.
    1.  Import the original repository with abapGit into package `$ORIGAMI_<REPO>`. If the objects already exist in the system (for example `/DMO/` on an ABAP Platform Trial), pull over them.
    2.  Run `node check.mjs activate --package <pkg> --out before.json`. It lists objects through ADT `GET /sap/bc/adt/repository/nodestructure`, activates them through `POST /sap/bc/adt/activation?method=activate&preauditRequested=true`, and records the results.
    3.  Run `origami --write` on a copy of the repository, and pull that copy with abapGit.
    4.  Run `node check.mjs activate … --out after.json`.
    5.  Run `node check.mjs compare before.json after.json`. Per object, it compares the activation status and the multiset of (message type, message ID), ignoring positions.

    *   **HTTP:** Node 22 `fetch`, with a CSRF token (`x-csrf-token: fetch`) and a cookie jar.
    *   **Environment variables:** `SAP_URL`, `SAP_CLIENT`, `SAP_USER`, `SAP_PASSWORD`.
    *   **System:** requires a system from Open Question 5.

##### 79. `fuzz/Cargo.toml`, `fuzz/fuzz_targets/format_abap.rs`, `fuzz/fuzz_targets/format_cds.rs` (Create) [M0]

*   **Type / Scope**: `cargo-fuzz` targets (nightly, own workspace, `publish = false`).
*   **Proposed Changes**:
    *   `fuzz/Cargo.toml` pins `libfuzzer-sys` and the path dependency on `origami_core` directly.
    *   Each target calls `origami_core::format` with a fixed language hint.
    *   It asserts that there is no diagnostic with code X001 or X002, and that an `Ok` output formats to itself.
    *   Seed inputs come from the corpus.

##### 80. `scripts/bench-cli.sh` (Create) [M0]

*   **Type / Scope**: End-to-end benchmark.
*   **Proposed Changes**:
    1.  Copy the first 1,000 supported corpus files in path order to `target/bench-input/`, and format them there so that `--check` exits 0.
    2.  Run `hyperfine --runs 50 --export-json` for one median-sized file per language and for the 1,000-file set.
    3.  Call `cargo xtask bench-check`.

#### CI and release

##### 81. `.github/workflows/ci.yml` (Create) [M0]

*   **Type / Scope**: CI on push and pull request.
*   **Proposed Changes**:
    *   **Setup:** `actions/checkout@v4`, `Swatinem/rust-cache@v2`, `actions/setup-node@v4` (Node 22), and `taiki-e/install-action` for `cargo-deny`, `cargo-llvm-cov`, `cargo-about` and `hyperfine`.
    *   **Job `test`** (matrix `ubuntu-latest`, `macos-14`, `windows-latest`): `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `cargo test -p origami_core --features fault-injection`.
    *   **Job `quality`** (`ubuntu-latest`):
        *   `cargo deny check`;
        *   `cargo llvm-cov -p origami_syntax --fail-under-lines 85`, and the same for `origami_layout` and `origami_core`;
        *   `--fail-under-lines 75` for `origami_abap` and `origami_cds`.
    *   **Job `corpus`** (`ubuntu-latest`):
        *   `cargo xtask corpus fetch`, then `cargo xtask corpus run`;
        *   `npm ci` in `tools/abaplint-oracle`, then `cargo xtask oracle`;
        *   `cargo bench -p origami_core`, then `scripts/bench-cli.sh`.
    *   **Job `extension`** (`ubuntu-latest`, from M4): `cargo build`, then in `editors/vscode`: `npm ci && npx tsc --noEmit && npx eslint . && xvfb-run -a npm test`, with `ORIGAMI_TEST_BINARY=target/debug/origami`.

##### 82. `.github/workflows/fuzz.yml` (Create) [M0]

*   **Type / Scope**: Nightly scheduled fuzzing.
*   **Proposed Changes**:
    *   Runs `cargo +nightly fuzz run <target> -- -max_total_time=3600` for each target.
    *   On a crash, it uploads the artifact and runs `gh issue create`, with `permissions: issues: write`.

##### 83. `.github/workflows/release.yml` (Create) [M4]

*   **Type / Scope**: Tag-triggered release (`vX.Y.Z`).
*   **Proposed Changes**:
    1.  **Build** the seven targets in the VS Code packaging list:

        | Target triple | Built on |
        |---|---|
        | `x86_64-apple-darwin`, `aarch64-apple-darwin` | macOS runner |
        | `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc` | Windows runner |
        | `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` | `cargo-zigbuild`, glibc 2.17 |
        | `x86_64-unknown-linux-musl` (alpine) | `cargo-zigbuild` |

    2.  **Sign:** `codesign` and `notarytool` for macOS; `signtool` for Windows. Linux binaries ship with checksums only.
    3.  **Generate** SHA-256 checksums and `THIRD_PARTY_NOTICES.md`.
    4.  **Create the GitHub Release** with the binaries and the `cargo xtask release-diff` output.
    5.  **`cargo publish`** in this order: `origami_syntax`, `origami_layout`, `origami_abap`, `origami_cds`, `origami_core`, `abap-origami`. `xtask` and `fuzz` are never published.
    6.  **Extension:** build one VSIX per platform with the matching binary in `editors/vscode/bin/`, then publish with `vsce` and `ovsx`.

    *   **Versioning:** all crates and the extension share one version.
    *   **Secrets:**
        *   `CARGO_REGISTRY_TOKEN`, `VSCE_PAT`, `OVSX_PAT`;
        *   `APPLE_CERT_P12`, `APPLE_CERT_PASSWORD`, `APPLE_API_KEY`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER`;
        *   `WINDOWS_CERT_PFX`, `WINDOWS_CERT_PASSWORD`.

#### VS Code extension (`editors/vscode`)

##### 84. `editors/vscode/package.json`, `editors/vscode/package-lock.json` (Create) [M4]

*   **Type / Scope**: Extension manifest and lockfile.
*   **Proposed Changes**:
    *   **Identity:** `name: "abap-origami"`, `displayName: "origami – ABAP & CDS formatter"`, `main: "./out/extension.js"`.
    *   **Engine:** `engines.vscode: "^1.90.0"`.
    *   **Scripts:** `build` (`node esbuild.mjs`), `lint` (`eslint .`), `test` (`node ./out/test/runTest.js`).
    *   **`activationEvents`:** `onLanguage:abap`, `onLanguage:acds`, and `workspaceContains` for `**/*.abap`, `**/*.asddls`, `**/*.asddlxs`, `**/*.asdcls`, `**/*.asbdef` and `**/*.srvdsrv` (the abapGit file extensions).
    *   **`contributes.configuration`:** `origami.path` (`scope: "machine"`) and `origami.trace` (boolean).
    *   **Languages:** no `contributes.languages`. The extension does not redefine the `abap`/`acds` language IDs.
    *   **Workspace trust:** `capabilities.untrustedWorkspaces: { supported: "limited", description: "In untrusted workspaces only the bundled origami binary is used." }`.

##### 85. `editors/vscode/src/extension.ts` (Create) [M4]

*   **Type / Scope**: Activation.
*   **Proposed Changes**:
    *   Registers one `DocumentFormattingEditProvider` for the selector `[{ language: "abap" }, { language: "acds" }, { scheme: "file", pattern: "**/*.{abap,asddls,asddlxs,asdcls,asbdef,srvdsrv}" }]`.
    *   Creates the `DiagnosticCollection`, the status-bar item and the output channel `origami`.

##### 86. `editors/vscode/src/binary.ts` (Create) [M4]

*   **Type / Scope**: Binary resolution.
*   **Proposed Changes**:
    *   **Resolution order:** as in the Technical Plan. If `!vscode.workspace.isTrusted`, steps 1–2 are skipped, and resolution runs again on `onDidGrantWorkspaceTrust`.
    *   **Version check:**
        *   The nearest `origami.toml` is found with `workspace.fs.stat`.
        *   Each candidate, the bundled binary included, is checked with `origami --version --json --config <path>`. The extension reads `satisfies_required_version`.
        *   If no candidate satisfies it, the extension does not format, and the status bar shows `origami: version mismatch`.
    *   **Caching:** results are cached per workspace folder, and the cache is invalidated when `origami.toml` changes.

##### 87. `editors/vscode/src/format.ts` (Create) [M4]

*   **Type / Scope**: Formatting request.
*   **Proposed Changes**:
    *   **Spawn:** `origami --stdin --stdin-filepath <name> --diagnostics=json [--language <hint>]`, with a 2-second timeout.
        *   For `file:` documents, `<name>` is the absolute `fsPath`.
        *   For other schemes, it is the name from `uri.ts`, and `--no-config` is added.
        *   The working directory is the document's workspace folder, or the extension's global storage directory.
    *   **Exit codes:**
        *   0 or 5: apply the edits.
        *   2: no edits.
        *   Any other code: no edits, and the status bar shows `origami: error – see Problems`.
    *   **Timeout:** kill the process with SIGKILL, and show `origami: timed out`.
    *   **Cancellation:** the cancellation token kills the process.
    *   **Trace:** with `origami.trace`, each command line and its stderr is logged to the output channel.

##### 88. `editors/vscode/src/edits.ts` (Create) [M4]

*   **Type / Scope**: Minimal edits.
*   **Proposed Changes**: jsdiff `diffLines`, then `diffChars` inside each changed hunk, converted to `TextEdit[]`. Returns `[]` when `changed` is false.

##### 89. `editors/vscode/src/diagnostics.ts` (Create) [M4]

*   **Type / Scope**: Diagnostics mapping.
*   **Proposed Changes**:
    *   Maps JSON diagnostics to `vscode.Diagnostic`. The 1-based line and UTF-16 column become a 0-based line and character.
    *   `code` links to `docs/CLI.md`.
    *   Diagnostics from every run are shown, including W001–W004 from exit-0 runs. They are cleared only when a run produces none.

##### 90. `editors/vscode/src/uri.ts` (Create) [M4]

*   **Type / Scope**: Names and hints for non-file documents.
*   **Proposed Changes**:
    *   **Blocked** until the S2 result amends this item.
    *   For `abap:` URIs, it derives `<object>.<type>.<ext>` from the URI path when S2 shows the type is encoded there.
    *   Otherwise it uses `document.<languageId>` and passes `--language abap` for `abap` and `--language cds` for `acds`.
    *   This mapping is the only ADT-specific code.

##### 91. `editors/vscode/test/{runTest.ts, suite/index.ts, formatting.test.ts, binary.test.ts, edits.test.ts}`, `editors/vscode/test/fixtures/*` (Create) [M4]

*   **Type / Scope**: Extension tests with `@vscode/test-electron` and `mocha`.
*   **Proposed Changes**:
    *   **Setup:**
        *   `runTest.ts` downloads VS Code and runs `suite/index.ts`.
        *   The tests use the binary at `ORIGAMI_TEST_BINARY` through `origami.path`.
    *   **Cases:**
        *   formatting a fixture of each language;
        *   a structural error leaves the document unchanged and shows a diagnostic;
        *   warnings from exit-0 runs are shown;
        *   version-pin resolution, including a mismatch and an untrusted workspace;
        *   minimal edits keep the cursor position.
    *   **CUJ 1:** an in-memory `FileSystemProvider` registered for the `abap` scheme provides a writable virtual BDEF document.

##### 92. `editors/vscode/{tsconfig.json, esbuild.mjs, eslint.config.mjs, .vscodeignore, README.md, CHANGELOG.md}` (Create) [M4]

*   **Type / Scope**: Build, lint, packaging configuration and the Marketplace page.
*   **Proposed Changes**:
    *   `eslint.config.mjs` uses `typescript-eslint` recommended rules.
    *   The README documents the `editor.defaultFormatter` setup for `[abap]` and `[acds]` and links to `docs/adoption.md`.
    *   `.vscodeignore` excludes sources and keeps `bin/` and `out/`.

### Verification Plan

*   **Compilation/Build**: `cargo build --workspace --release`; `npm ci && npm run build` in `editors/vscode`.
*   **Unit Tests**: `cargo test --workspace` (lexers, printer, syntax, core, rule snapshots); `cargo test -p origami_core --features fault-injection`.
*   **Integration Tests**: `crates/origami_cli/tests/cli.rs`; `npm test` in `editors/vscode`.
*   **Static Analysis & Linting**: `cargo fmt --check`; `cargo clippy --workspace -- -D warnings`; `cargo deny check`; `npx tsc --noEmit` and `npx eslint .` in `editors/vscode`.
*   **Test Coverage Requirement**: at least 85% line coverage (`cargo llvm-cov -p <crate>`) for `origami_syntax`, `origami_layout` and `origami_core`, and at least 75% for `origami_abap` and `origami_cds`.
*   **E2E/Integration**:
    *   `cargo xtask corpus run`: no E-codes on corpus files that activate, no W001 and no X-codes, 100% idempotency, and fallback rates not above `corpus/baseline.json` and below the milestone target.
    *   `cargo xtask oracle`: zero mismatches between the abaplint token sequences of input and output, with words compared case-insensitively.
    *   Nightly fuzzing: 1 hour per target with no crash.
    *   M3: `tools/sap-activation-check` reports identical activation results.
*   **Performance**: `cargo xtask bench-check` within 20% of the G4 targets.
*   **Milestone exits**: each milestone's exit criteria from the Technical Plan are checked and recorded in the pull request that closes it.

--------------------------------------------------------------------------------

*Drafted with EGM*
