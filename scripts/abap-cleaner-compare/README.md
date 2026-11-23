# origami and ABAP Cleaner: layout comparison

Builds [site/abap-cleaner.html](../../site/abap-cleaner.html), the page that
shows the same code formatted by ABAP Cleaner's layout rules and by origami,
and lists every ABAP Cleaner rule next to origami's rules.

* `examples/`: the inputs, run through both tools as they are.
* `Compare.java`: runs ABAP Cleaner through its Java API (the same way as
  [the benchmark](../abap-cleaner-bench/Bench.java)), not through its
  launcher.
* `rules.tsv`: every ABAP Cleaner rule, run alone over the test corpus; the
  last column counts files where more than whitespace and letter case
  changed.
* `build.py`: runs both tools on the examples and writes the page. The table
  of what origami does for each rule (`MAP`) is written by hand; check it
  when a STYLE rule changes.

## The layout-only profile

ABAP Cleaner's default profile, reduced to:

* all rules in the groups Empty lines, Spaces, Pretty printer, Alignment and
  the CDS groups for spaces, alignment, empty lines, line breaks and
  annotations;
* minus the rules there that change names, comments or code: CamelCase for
  CDS names (3 rules), test classes for CDS views, typo correction in DDL
  comments, and rearranging annotations;
* plus "Move AND/OR etc. from line end to next line start", which only moves
  line breaks;
* with default options, except that "Remove notorious 'do not include other
  source files here!!!' comments" is off.

On the 1,964 ABAP and 144 CDS files of the test corpus (2026-11-23) this
profile changes no code and deletes no comment. The one change of token
order: a closing bracket moved in front of an end-of-line comment or a
pragma.

## Run

```sh
cargo build --release -p abap-origami
ABAP_CLEANER_PLUGINS=~/.vscode/extensions/saposs.abap-cleaner-<version>-<os>/binary/abapcleaner.app/Contents/Eclipse/plugins \
  python3 scripts/abap-cleaner-compare/build.py          # add --scan to redo rules.tsv (about 30 minutes)
```
