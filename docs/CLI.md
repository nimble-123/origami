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
  --preview             apply style rules that are still in preview
  --diagnostics F       human (default) | json | sarif; always on stderr
  --max-diagnostics N   cap diagnostics per run (default 200)
  --config FILE         use only this config file, no discovery
  --no-config           ignore all config files
  --line-width N        40..=255
  --indent-width N      1..=8
  --keyword-case C      upper | lower (ABAP keywords only)
  -j, --jobs N          worker threads for directory mode (default: CPU count)
  --version [--json] [--config FILE]
```

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
