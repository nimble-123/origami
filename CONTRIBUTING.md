# Contributing to origami

Thank you for helping. The most useful contribution is a piece of real ABAP or
CDS that origami formats in a way you would not, or leaves alone when it
should not: [open an issue](https://github.com/nimble-123/origami/issues/new)
with the input, what origami printed (`origami --review` shows it with the
reasons), and what you expected. Strip anything confidential; a few lines that
show the problem are enough.

## Ground rules

* **Layout only.** origami changes whitespace, line breaks, indentation and
  the case of ABAP keywords, nothing else. Every change must keep the token
  stream identical; the self-check enforces it at run time and the corpus
  tests in CI. See [docs/INVARIANTS.md](docs/INVARIANTS.md).
* **Every output decision is a rule.** A change to the formatted output needs
  a rule in [docs/STYLE.md](docs/STYLE.md) (or a change to one) with an
  example and a test directory in `crates/origami_core/tests/rules/<ID>/`.
  New rules and rule changes are approved by the maintainer before the layout
  code is merged ("How rules work" in STYLE.md).
* **No new knobs.** The style is fixed apart from line width, indentation,
  keyword case and the choice between the two styles `aligned` and `compact`
  (`style`, see "Styles" in STYLE.md). Proposals for options are discussed in
  an issue first.

## Set up

You need Rust 1.100 or newer (edition 2024). For the VS Code extension, Node 22;
for the website, the `wasm32-unknown-unknown` target.

```sh
cargo build --workspace
cargo test --workspace
cargo test -p origami_core --features fault-injection
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

Before a pull request, run the whole chain with one command. It runs the steps
below in order and stops at the first failure:

```sh
cargo xtask ci           # fmt, clippy, test, corpus, explain, oracle, stress
cargo xtask ci --full    # the same, plus the idempotency sweep (minutes)
```

The corpus steps need `cargo xtask corpus fetch` once; the oracle step is
skipped, with a note, until you run `npm ci` in `tools/abaplint-oracle`.

The individual real-world checks, for a pull request that touches formatting:

```sh
cargo xtask corpus fetch      # once: clones the public test repositories
cargo xtask corpus run        # every file formats, is idempotent, no regressions
cargo xtask explain --corpus  # line-break notes are consistent
cargo xtask oracle            # abaplint's lexer sees the same tokens
cargo xtask stress 20000 1    # mutated inputs never crash or corrupt
cargo xtask idempotency       # idempotent at every width, indent and case (minutes)
```

### Idempotency sweep

`cargo xtask idempotency` formats every corpus file at 80 settings (8 line
widths, 5 indent widths, 2 keyword cases) and checks that formatting the
output again changes nothing, that `explain` agrees with `format`, and that
formatting a few lines of an already formatted file changes nothing. The
corpus run checks the default setting only, and a second pass tends to differ
at widths near the length of a construct. The nightly fuzz workflow runs the
sweep too. A failure names the file and the setting; reproduce it with
`--line-width` and `--indent-width` and fix the layout code, not the input.

### Fuzzing

The fuzz targets feed arbitrary bytes to the formatter and check that it never
crashes, never reports X001 or X002, and that its output formats to itself. You
need a nightly toolchain and `cargo install cargo-fuzz`:

```sh
cargo +nightly fuzz run format_abap     # or format_cds
```

A finding is saved under `fuzz/artifacts/`. Copy it to `fuzz/found/` with a
name that says what it is (`abap-crlf.bin`) and open a pull request with the
fix. `crates/origami_core/tests/fuzz_regressions.rs` runs every file in
`fuzz/found/` as an ordinary test, so a fixed finding stays fixed. Prefix the
name with `open-` for a finding that is known and not yet fixed; the test
skips those. The nightly workflow runs both targets and opens an issue on a
failure.

The VS Code extension (`editors/vscode`): `npm ci`, `npm test`,
`npm run lint`, `npm run test:e2e`. The website (`site/`):
`sh site/build.sh && python3 -m http.server -d site/dist`.

## Commits and pull requests

Pull requests are squash-merged, and the pull request title becomes the commit
on `main`. It must be a [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/);
a check enforces it. [release-please](https://github.com/googleapis/release-please)
turns these commits into the version number and the changelog.

The commit on `main` is the title only; the description stays on the pull
request. The repository is set up that way (squash commit message: blank)
because release-please drops a commit whose body it cannot parse, and a line
of a diff block such as `+lo_log->add( iv_text = lv_text` is enough for
that: 0.4.1 and 0.4.2 each lost a changelog entry this way.

A pull request that changes only documentation skips the jobs that build and
test the code (`changes` in `.github/workflows/ci.yml` decides). Documentation
is `site/`, `docs/` except `docs/STYLE.md` (tests read it), Markdown files
outside the code folders, and `scripts/abap-cleaner-compare/`.

```text
<type>(<optional scope>): <what changes, in the imperative>
```

| Type | Use for | In the changelog |
|---|---|---|
| `feat` | a new capability (flag, language construct, editor command) | Features |
| `fix` | a bug fix that does not change formatted output | Fixes |
| `style` | a change to formatted output; name the STYLE rule ID | Formatting output |
| `perf` | faster, same output | Performance |
| `docs`, `refactor`, `test`, `build`, `ci`, `chore` | everything else | hidden |

Scopes: `abap`, `cds`, `core`, `layout`, `cli`, `mcp`, `vscode`, `wasm`,
`site`, `xtask`, `release`, `deps` (and `main`, which release-please uses for
the release pull request).

Examples:

```text
feat(cli): add --review with notes on introduced line breaks
style(abap): break database DML before every clause (ABAP-SQL-001)
fix(vscode): keep edits made after Preview Changes was opened
```

A change that breaks configuration, the command line, exit codes or the JSON
diagnostics schema gets a `!` after the type (`feat(cli)!: …`); say what
breaks in the title, because only the title reaches the changelog. Within a
style edition,
formatted output changes only to fix bugs; every such change is a `style`
commit with its rule ID.

## Releases

release-please keeps a release pull request open. Merging it tags `vX.Y.Z`;
the tag builds the binaries for macOS, Linux and Windows, attaches them and
the VS Code extensions to the GitHub release, publishes the crates and
updates the Homebrew formula in `nimble-123/homebrew-tap`.

Secrets, accounts and the release checklist: [docs/releasing.md](docs/releasing.md).

## Diagrams and images

Diagrams are SVG files in `docs/assets/` built from `docs/diagrams/` (light
and dark variants) and embedded in Markdown with `<picture>`. Colors, fonts
and the logo follow [docs/brand.md](docs/brand.md).

## License

By contributing, you agree that your contribution is licensed under the
project's terms: MIT or Apache-2.0, at the user's option.
