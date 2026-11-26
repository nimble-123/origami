# origami command-line contract

This contract is frozen from v1.0. Until then (0.x), flags, exit codes and
diagnostic codes can change in a release, and the changelog lists each
change. Pin the version with `required_version` to rely on it. Formatted
output follows the same rule: [STYLE.md](STYLE.md), "Stability".

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
  --review              show the changes with notes on introduced line breaks
                        (stdout); writes nothing, implies --check; not with
                        --diff or --write
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
  --style S             aligned (default) | compact (STYLE.md, "Styles")
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
`--changed` takes the next argument as the revision unless it starts with
`-`: `origami --changed main src/` compares against `main`, and
`origami --changed src/` would read `src/` as a revision. To pass paths,
write `--changed=REF path…` or put the paths first (`origami src/
--changed`).

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
| `format_files` | optional `paths`, `write` (default false), `changed` (git revision) | summary and unified diffs; `structuredContent.files`: per file `path`, `status`, `changed`, `diagnostics`, `diff` (or `diff_omitted: true`); `truncated` and `omitted` when the result was shortened |

Configuration discovery, the self-check and the diagnostic codes are the same
as on the command line. Nothing is written unless `format_files` gets
`"write": true`.

Limits:

* A message may be at most 16 MiB long (one line); a longer one gets the
  JSON-RPC error -32600 and the server reads on.
* `lines` must be pairs `[start, end]` with 1 <= start <= end <= 2^32 - 1;
  anything else is the JSON-RPC error -32602 (invalid params).
* Tool calls run one at a time, each with a deadline of 30 s
  (`ORIGAMI_MCP_TIMEOUT` sets it in seconds). A call that misses it gets the
  JSON-RPC error -32001 and is cancelled; with `"write": true`, files
  already rewritten stay rewritten. Other requests (`ping`, `tools/list`)
  are answered while a call runs. `notifications/cancelled` cancels a call;
  it then gets no reply.
* A `format_files` result carries about 100 KB of file entries and diffs.
  Beyond that, the remaining changed files are listed with
  `diff_omitted: true`, then not at all (`omitted` counts them);
  `truncated` is true and the text says to call again with fewer `paths`.
  The counts (`changed`, `errors`) always cover every file.

`format_files` works only below the server's working directory. A path that
resolves outside it (absolute, through `..`, or through a symbolic link) makes
the whole call fail with `isError: true` and `structuredContent.outside`
listing those paths; nothing is read or written. Directory walks do not
follow symbolic links.

## Review view (`--review`)

`origami --review` shows what origami would change, for people: for each file
that would change, a header with the path and counts, then each change with
two lines of context, old and new line numbers, removed (`-`) and added (`+`)
lines, and within changed lines only the changed characters highlighted.
Below each construct whose line breaks origami introduced, a note says why
(see "Line-break notes" in [STYLE.md](STYLE.md)):

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

* Exit codes as for `--check`; files are never written. Works on paths, on
  stdin, with `--lines` and with `--changed` (only the kept changes and their
  notes are shown).
* Colors only when stdout is a terminal (`NO_COLOR` and `CLICOLOR_FORCE` as for
  the diagnostics). Without colors, changed characters are marked `[-…-]`
  (removed) and `{+…+}` (added).
* The view shows the text with LF line ends and without a byte-order mark;
  the header says `CRLF` or `BOM` when the file has them.
* Control characters from the source are shown escaped (`\x1b`, `\u{202e}`).
* The view and its notes are for reading: their layout and wording may change
  in any release. Scripts and `git apply` use `--check --diff`, whose output
  stays a plain unified diff. A colored diff of that output is also
  available through a diff viewer: `origami --check --diff | delta`.

## Streams

* **stdout** carries only formatted text (stdin mode), the diff (`--diff`) or
  the review view (`--review`).
* **stderr** carries diagnostics in every format.

## stdin mode

* Success means file status `formatted` or `unchanged`, even when the exit
  code is 5. The formatted text then goes to stdout.
* With `--check`, stdout carries nothing but the diff (`--diff`), and the exit
  code is 1 if the input would change.
* With `--review`, stdout carries the review view instead of the formatted
  text.
* On any error, stdout stays empty. Never write empty stdout back to a source.
* Language: `--language`, else the `--stdin-filepath` name, else content
  sniffing. A name that matches no pattern falls back to sniffing.

## Exit codes

In a multi-file run every file is processed, and the highest code in this order
wins: **70 > 4 > 3 > 2 > 5 > 1 > 0**. Configuration and usage errors (E020,
E021) abort before any file is processed. A usage error found while parsing
the command line is reported in the requested diagnostics format
(`--diagnostics` or `ORIGAMI_DIAGNOSTICS`); with human output it is the
parser's own message. A panic outside a file's formatting (option handling,
reporting, the MCP server) ends the run with exit 70 and one X002 diagnostic
for the path `<internal>`.

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
| W004 | warning | nesting-depth or size limit reached; the affected units, or the whole file, are left as written | 0 / 5 | file | yes |
| W101 | warning | unknown config key | 0 / 5 | run | yes |
| X001 | internal | self-check failed after quarantine | 70 | file | yes |
| X002 | internal | panic caught | 70 | file | yes |

W004 limits (reported once per file):

* **Nesting depth:** a unit (ABAP statement, CDS-family element) with a token
  nested deeper than 100 levels of blocks, brackets and CDS `case` expressions
  combined is printed
  byte for byte. Real code stays far below 100.
* **Input size:** a file larger than 8 MiB is not formatted: status
  `unchanged`, output equal to the input.
* **Output size:** a layout that would make the output more than 16 times the
  input plus 1 MiB leaves the whole file as written.

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
style            = "aligned"    # "aligned" (ABAP Cleaner layout) | "compact" (0.3 layout)
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
