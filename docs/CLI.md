# origami command-line contract

This contract is frozen at v1.0. Agents and CI scripts can rely on it.

## Usage

```text
origami [OPTIONS] [PATHS...]

  PATHS                 files or directories; directories are walked, honoring
                        .gitignore and .origamiignore; default: current directory
  --stdin               read one source from stdin, write the result to stdout
  --stdin-filepath P    name used for language detection and config lookup;
                        implies --stdin
  --language L          abap | ddls | ddlx | dcl | bdef | srvd |
                        cds (CDS family, detected from content)
  --write               rewrite files in place (default for PATHS)
  --check               write nothing; exit 1 if any file would change
  --diff                with --check: print a unified diff to stdout
  --strict              exit 5 if any warning was emitted
  --lines START:END     format only changes that touch these lines (1-based,
                        inclusive, repeatable); stdin mode or one file
  --changed [REF]       format only lines changed against a git revision
                        (default HEAD) and untracked files in full
  --preview             apply style rules that are still in preview
  --diagnostics F       human | agent | json | sarif; always on stderr.
                        Default: agent if CLAUDECODE is set, else human;
                        env ORIGAMI_DIAGNOSTICS sets it too
  --max-diagnostics N   cap diagnostics per run (default 200)
  --config FILE         use only this config file, no discovery
  --no-config           ignore all config files
  --line-width N        40..=255
  --indent-width N      1..=8
  --keyword-case C      upper | lower (ABAP keywords only)
  -j, --jobs N          worker threads for directory mode (default: CPU count)
  --mcp                 run as an MCP server on stdin/stdout
  --version [--json] [--config FILE]
```

## Partial formatting

`--lines` and `--changed` format the whole source internally, then keep only
the changes that touch the requested lines (a statement that starts or ends
in the range is reformatted completely). The result passes the same
self-check as a full run; if the selected changes cannot stand on their own,
the file stays as written and W001 is reported. `--changed` runs
`git diff -U0 --relative REF` and `git ls-files --others --exclude-standard`
in the current directory; outside a git repository it is a usage error.

## Diagnostics formats

* **human**: one block per file that changed or has findings (`✎` changed,
  `✗` error with a code frame, `!` warnings; at most three W003 lines per
  file), a summary line and the next step. Colors only when stderr is a
  terminal; `NO_COLOR` turns them off, `CLICOLOR_FORCE=1` on. The layout is
  for people and may change between releases.
* **agent**: for AI agents. First line `origami: N files, …`; then one line
  per finding, prefixed `ERROR` / `WARN` with `path:line:column`, code,
  message and a fix hint, and `CHANGE` / `FORMATTED` per changed file; W003
  only as one `NOTE` count; last line `NEXT: …` says what to do. Stable
  within a major version.
* **json**: the schema in `schemas/diagnostics.v1.json` (frozen).
* **sarif**: SARIF 2.1.0.

## MCP server

`origami --mcp` runs a Model Context Protocol server on stdin/stdout
(JSON-RPC 2.0, one message per line; protocol versions 2025-06-18,
2025-03-26, 2024-11-05). Tools:

| Tool | Arguments | Result |
|---|---|---|
| `format` | `source`, optional `filename`, `language`, `lines` (`[[start, end]]`) | formatted text; `structuredContent`: `source`, `changed`, `status`, `diagnostics`. A structural error returns `isError: true` and the diagnostics. |
| `format_files` | optional `paths`, `write` (default false), `changed` (git revision) | summary and unified diffs; `structuredContent.files`: per file `path`, `status`, `changed`, `diagnostics`, `diff` |

Configuration discovery, the self-check and the diagnostic codes are the same
as on the command line. Nothing is written unless `format_files` gets
`"write": true`.

## Streams

* **stdout** carries only formatted text (stdin mode) or the diff (`--diff`).
* **stderr** carries diagnostics in every format.

## stdin mode

* Success means file status `formatted` or `unchanged`, even when the exit
  code is 5. The formatted text then goes to stdout.
* With `--check`, stdout stays empty, and the exit code is 1 if the input would
  change.
* On any error, stdout stays empty. Never write empty stdout back to a source.
* Language: `--language`, else the `--stdin-filepath` name, else content
  sniffing. A name that matches no pattern falls back to sniffing.

## Exit codes

In a multi-file run every file is processed, and the highest code in this order
wins: **70 > 4 > 3 > 2 > 5 > 1 > 0**. Configuration and usage errors (E020,
E021) abort before any file is processed.

| Code | Meaning |
|---|---|
| 0 | success; nothing to report |
| 1 | `--check`: at least one file would change |
| 2 | at least one file has a structural error and was left untouched |
| 3 | usage or configuration error, or an explicitly named unsupported file |
| 4 | I/O error |
| 5 | `--strict`: warnings were emitted (this includes W003, so `--strict` fails on every grammar gap until coverage is complete) |
| 70 | internal error |

## Diagnostic codes

| Code | Severity | Trigger | Exit | Scope | Run continues |
|---|---|---|---|---|---|
| E001 | error | unterminated literal, template or block comment | 2 | file | yes |
| E002 | error | unbalanced bracket | 2 | file | yes |
| E003 | error | ABAP statement without terminating period | 2 | file | yes |
| E004 | error | block-pair mismatch | 2 | file | yes |
| E005 | error | input is not valid UTF-8 | 2 | file | yes |
| E010 | error | language not detected | 2 | file | yes |
| E011 | error | file named explicitly but not a supported type | 3 | file | yes |
| E020 | error | invalid config file or value, or `required_version` mismatch | 3 | run | no |
| E021 | error | invalid command-line usage | 3 | run | no |
| E030 | error | file cannot be read | 4 | file | yes |
| E031 | error | file cannot be written | 4 | file | yes |
| W001 | warning | unit quarantined by the self-check | 0 / 5 | file | yes |
| W002 | warning | unit left as written because of the 255-character limit | 0 / 5 | file | yes |
| W003 | warning | unknown statement formatted generically | 0 / 5 | file | yes |
| W004 | warning | nesting-depth limit reached | 0 / 5 | file | yes |
| W101 | warning | unknown config key | 0 / 5 | run | yes |
| X001 | internal | self-check failed after quarantine | 70 | file | yes |
| X002 | internal | panic caught | 70 | file | yes |

Codes are stable once released. `I`-codes are reserved.

## JSON diagnostics

`--diagnostics=json` writes one object per run to stderr, defined by
[`schemas/diagnostics.v1.json`](../schemas/diagnostics.v1.json).

* `line` is 1-based; `column` is 1-based in UTF-16 code units (as in VS Code);
  `offset` is the 0-based byte offset in the text after CRLF-to-LF conversion.
* `status` is `formatted`, `unchanged`, `error` or `skipped` (config files that
  carry W101).
* `truncated` is true when `--max-diagnostics` cut the list.

## `--version --json`

```json
{"version":"1.0.0","style_edition":"2026","json_schema":1,
 "required_version":">=1.0, <2","satisfies_required_version":true}
```

`required_version` and `satisfies_required_version` are `null` unless
`--config FILE` is given; then the binary evaluates that file's
`required_version` against its own version. The VS Code extension uses this, so
it needs no TOML or semver code.

## Configuration: `origami.toml`

```toml
style_edition    = "2026"       # which version of the style rules to apply
required_version = ">=1.0, <2"  # semver range (Rust syntax); mismatch is E020
line_width       = 120          # 40..=255
indent_width     = 2            # 1..=8
preview          = false

[abap]
keyword_case = "upper"          # "upper" | "lower"
```

* **Discovery:** from each file's directory upwards, including the directory
  that contains `.git`, then stop. In stdin mode without `--stdin-filepath`
  the search starts at the current working directory.
* **Precedence:** flags > file > defaults.
* **Unknown keys:** W101. `.editorconfig` and editor settings are ignored on
  purpose.
