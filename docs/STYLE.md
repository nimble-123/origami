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
before: Data lv_Value type i.
after:  DATA lv_Value TYPE i.
```

### GEN-COMMENT-001: Full-line star comments stay in column 1
Milestone: M0

A `*` comment is printed in column 1 and unchanged.

```abap
before: * This comment stays here
          DATA x TYPE i.
after:  * This comment stays here
        DATA x TYPE i.
```

## ABAP (M1 batch 1)

Approved by the maintainer, Nils Lutz, on 2026-11-21. Derived from
`cargo xtask freq` on the corpus: the statement kinds below cover about 90% of
all ABAP statements. Four choices were decided explicitly: hanging call
parameters (ABAP Cleaner style), a two-level continuation indent, `WHEN` and
`CATCH` indented as in ADT, and one space before end-of-line comments.

### ABAP-CASE-001: Keywords upper case, everything else as written
Milestone: M1

Tokens that a statement parser recognized as keywords are printed in the
configured case (default upper). This includes constructor operators (`VALUE`,
`NEW`, `CONV`, `COND`, `SWITCH`, `REDUCE`, `CORRESPONDING`, `REF`, `CAST`,
`EXACT`, `FILTER`) and ABAP SQL keywords. Identifiers, built-in functions
(`lines( )`, `xsdbool( )`), built-in types (`i`, `string`), `me`, `abap_true`,
literals and comments keep their case.

```abap
before: data(lv_count) = lines( mt_items ).
after:  DATA(lv_count) = lines( mt_items ).
```

### ABAP-STMT-001: One statement per line
Milestone: M1

Statements that share a line are split onto separate lines. A chained
statement (`DATA: a …, b ….`) is one statement.

```abap
before: lv_a = 1. lv_b = 2.
after:  lv_a = 1.
        lv_b = 2.
```

### ABAP-BLOCK-001: Block indentation
Milestone: M1

Each block level indents by `indent_width` (default 2). `ELSEIF` and `ELSE` are
printed at the level of their `IF`. `WHEN` (in `CASE`) and `CATCH`/`CLEANUP`
(in `TRY`) are indented one level, and their bodies two levels, as in the ADT
pretty printer.

```abap
IF lv_a = 1.
  do_a( ).
ELSE.
  do_b( ).
ENDIF.
CASE lv_kind.
  WHEN 'A'.
    do_a( ).
  WHEN OTHERS.
    do_b( ).
ENDCASE.
TRY.
    do_a( ).
  CATCH cx_root INTO DATA(lx).
    handle( lx ).
ENDTRY.
```

### ABAP-CLASS-001: Class sections and members
Milestone: M1

In a class definition, `PUBLIC SECTION`, `PROTECTED SECTION` and
`PRIVATE SECTION` are indented one level and their members two levels. In a
class implementation, `METHOD` is indented one level and its body two levels.
Interface members are indented one level.

```abap
CLASS zcl_demo DEFINITION PUBLIC FINAL CREATE PUBLIC.
  PUBLIC SECTION.
    METHODS run.
ENDCLASS.

CLASS zcl_demo IMPLEMENTATION.
  METHOD run.
    do_it( ).
  ENDMETHOD.
ENDCLASS.
```

### ABAP-BLANK-001: Blank lines
Milestone: M1

Blank lines between statements are kept as written, at most one in a row.
Blank lines directly after a block opener (`METHOD`, `IF`, `LOOP`, …) and
directly before its closer are removed. Blank lines inside a statement are
removed.

```abap
before: METHOD run.

          do_it( ).


          done( ).

        ENDMETHOD.
after:  METHOD run.
          do_it( ).

          done( ).
        ENDMETHOD.
```

### ABAP-WIDTH-001: Line breaking
Milestone: M1

A statement is printed on one line when it fits into `line_width` (default
120). Otherwise it breaks at its outermost break points first (parameter lists,
constructor expressions, logical operators, SQL clauses), and inner parts stay
flat if they fit. Literals and string templates are never split; a line may
exceed the width when one of them alone does not fit.

### ABAP-INDENT-001: Continuation indent
Milestone: M1

Continuation lines of a statement are indented two levels (default 4 spaces)
relative to the start of the statement, unless a rule aligns them to a column
(ABAP-CALL-001, ABAP-VALUE-001, ABAP-DECL-001). This keeps a wrapped condition
visually apart from the block body, which is indented one level.

```abap
IF lv_amount > 0
    AND lv_currency IS NOT INITIAL.
  book( ).
ENDIF.
```

### ABAP-CALL-001: Method call parameters
Milestone: M1

A call is printed flat when it fits. Otherwise the first parameter stays
behind the opening `(`, every further parameter goes on its own line, aligned
under the first one, and the closing `)` stays at the end of the last
parameter, followed by the period (ABAP Cleaner style).

Fallback: if a parameter does not fit at the hanging column, the line breaks
after `(` and the parameters go on their own lines at the continuation indent.

```abap
DATA(lv_result) = lo_calculator->calculate( iv_amount   = lv_amount
                                            iv_currency = lv_currency
                                            iv_date     = sy-datum ).
```

### ABAP-CALL-002: Parameter keywords
Milestone: M1

When a broken call uses `EXPORTING`, `IMPORTING`, `CHANGING`, `RECEIVING` or
`EXCEPTIONS`, the first keyword stays behind `(`, further keywords are aligned
under it, and each keyword's parameters hang behind the keyword, aligned under
its first parameter. Keywords are never added or removed. The fallback of
ABAP-CALL-001 applies.

```abap
lo_service->read( EXPORTING iv_id   = lv_id
                            iv_lang = sy-langu
                  IMPORTING es_data = ls_data ).
```

### ABAP-ALIGN-001: Align `=` in broken parameter lists (A1)
Milestone: M1

In a broken parameter list, the `=` signs of consecutive parameters are
aligned. A blank line or comment line ends the aligned group. If aligning
would push a line past the width, or a `=` touches the name before it, the
group uses single spaces.

### ABAP-ALIGN-002: Align `=` in broken constructor expressions (A2)
Milestone: M1

Same as ABAP-ALIGN-001, for component assignments in a broken `VALUE`, `NEW`,
`CORRESPONDING` or `CONV` expression.

```abap
rs_flight = VALUE #( carrier_id    = 'LH'
                     connection_id = '0400'
                     flight_date   = sy-datum ).
```

### ABAP-VALUE-001: Constructor expressions
Milestone: M1

A constructor expression is printed flat when it fits. Otherwise it follows
ABAP-CALL-001: the first component assignment or table row stays behind `#(`
(or `type(`), the others are aligned under it, and the closing `)` stays at the
end of the last line, with the same fallback. Rows `( … )` that fit are
printed flat.

```abap
mt_items = VALUE #( ( id = 1 name = `Alpha` )
                    ( id = 2 name = `Beta` ) ).
```

### ABAP-DECL-001: Declarations and chains
Milestone: M1

A declaration (`DATA`, `TYPES`, `CONSTANTS`, `CLASS-DATA`, `STATICS`,
`FIELD-SYMBOLS`) that fits is printed on one line. In a chain that does not
fit, each item goes on its own line, aligned after the colon. Components
between `BEGIN OF` and `END OF` are indented one level more.

```abap
DATA: lv_count TYPE i,
      lt_items TYPE ty_items.
TYPES: BEGIN OF ty_item,
         id   TYPE i,
         name TYPE string,
       END OF ty_item.
```

### ABAP-ALIGN-003: Align `TYPE` in broken declaration chains (A3)
Milestone: M1

In a broken `DATA:`, `TYPES:`, `CONSTANTS:` or `CLASS-DATA:` chain, the
`TYPE`/`LIKE` keyword of consecutive items is aligned (see the example in
ABAP-DECL-001). A blank line or comment line ends the group; nested
`BEGIN OF` levels align separately.

### ABAP-METHODS-001: Method signatures
Milestone: M1

A `METHODS` or `CLASS-METHODS` declaration that fits is printed on one line.
Otherwise each section keyword (`IMPORTING`, `EXPORTING`, `CHANGING`,
`RETURNING`, `RAISING`, `EXCEPTIONS`) goes on its own line at the continuation
indent, and each parameter one level deeper. Parameter types are not aligned.

```abap
METHODS calculate
    IMPORTING
      iv_amount TYPE p
      iv_currency TYPE waers
    RETURNING
      VALUE(rv_result) TYPE p
    RAISING
      zcx_calc_error.
```

### ABAP-COND-001: Logical expressions
Milestone: M1

A condition that does not fit breaks before `AND` and `OR` at the outermost
level, at the continuation indent. Parenthesized sub-conditions stay flat when
they fit.

### ABAP-SQL-001: ABAP SQL
Milestone: M1

A `SELECT` (and `INSERT`, `UPDATE`, `MODIFY`, `DELETE` on database tables)
that fits is printed on one line. Otherwise each clause (`FROM`, `FIELDS`,
`WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`, `INTO`, `APPENDING`, `UP TO`) starts
a new line at the continuation indent.

```abap
SELECT carrid, connid, fldate
    FROM sflight
    WHERE carrid = @lv_carrier
    INTO TABLE @DATA(lt_flights).
```

### ABAP-COMMENT-002: End-of-line comments
Milestone: M1

An end-of-line `"` comment is separated from the code before it by exactly one
space. Hand-aligned comment columns are not kept.

```abap
before: lv_a = 1.          " first
after:  lv_a = 1. " first
```

## CDS family (M2/M3 batch 1, structural)

**Proposed, pending maintainer approval.** These rules apply to DDLS, DDLX,
DCL, BDEF and SRVD. The layout is structural: it knows units, blocks and
`key` elements, but not yet the full grammar of headers and expressions.
One rule is marked **[decision]**.

### CDS-UNIT-001: One unit per line
Milestone: M2

Each annotation, each select-list element (DDLS, ended by `,`), each
statement ended by `;` (DDLX, DCL, BDEF, SRVD), and each block brace `{` or `}`
starts on its own line. A short block that the author wrote on one line stays
on that line.

```cds
before: define service Z {
        expose A as A; expose B as B; }
after:  define service Z
        {
          expose A as A;
          expose B as B;
        }
```

### CDS-BLOCK-001: Block indentation
Milestone: M2

Units are indented one level per enclosing `{ }` block. `{` and `}` are at the
level of the header they belong to.

```cds
define behavior for ZI_Travel alias Travel
  persistent table ztravel
  lock master
{
  create;
  association _Booking { create; }
}
```

### CDS-KEY-001: Non-key elements line up with key element names
Milestone: M2

In a block that contains `key` elements, the other elements and their
annotations are indented two more levels, so that element names line up with
the names after `key `.

```cds
{
  key TravelUUID,
      @Semantics.text: true
      Description,
      _Booking
}
```

### CDS-CONT-001: Line breaks inside a unit
Milestone: M2

Inside a unit (a header, a long annotation value, a `where` condition), the
author's line breaks are kept. Continuation lines move with the unit and never
start left of one indent level.

### CDS-SPACE-001: Single spaces between tokens
Milestone: M2

**[decision]** Runs of spaces between tokens on one line shrink to one space.
Hand-aligned columns, for example annotation values or `as` aliases, are not
kept. Alternative: keep the author's spacing inside CDS units.

```cds
before: id:              'Connection',
after:  id: 'Connection',
```

### CDS-CASE-001: Letter case is never changed
Milestone: M2

CDS-family keywords and names keep their case as written.

### CDS-BLANK-001: Blank lines
Milestone: M2

Blank lines between units are kept, at most one in a row. Blank lines directly
after `{` and directly before `}` are removed.
