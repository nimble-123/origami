# origami

A layout-only formatter for ABAP and the CDS family: ABAP, CDS data definitions
(DDLS), metadata extensions (DDLX), access controls (DCL), RAP behavior
definitions (BDEF) and service definitions (SRVD). It runs offline, in
milliseconds, on abapGit repositories and on ADT for VS Code documents.

origami changes layout only: whitespace, line breaks, indentation and the case
of ABAP keywords. It never adds or removes a gap between two tokens, never
re-cases identifiers and never rewrites statements. After every run it re-lexes
its own output and refuses to write if anything but layout changed.

> **Status: feature-complete for v1.0, not yet released.** All six
> languages are formatted by grammar: ABAP (modern and classic: classes,
> interfaces, RAP handlers, EML, ABAP SQL, reports, selection screens, event
> blocks, `FORM` routines, function modules) and the CDS family (annotations,
> view headers, `case` expressions, DCL grants, BDEF and SRVD). Statements
> without a parser keep their own line breaks and are re-indented (W003); on
> the test corpus that is 0.2% of ABAP statements. Every corpus file (1,891
> ABAP and 227 CDS-family files) passes the self-check and is idempotent, and
> the independent abaplint lexer finds no token changes. Formatting is about
> 17× faster than ABAP Cleaner ([comparison](docs/benchmark-abap-cleaner.md)).
> See [the design](docs/designs/origami.md) and [the style guide](docs/STYLE.md).

## Install

```sh
cargo install --path crates/origami_cli   # binary: origami
```

VS Code: build the extension in `editors/vscode` (see below) and install the
VSIX. Released binaries, `cargo install abap-origami` and Marketplace
publishing follow with v1.0.

```sh
cd editors/vscode
npm ci && npm run build
mkdir -p bin && cp ../../target/release/origami bin/
npx vsce package --target darwin-arm64 --no-dependencies --allow-missing-repository
code --install-extension abap-origami-darwin-arm64-0.1.0.vsix
```

## Use

```sh
origami                       # format all supported files below .
origami --check               # CI: exit 1 if anything would change
origami --check --diff        # show what would change
cat zcl_foo.clas.abap | origami --stdin-filepath zcl_foo.clas.abap
origami --check --diagnostics json src/   # machine-readable, on stderr
```

The full contract (flags, exit codes, diagnostic codes, JSON schema, config) is
in [docs/CLI.md](docs/CLI.md).

### pre-commit

```yaml
repos:
  - repo: <origami repository URL>
    rev: <tag>
    hooks:
      - id: origami        # check only
      # - id: origami-fix  # rewrite files
```

### AI agents

Pipe source through stdin. On any error stdout stays empty, so an agent never
writes an empty object back. `"changed": false` in the JSON diagnostics lets
it skip a write-back through the ADT MCP server.

## Supported files

| abapGit file | Language | Status |
|---|---|---|
| `*.clas.abap` (incl. `locals_def`, `locals_imp`, `testclasses`, `macros`), `*.intf.abap` | ABAP | formatted |
| `*.prog.abap`, `*.fugr.*.abap`, `*.type.abap` | ABAP | formatted, including reports, selection screens, event blocks, `FORM` routines and function modules |
| `*.screen_<nr>.abap` | dynpro flow logic | not formatted (skipped) |
| `*.ddls.asddls`, `*.drty.acds` | CDS DDL | formatted |
| `*.ddlx.asddlxs` | DDLX | formatted |
| `*.dcls.asdcls` | DCL | formatted |
| `*.bdef.asbdef` | BDEF | formatted |
| `*.srvd.srvdsrv` | SRVD | formatted |

## Configuration

`origami.toml`, found by walking up to the git root:

```toml
required_version = ">=0.1"
line_width = 120
indent_width = 2

[abap]
keyword_case = "upper"
```

## Documentation

* [Style](docs/STYLE.md): every output rule, with approval gate
* [Invariants](docs/INVARIANTS.md): what "layout only" means and how it is checked
* [CLI contract](docs/CLI.md)
* [Adopting origami in a team](docs/adoption.md)
* [Design](docs/designs/origami.md)

## Development

```sh
cargo test --workspace
cargo test -p origami_core --features fault-injection
cargo xtask corpus fetch && cargo xtask corpus run   # real-world corpus
cargo xtask freq                                     # statement frequencies
```

## License

MIT OR Apache-2.0.
