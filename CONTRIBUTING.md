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
* **No new knobs.** The style is fixed apart from line width, indentation and
  keyword case. Proposals for options are discussed in an issue first.

## Set up

You need Rust 1.94 or newer (edition 2024). For the VS Code extension, Node 22;
for the website, the `wasm32-unknown-unknown` target.

```sh
cargo build --workspace
cargo test --workspace
cargo test -p origami_core --features fault-injection
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

Before a pull request that touches formatting, also run the real-world checks:

```sh
cargo xtask corpus fetch      # once: clones the public test repositories
cargo xtask corpus run        # every file formats, is idempotent, no regressions
cargo xtask explain --corpus  # line-break notes are consistent
cargo xtask oracle            # abaplint's lexer sees the same tokens
cargo xtask stress 20000 1    # mutated inputs never crash or corrupt
```

The VS Code extension (`editors/vscode`): `npm ci`, `npm test`,
`npm run lint`, `npm run test:e2e`. The website (`site/`):
`sh site/build.sh && python3 -m http.server -d site/dist`.

## Commits and pull requests

Pull requests are squash-merged, and the pull request title becomes the commit
on `main`. It must be a [Conventional Commit](https://www.conventionalcommits.org/en/v1.0.0/);
a check enforces it. [release-please](https://github.com/googleapis/release-please)
turns these commits into the version number and the changelog.

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
diagnostics schema gets a `!` after the type (`feat(cli)!: …`) and a
`BREAKING CHANGE:` paragraph in the description. Within a style edition,
formatted output changes only to fix bugs; every such change is a `style`
commit with its rule ID.

## Releases

release-please keeps a release pull request open. Merging it tags `vX.Y.Z`;
the tag builds the binaries for macOS, Linux and Windows, attaches them and
the VS Code extensions to the GitHub release, publishes the crates and
updates the Homebrew formula in `nimble-123/homebrew-tap`.

## Diagrams and images

Diagrams are SVG files in `docs/assets/` built from `docs/diagrams/` (light
and dark variants) and embedded in Markdown with `<picture>`. Colors, fonts
and the logo follow [docs/brand.md](docs/brand.md).

## License

By contributing, you agree that your contribution is licensed under the
project's terms: MIT or Apache-2.0, at the user's option.
