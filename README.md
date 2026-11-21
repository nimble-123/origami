# origami

A layout-only formatter for ABAP and the CDS family: ABAP, CDS data definitions
(DDLS), metadata extensions (DDLX), access controls (DCL), RAP behavior
definitions (BDEF) and service definitions (SRVD). It runs offline, in
milliseconds, on abapGit repositories and on ADT for VS Code documents.

origami changes layout only: whitespace, line breaks, indentation and the case
of ABAP keywords. It never adds or removes a gap between two tokens, never
re-cases identifiers and never rewrites statements. After every run it re-lexes
its own output and refuses to write if anything but layout changed.

> **Status: M1 complete for ABAP, structural layout for the CDS family.**
> All six languages are formatted. ABAP uses statement parsers (assignments,
> calls, constructor expressions, declarations, method signatures, control
> statements, ABAP SQL `SELECT`, …); statements without a parser keep their
> own line breaks and are re-indented (W003). The CDS family (DDLS, DDLX,
> DCL, BDEF, SRVD) uses a structural layout: one annotation or element per
> line, block indentation, `key` alignment. On the test corpus (1,835 files)
> every file passes the self-check and is idempotent, and the independent
> abaplint lexer finds no token changes. See [the design](docs/designs/origami.md).

## Install

```sh
cargo install --path crates/origami_cli   # binary: origami
```

Released binaries, `cargo install abap-origami` and the VS Code extension
follow with v1.0.

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
| `*.clas.abap` (incl. `locals_def`, `locals_imp`, `testclasses`, `macros`), `*.intf.abap` | ABAP | M1 |
| `*.prog.abap`, `*.fugr.*.abap`, `*.type.abap` | ABAP | fallback until the M4 decision |
| `*.ddls.asddls`, `*.drty.acds` | CDS DDL | M2 |
| `*.ddlx.asddlxs` | DDLX | M2 |
| `*.dcls.asdcls` | DCL | M2 |
| `*.bdef.asbdef` | BDEF | M3 |
| `*.srvd.srvdsrv` | SRVD | M3 |

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
