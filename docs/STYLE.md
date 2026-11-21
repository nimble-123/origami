# origami style

This is the normative style of origami. Every output decision is a rule here,
written and approved before its layout code is merged.

## How rules work

* **Format.** Each rule has a heading `### <ID>: <title>`, followed by a
  `Milestone: M<n>` line, a one-paragraph statement and a before/after example.
  The ID matches `^(GEN|ABAP|CDS|DDLS|DDLX|DCL|BDEF|SRVD)-[A-Z]+-\d{3}$`.
* **Approval gate.** Each rule enters this file through its own pull request.
  The maintainer, Nils Lutz, approves it before any layout pull request that
  cites the rule ID is merged. A layout pull request that cites a rule ID not
  yet on `main` fails review.
* **Tests.** Every rule has a directory `crates/origami_core/tests/rules/<ID>/`
  with `input.<abapGit file name>` and `expected.<abapGit file name>`. The
  harness skips rules whose milestone is later than the current one.
* **Batches.** The first pull request of M1 adds the rules for the ABAP
  statement kinds that `cargo xtask freq` ranks highest, including the
  alignment rules `ABAP-ALIGN-001..003`. M2 and M3 start the same way.
* **Stability.** Within a style edition, output changes only to fix bugs, and
  every such change is listed in `CHANGELOG.md` with its rule ID. New rules
  first appear behind `--preview`.

## General

### GEN-ENCODING-001: Line endings are kept
Milestone: M0

The output uses the line-break style of the input's first line break (LF or
CRLF). Mixed line endings become uniform in that style.

```text
before: "DATA x.\r\nDATA y.\n"
after:  "DATA x.\r\nDATA y.\r\n"
```

### GEN-ENCODING-002: A byte-order mark is kept
Milestone: M0

If the input starts with a UTF-8 byte-order mark, so does the output. origami
never adds one.

```text
before: "\xEF\xBB\xBFDATA x.\n"
after:  "\xEF\xBB\xBFDATA x.\n"
```

### GEN-ENCODING-003: The final newline state is kept
Milestone: M0

A file that ends with a line break keeps it; a file without one stays without.

```text
before: "DATA x."
after:  "DATA x."
```

### GEN-SAFETY-001: Embedded languages are copied byte for byte
Milestone: M0

The bodies of AMDP methods (`BY DATABASE PROCEDURE|FUNCTION|GRAPH WORKSPACE`),
`EXEC SQL … ENDEXEC` and `DEFINE … END-OF-DEFINITION` are never changed, not
even re-indented.

```abap
METHOD get BY DATABASE PROCEDURE FOR HDB LANGUAGE SQLSCRIPT.
    lt = select   *   from "/DMO/FLIGHT";
ENDMETHOD.
```

### GEN-CASE-001: Identifiers are never re-cased
Milestone: M0

Only tokens that a parser recognized as keywords may change case. Identifiers,
literals, comments and every word of an unrecognized statement keep their case.

```abap
before: Data lv_Value TYPE i.
after:  Data lv_Value TYPE i.    " M0: no statement is recognized yet
```

### GEN-COMMENT-001: Full-line star comments stay in column 1
Milestone: M0

A `*` comment is printed in column 1 and unchanged.

```abap
* This comment stays here
  DATA x TYPE i.
```
