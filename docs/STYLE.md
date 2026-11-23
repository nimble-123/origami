# origami style

This is the normative style of origami. Every output decision is a rule here,
written and approved before its layout code is merged.

## How rules work

* **Format.** Each rule has a heading `### <ID>: <title>`, followed by a
  `Milestone: M<n>` line, for rules that introduce line breaks an
  `Explanation: <phrase>` line (see "Line-break notes"), a one-paragraph
  statement and a before/after example.
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

## Line-break notes

Approved by the maintainer, Nils Lutz, on 2026-11-22, together with the
`Explanation:` lines of the rules.

`origami --review` and `origami_core::explain` explain each line break that
origami introduces, one note per broken construct and cause. A break that was
already in the input gets no note. A note names its cause:

* **Width:** the construct (or the line that holds its end) would end past
  the line width; the note gives the column it would end at, measured with
  the construct printed on one line.
* **Rule:** the rule always breaks there (for example ABAP-STMT-001).
* **Comment:** an end-of-line comment inside the construct forces the break.

The note starts with the rule ID and the rule's `Explanation:` phrase when the
construct belongs to a rule, for example "ABAP-CALL-001: one parameter per
line, because the call would end at column 134 (limit 120)". Breaks that no
rule covers name the cause only. Rule IDs are never renumbered or reused, so
a note's rule ID always finds its rule here. The wording of notes is not a
stable interface.

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

### GEN-CASE-001: Literals, comments and unrecognized statements keep their case
Milestone: M0

Only tokens that a parser recognized as keywords (ABAP-CASE-001) or as ABAP
identifiers (ABAP-CASE-002) may change case. Literals, comments and every word
of an unrecognized statement keep their case.

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

### ABAP-CASE-001: Keywords upper case
Milestone: M1

Tokens that a statement parser recognized as keywords are printed in the
configured case (default upper). This includes constructor operators (`VALUE`,
`NEW`, `CONV`, `COND`, `SWITCH`, `REDUCE`, `CORRESPONDING`, `REF`, `CAST`,
`EXACT`, `FILTER`) and ABAP SQL keywords. Built-in functions (`lines( )`,
`xsdbool( )`), built-in types (`i`, `string`), `me` and `abap_true` are
identifiers (ABAP-CASE-002); literals and comments keep their case.

Approved by the maintainer, Nils Lutz, on 2026-11-23: a keyword is
upper-cased wherever it stands in a recognized statement, as ABAP Cleaner
does. This includes the words of RAP derived types (`TYPE TABLE FOR READ
RESULT`, `TYPE RESPONSE FOR MAPPED LATE`), inline `FIELD-SYMBOL( )`,
`OTHERS` in `WHEN OTHERS` and `EXCEPTIONS`, logical expressions inside
function arguments and constructor bodies, table keys (`[ KEY k COMPONENTS …
]`), format options of string templates (`{ d DATE = ISO }`), text symbols
with a numeric ID (`TEXT-001`), and in ABAP SQL the aggregate functions
(`COUNT( )`, `SUM( )`, `MIN( )`, `MAX( )`, `AVG( )`) and `CAST( … AS CHAR( n )
)`. Other SQL functions (`length( )`, `substring( )`) are built-in functions
and keep their case.

```abap
before: data(lv_count) = lines( mt_items ).
after:  DATA(lv_count) = lines( mt_items ).
before: loop at mt_items assigning field-symbol(<ls_item>) where status is initial.
after:  LOOP AT mt_items ASSIGNING FIELD-SYMBOL(<ls_item>) WHERE status IS INITIAL.
```

### ABAP-CASE-002: Identifiers in lower case
Milestone: M1

Approved by the maintainer, Nils Lutz, on 2026-11-23, as ABAP Cleaner does
it. In a recognized statement, every word that is not a keyword and has no
lower-case letter is printed in lower case: variables, types, classes,
methods, `sy-datum`, `me`, built-in types and functions. A word with a
lower-case letter keeps its case (`lv_a`, `lvMixed`, `/DMO/I_Travel`).

Never changed: statements without grammar (W003), units left as written
(W002), embedded languages, literals, comments, pragmas and the CDS family
(CDS-CASE-001, where alias case is the OData property name). Operators that
cannot be names (`AND`, `OR`, `NOT`, `EQ`, `NE`, `BETWEEN`, `DIV`, `MOD`, …)
and constructor operators before their type (`VALUE #(`, `CONV i(`) are never
lower-cased, even where a parser did not mark them as keywords.

```abap
before: DATA LV_COUNT TYPE I.
        LV_COUNT = LINES( MT_ITEMS ) + lvOffset.
after:  DATA lv_count TYPE i.
        lv_count = lines( mt_items ) + lvOffset.
```

### ABAP-STMT-001: One statement per line
Milestone: M1
Explanation: each statement on its own line

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
Explanation: one parameter per line

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
Explanation: each parameter group on its own line

When a broken call uses `EXPORTING`, `IMPORTING`, `CHANGING`, `RECEIVING` or
`EXCEPTIONS`, the first keyword stays behind `(`, further keywords are aligned
under it, and each keyword's parameters hang behind the keyword, aligned under
its first parameter. Keywords are never added or removed. The fallback of
ABAP-CALL-001 applies.

Approved by the maintainer, Nils Lutz, on 2026-11-23: a call with two or more
parameter keywords is always broken this way, even when it would fit on one
line (as ABAP Cleaner does). A call with one keyword stays on one line if it
fits.

```abap
lo_service->read( EXPORTING iv_id   = lv_id
                            iv_lang = sy-langu
                  IMPORTING es_data = ls_data ).
lo_service->read( EXPORTING iv_id = lv_id
                  IMPORTING es_data = ls_data ).
lo_service->check( EXPORTING iv_id = lv_id ).
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
Explanation: one row per line

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
Explanation: one declaration per line

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

### ABAP-CHAIN-001: Chains of other statements
Milestone: M1
Explanation: one item per line

Approved by the maintainer, Nils Lutz, on 2026-11-21. A chain of any other keyword
statement (`CLEAR:`, `INTERFACES:`, `EVENTS:`, `SET HANDLER:`, …) follows the
declaration rule: on one line if it fits, otherwise one item per line,
aligned after the colon.

```abap
CLEAR: lv_count,
       lt_items,
       ls_header.
```

### ABAP-ALIGN-003: Align `TYPE` in broken declaration chains (A3)
Milestone: M1

In a broken `DATA:`, `TYPES:`, `CONSTANTS:` or `CLASS-DATA:` chain, the
`TYPE`/`LIKE` keyword of consecutive items is aligned (see the example in
ABAP-DECL-001). A blank line or comment line ends the group; nested
`BEGIN OF` levels align separately.

### ABAP-EVENT-001: Event blocks
Milestone: M4

Approved by the maintainer, Nils Lutz, on 2026-11-21. The statements of an event block
(`LOAD-OF-PROGRAM`, `INITIALIZATION`, `AT SELECTION-SCREEN …`,
`START-OF-SELECTION`, `END-OF-SELECTION`, `TOP-OF-PAGE`, `END-OF-PAGE`,
`AT LINE-SELECTION`, `AT USER-COMMAND`, `AT PFnn`) are indented one level,
as in the ADT pretty printer. An event block ends at the next event keyword or
processing block (`FORM`, `MODULE`, `FUNCTION`, `CLASS`, `INTERFACE`).

```abap
START-OF-SELECTION.
  PERFORM load_data.
  PERFORM display.

FORM load_data.
  …
ENDFORM.
```

### ABAP-DECL-002: Structures across statements
Milestone: M4

Approved by the maintainer, Nils Lutz, on 2026-11-21. When `BEGIN OF` and `END OF` are in
different statements (`DATA: BEGIN OF s.` … `DATA: END OF s.`), the
statements between them are indented one level.

```abap
DATA: BEGIN OF status_view.
  INCLUDE STRUCTURE vimstatus.
DATA: END OF status_view.
```

### ABAP-FORM-001: FORM signatures
Milestone: M4
Explanation: each section and parameter on its own line

Approved by the maintainer, Nils Lutz, on 2026-11-21. A `FORM` signature follows
ABAP-METHODS-001 with the sections `TABLES`, `USING`, `CHANGING` and
`RAISING`: on one line if it fits, otherwise each section on its own line at
the continuation indent and each parameter one level deeper.

```abap
FORM read_flights
    TABLES
      et_flights STRUCTURE sflight
    USING
      iv_carrier TYPE s_carr_id
      iv_date TYPE s_date.
```

Other classic statements (`REPORT`, `PARAMETERS`, `SELECT-OPTIONS`,
`SELECTION-SCREEN`, `PERFORM`, `WRITE`, `FUNCTION`, `RAISE`, list and dataset
statements, …) follow the general keyword-statement and chain rules.
Dynpro flow logic (`*.screen_<nr>.abap`) follows the same rules: a
`PROCESS BEFORE OUTPUT` / `AFTER INPUT` / `ON HELP-REQUEST` /
`ON VALUE-REQUEST` block is indented like an event block (ABAP-EVENT-001),
`CHAIN … ENDCHAIN` and `LOOP … ENDLOOP` like other blocks, and `MODULE x.`
is a call.

### ABAP-METHODS-001: Method signatures
Milestone: M1
Explanation: each section and parameter on its own line

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
Explanation: condition split before AND, OR

A condition that does not fit breaks before `AND` and `OR` at the outermost
level, at the continuation indent. Parenthesized sub-conditions stay flat when
they fit.

### ABAP-SQL-001: ABAP SQL
Milestone: M1
Explanation: each clause on its own line

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

Approved by the maintainer, Nils Lutz, on 2026-11-22:
`INTO CORRESPONDING FIELDS OF` and `APPENDING CORRESPONDING FIELDS OF` are one
clause. `INSERT`, `UPDATE`, `MODIFY` and `DELETE` count as database statements
when their form exists only for database tables: every `UPDATE`,
`INSERT INTO …`, `INSERT dbtab FROM …`, `DELETE FROM …`,
`MODIFY dbtab FROM TABLE …`, or any statement with a host variable escaped by
`@`. `DELETE FROM` and `INSERT INTO` keep the table on the first line.
Approved by the maintainer, Nils Lutz, on 2026-11-22: in the strict syntax,
where `FROM` comes before the field list (`SELECT [SINGLE|DISTINCT] FROM dbtab
… FIELDS …`), the table stays on the first line too, and `FIELDS` starts the
next clause line. Other
forms may work on internal tables and keep the layout of keyword statements
(a line breaks only before a clause that does not fit).


### ABAP-SQL-002: Common table expressions and subqueries
Milestone: M4
Explanation: each common table expression on its own line

Approved by the maintainer, Nils Lutz, on 2026-11-21. In `WITH +cte AS ( SELECT … ) …`
each common table expression starts its own line, aligned after `WITH`, and
the main query starts its own line at the statement's indent. A subquery in
parentheses stays on one line if it fits; otherwise the query starts on the
line after `(`, one level deeper, follows ABAP-SQL-001, and `)` gets its own
line. `OPEN CURSOR … FOR SELECT …` and `MERGE INTO …` (8.16) follow
ABAP-SQL-001.

```abap
WITH +carriers AS ( SELECT carrid, carrname FROM scarr ),
     +seats AS (
       SELECT carrid, connid, SUM( seatsocc ) AS seats
           FROM sflight
           GROUP BY carrid, connid
     )
SELECT * FROM +seats INTO TABLE @DATA(result).
```
### ABAP-COMMENT-002: End-of-line comments
Milestone: M1

An end-of-line `"` comment is separated from the code before it by exactly one
space. Hand-aligned comment columns are not kept.

```abap
before: lv_a = 1.          " first
after:  lv_a = 1. " first
```

### ABAP-CALL-003: CALL FUNCTION and CALL METHOD
Milestone: M1
Explanation: each parameter group on its own line

Approved by the maintainer, Nils Lutz, on 2026-11-21. A `CALL FUNCTION` or
`CALL METHOD` statement is printed on one line when it fits. Otherwise each
parameter group (`EXPORTING`, `IMPORTING`, `CHANGING`, `TABLES`, `EXCEPTIONS`,
`RECEIVING`, `PARAMETER-TABLE`, `EXCEPTION-TABLE`) starts a line at the
continuation indent, and its parameters hang behind the keyword with aligned
`=` (as ABAP-ALIGN-001). Header additions (`DESTINATION`, `STARTING NEW TASK`,
`IN UPDATE TASK`, …) stay on the first line.

```abap
CALL FUNCTION 'Z_FLIGHT_TRAVEL_CREATE'
    EXPORTING is_travel  = ls_travel
              it_booking = lt_booking
    IMPORTING es_travel   = ls_result
              et_messages = lt_messages
    EXCEPTIONS not_found = 1
               OTHERS    = 2.
```

### ABAP-EML-001: Entity manipulation language
Milestone: M1
Explanation: each entity and operation on its own line

Approved by the maintainer, Nils Lutz, on 2026-11-21. `READ ENTITIES`, `READ ENTITY`,
`MODIFY ENTITIES` and `MODIFY ENTITY` statements are printed on one line when
they fit. Otherwise:

* each `ENTITY` and each response (`FAILED`, `REPORTED`, `MAPPED`) starts a
  line at the continuation indent;
* each operation (`CREATE`, `UPDATE`, `DELETE`, `EXECUTE`, `FIELDS`,
  `ALL FIELDS`, `BY \_assoc`, `FROM`) and its `RESULT` and `LINK` start a line
  one level deeper;
* `WITH …` and `FROM …` stay behind their operation and move to the next line
  only when they do not fit.

`COMMIT ENTITIES`, `GET PERMISSIONS` and `SET LOCKS` break before their
clauses when they do not fit.

```abap
READ ENTITIES OF /DMO/I_Travel_M IN LOCAL MODE
    ENTITY travel
      FIELDS ( customer_id ) WITH CORRESPONDING #( keys )
      RESULT DATA(travels)
    FAILED DATA(failed).
```

## CDS family (M2/M3 batch 1, structural)

Approved by the maintainer, Nils Lutz, on 2026-11-21, including single
spaces between CDS tokens (CDS-SPACE-001), indented continuation lines in BDEF
headers (CDS-CONT-001) and one-line RAP handler `METHODS` when they fit
(ABAP-METHODS-001). These rules apply to DDLS, DDLX, DCL, BDEF and SRVD. The
layout is structural: it knows units, blocks and `key` elements, but not yet
the full grammar of headers and expressions.

### CDS-UNIT-001: One unit per line
Milestone: M2
Explanation: each annotation and element on its own line

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

Runs of spaces between tokens on one line shrink to one space. Hand-aligned
columns, for example annotation values or `as` aliases, are not kept.

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

### CDS-ANNO-001: Annotation values
Milestone: M2
Explanation: annotation values on their own lines

Approved by the maintainer, Nils Lutz, on 2026-11-21. An annotation value is printed on
one line when it fits. Otherwise each object `{ … }` or array `[ … ]` that does
not fit puts every item on its own line, one level deeper, and its closing
bracket on its own line, as in JSON. Items that fit stay on one line. Where a
bracket is glued to its content in the input (`[{`, `}]`), no line break is
added between them.

```cds
@UI.facet: [
  { id: 'General', purpose: #STANDARD, type: #COLLECTION, position: 10 },
  {
    id: 'Details',
    targetQualifier: 'Details_FG',
    parentId: 'General',
    purpose: #STANDARD,
    type: #FIELDGROUP_REFERENCE,
    label: 'Details',
    position: 20
  }
]
```

### CDS-HEAD-001: Definition headers
Milestone: M2
Explanation: each clause of the header on its own line

Approved by the maintainer, Nils Lutz, on 2026-11-21. In a DDLS header, `as select from`,
`as projection on`, `provider contract`, `with parameters`, each `association`,
each `composition` and each join start a line one level deeper than `define`;
view parameters go one level deeper still. A clause that does not fit breaks
before `on`, `and` and `or`, two levels deeper. In a BDEF `define behavior for` header, each clause
(`implementation in class`, `persistent table`, `draft table`, `lock`,
`authorization`, `etag`, `total etag`, `early numbering`, `late numbering`,
`with …`) starts a line one level deeper.

```cds
define root view entity ZI_Travel
  as select from ztravel as Travel
  composition [0..*] of ZI_Booking as _Booking
  association [0..1] to ZI_Agency as _Agency on $projection.AgencyID = _Agency.AgencyID
{
```

### CDS-ELEM-001: Case expressions in select lists
Milestone: M2
Explanation: each WHEN on its own line

Approved by the maintainer, Nils Lutz, on 2026-11-21. A `case` expression with two or
more `when` always breaks: each `when` and `else` starts a line one level
deeper than its `case`, and `end` starts a line at the level of its `case`.
Nested cases go one level deeper each. A `case` with one `when` stays on one
line when it fits. Elements without `case` keep the author's line breaks.

```cds
      case currency
        when 'EUR' then 'X'
        when 'USD' then 'Y'
        else 'Z'
      end as CurrencyGroup,
```

### DCL-GRANT-001: Access conditions
Milestone: M2
Explanation: condition split before AND, OR

Approved by the maintainer, Nils Lutz, on 2026-11-21. `grant select on <entity> [where …];`
is printed on one line when it fits, instead of one word per line as in the
ADT template. Otherwise `where` starts a line one level deeper, and a long
condition breaks before `and` / `or` two levels deeper.

```cds
define role ZI_Travel
{
  grant select on ZI_Travel
    where ( AgencyID ) = aspect pfcg_auth( ZAGENCY, ZAGENCYID, ACTVT = '03' )
      and CurrencyCode = 'EUR';
}
```

