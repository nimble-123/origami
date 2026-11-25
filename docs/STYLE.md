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
* **Stability.** Before 1.0, any release may change the formatted output: a
  new rule, a changed rule or a bug fix. 0.1.1 did (ABAP-CASE-001 and
  ABAP-CASE-002). Every such change is listed in `CHANGELOG.md` under
  "Formatting output" with its rule ID, and the style edition (`2026`) stays
  the same. `--preview` is accepted but no rule reads it yet. To keep the
  output of one release, pin it with `required_version` in `origami.toml`
  (see [adoption.md](adoption.md)). 0.4 changes the default style from
  `compact` to `aligned` (see "Styles"); `style = "compact"` keeps the 0.3
  layout. From 1.0, a new or changed rule first
  appears behind `--preview` and becomes the default only with a new style
  edition; within an edition, output changes only to fix bugs.

## Styles

Approved by the maintainer, Nils Lutz, on 2026-11-25. origami has two styles,
chosen with `style` in `origami.toml` (or `--style`):

* **`aligned`** (default from 0.4): follows ABAP Cleaner's layout where the
  two differed. Continuation lines are indented one level, parameter
  keywords and declaration types are aligned in columns, method signatures
  are tables behind the method name, and blank lines are added between
  sections and groups.
* **`compact`**: the output of origami 0.3.

Each rule describes the `aligned` style. Where `compact` differs, a paragraph
starting with **Compact:** gives its form. `style` is the only option that
changes the style; see "No new knobs" in CONTRIBUTING.md. Until all of it is
implemented, `aligned` prints parts of the `compact` layout; new rules wait
at `Milestone: M5`.

## Line-break notes

Approved by the maintainer, Nils Lutz, on 2026-11-22, together with the
`Explanation:` lines of the rules.

`origami --review` and `origami_core::explain` explain each line break that
origami introduces, one note per broken construct and cause. A break that was
already in the input gets no note. A note names its cause:

* **Width:** the construct (or the line that holds its end, or its widest
  line) would end past the line width; the note gives the column it would
  end at, measured with the construct printed on one line.
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

Blank lines between statements are kept as written, at most one in a row
(two between classes and interfaces, ABAP-BLANK-002). Blank lines directly
after a block opener (`METHOD`, `IF`, `LOOP`, …) and
directly before its closer are removed. Blank lines inside a statement are
removed. A blank line between comment lines and the statement they lead is
kept, also directly after a block opener; only blank lines in front of the
comment lines are removed there. ABAP-BLANK-002 adds blank lines.

**Compact:** directly after a block opener, the blank line between comment
lines and their statement is removed too.

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

### ABAP-BLANK-002: Blank lines that are added
Milestone: M4

Approved by the maintainer, Nils Lutz, on 2026-11-25. As ABAP Cleaner does,
one blank line is added where none was written:

* in a class definition, before `PROTECTED SECTION` and `PRIVATE SECTION`;
* in a class definition section or an interface, between two declarations
  with different keywords (`INTERFACES`, `ALIASES`, `TYPES`, `CONSTANTS`,
  `DATA`, `CLASS-DATA`, `METHODS`, `CLASS-METHODS`, `EVENTS`,
  `CLASS-EVENTS`); a `TYPES BEGIN OF … END OF` structure counts as one
  `TYPES` declaration;
* in the same places, before and after a declaration that spans two or more
  lines (for example a method signature table);
* in a `METHOD`, `FORM` or `FUNCTION`, and in a file that starts with
  declarations (an include or a code snippet), between the declarations at
  its start (`DATA`, `TYPES`, `CONSTANTS`, `STATICS`, `FIELD-SYMBOLS`) and
  the first other statement;
* in a class implementation, between `ENDMETHOD` and the next `METHOD`.

Between two classes or interfaces (`ENDCLASS` or `ENDINTERFACE` and the next
`CLASS` or `INTERFACE`, with any comment lines in front of it), there are
exactly two blank lines.

Apart from that, a blank line is never added directly after a block opener or
before a closer, and never more than one in a row.

```abap
CLASS zcl_rates DEFINITION PUBLIC.
  PUBLIC SECTION.
    INTERFACES zif_rates.

    METHODS reset.
    METHODS calculate_all.

    METHODS calculate IMPORTING iv_amount   TYPE p
                                iv_currency TYPE waers.

  PRIVATE SECTION.
    DATA mv_rate TYPE p.
ENDCLASS.


CLASS zcl_rates IMPLEMENTATION.
  METHOD reset.
    DATA lv_count TYPE i.

    CLEAR mv_rate.
  ENDMETHOD.

  METHOD calculate_all.
  ENDMETHOD.
ENDCLASS.
```

**Compact:** no blank line is added.

### ABAP-WIDTH-001: Line breaking
Milestone: M1

A statement is printed on one line when it fits into `line_width` (default
120). Otherwise it breaks at its outermost break points first (parameter lists,
constructor expressions, logical operators, SQL clauses), and inner parts stay
flat if they fit. Literals and string templates are never split; a line may
exceed the width when one of them alone does not fit.

### ABAP-INDENT-001: Continuation indent
Milestone: M1

Continuation lines of a statement are indented one level (default 2 spaces)
relative to the start of the statement, unless a rule aligns them to a column
(ABAP-CALL-001, ABAP-VALUE-001, ABAP-DECL-001), as ABAP Cleaner indents SQL
clauses and `CALL FUNCTION` groups.

```abap
SELECT carrid, connid
  FROM spfli
  INTO TABLE @DATA(lt_routes)
  WHERE carrid = @lv_carrier.
```

**Compact:** two levels (default 4 spaces), which keeps a wrapped condition
visually apart from the block body:

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
under it, and each keyword's parameters hang behind the keyword. Keywords are
padded to the longest one in the call, so all parameter names start in one
column, and the `=` signs of all groups are aligned together (ABAP-ALIGN-001).
Keywords are never added or removed. The fallback of ABAP-CALL-001 applies.

Approved by the maintainer, Nils Lutz, on 2026-11-23: a call with two or more
parameter keywords is always broken this way, even when it would fit on one
line (as ABAP Cleaner does). A call with one keyword stays on one line if it
fits.

```abap
lo_service->read( EXPORTING iv_id   = lv_id
                            iv_lang = sy-langu
                  IMPORTING es_data = ls_data ).
lo_service->read( EXPORTING iv_identifier = lv_id
                  CHANGING  ct_log        = lt_log ).
lo_service->check( EXPORTING iv_id = lv_id ).
```

**Compact:** keywords are not padded, and the `=` signs are aligned per
keyword:

```abap
lo_service->read( EXPORTING iv_identifier = lv_id
                  CHANGING ct_log = lt_log ).
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
`FIELD-SYMBOLS`) that fits is printed on one line. In a chain with two or more
items, each item goes on its own line, aligned after the colon, even when the
chain would fit on one line. Components between `BEGIN OF` and `END OF` are
indented one level more.

**Compact:** a chain that fits stays on one line; only a chain that does not
fit puts each item on its own line.

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
ABAP-DECL-001), and so are `VALUE` and `READ-ONLY` as ABAP-ALIGN-008
describes. A blank line or comment line ends the group; nested `BEGIN OF`
levels align separately.

**Compact:** only `TYPE`/`LIKE` is aligned.

### ABAP-ALIGN-008: Align `TYPE` across declaration statements
Milestone: M4

Approved by the maintainer, Nils Lutz, on 2026-11-25. Consecutive one-line
declarations with the same keyword (`DATA`, `CLASS-DATA`, `CONSTANTS`,
`TYPES`, `STATICS`, `FIELD-SYMBOLS`) at the same block level are aligned in
columns, as ABAP Cleaner does: the `TYPE`/`LIKE` keyword, then `VALUE` and
then `READ-ONLY`. The type part (`TYPE c LENGTH 10`, `TYPE STANDARD TABLE OF
…`) is one column. A `VALUE` or `READ-ONLY` column is made only if at least a
fifth of the declarations in the group have one; its position is set by the
declarations that have it, so a long type without a value does not push it.
Blank lines and comment lines between the declarations do not end the
group; another statement, a chain or a `BEGIN OF` structure does. If
aligning would push a line past the width, the group uses single spaces.

```abap
DATA lv_count   TYPE i           VALUE 1.
DATA lt_flights TYPE STANDARD TABLE OF sflight WITH EMPTY KEY.
" logging
DATA lv_text    TYPE c LENGTH 10 VALUE 'abc'.
DATA lv_rate    TYPE p DECIMALS 2.
CONSTANTS lc_max  TYPE i      VALUE 10.
CONSTANTS lc_name TYPE string VALUE `origami`.
DATA mv_id    TYPE i         READ-ONLY.
DATA mv_count TYPE i VALUE 3 READ-ONLY.
```

**Compact:** not aligned.

### ABAP-ALIGN-004: Align assignments to the same object
Milestone: M1

Approved by the maintainer, Nils Lutz, on 2026-11-23, as ABAP Cleaner's
"Align assignments to the same object" with its default options.
Consecutive assignments whose targets are components of the same object
align their assignment operators. The object is the target's name up to
its last `-`: `ls_struc` for `ls_struc-comp` and `ls_struc-date+6(2)`,
`<ls_fs>` for `<ls_fs>-comp`, `ro_obj` for `ro_obj->mv_attr`,
`ro_obj->ms_s` for `ro_obj->ms_s-comp`; names are compared regardless of
case. Operators of different widths are right-aligned in a column as wide as
the widest (`=`, `?=`, `+=`, `-=`, `*=`, `/=`, `&&=`). Blank lines and comment
lines do not end a group; any other statement does, including an assignment
to a plain variable, to another object, or to a target with a table
expression or call (`lt_tab[ 1 ]-comp`), which are never aligned.

Width: when aligning a group would push one of its statements past the line
width (a statement that fits on one line would no longer fit, or the first
line of a longer statement would end past the width), the whole group keeps
single spaces. Statements that are too wide anyway do not count, and
continuation lines of a broken statement move with it. This also applies to
ABAP-ALIGN-005 to ABAP-ALIGN-007. Groups and the decision depend only on
the statements, blank lines and comments, which formatting keeps, so a
second run aligns the same way.

Deviations from ABAP Cleaner: it aligns regardless of the line width, and
compares object names case-sensitively.

```abap
before: ls_struc-component += 1.
        "comment
        ls_struc-comp2 = 'a'.

        ls_struc-comp4 -= 1.
        lv_other = 2.
        ro_instance->mv_name = iv_name.
        ro_instance->mv_id = iv_id.
after:  ls_struc-component += 1.
        "comment
        ls_struc-comp2      = 'a'.

        ls_struc-comp4     -= 1.
        lv_other = 2.
        ro_instance->mv_name = iv_name.
        ro_instance->mv_id   = iv_id.
```

### ABAP-ALIGN-005: Align `METHODS … FOR TESTING`
Milestone: M1
Explanation: one test method per line

Approved by the maintainer, Nils Lutz, on 2026-11-23, as ABAP Cleaner's
"Align METHODS … FOR TESTING" with its default options. Consecutive
`METHODS name [ABSTRACT] FOR TESTING [RAISING …].` declarations are padded
after the method name, so that the words after it start in one column. A
declaration that fits stays on one line (ABAP-METHODS-001), so
`METHODS name` and `FOR TESTING` on two lines are joined. Blank lines and
comment lines do not end a group; any other statement does, including a
chain. A `METHODS:` chain with two or more test methods always has one item
per line, aligned after the colon, and its test methods are aligned
separately from the statements around it; its other items keep their own
lines and are not aligned. The width rule of ABAP-ALIGN-004 applies.

Deviations from ABAP Cleaner: it aligns `ABSTRACT` and `FOR TESTING` in two
columns (origami aligns the first word after the name), aligns end-of-line
comments of the group (origami keeps one space, ABAP-COMMENT-002), and
ignores the line width. In chains it also breaks a chain with only one test
method, widens the column for the names of the other items, and leaves a
chain unaligned when its first item has parameters or a comment line before
it.

```abap
before: METHODS test_setup
          FOR TESTING.
        METHODS any_test_method_name FOR TESTING RAISING cx_static_check.
        METHODS: first_test_method FOR TESTING, second_test_method FOR TESTING RAISING cx_message.
after:  METHODS test_setup           FOR TESTING.
        METHODS any_test_method_name FOR TESTING RAISING cx_static_check.
        METHODS: first_test_method  FOR TESTING,
                 second_test_method FOR TESTING RAISING cx_message.
```

### ABAP-ALIGN-006: Align `METHODS … REDEFINITION`
Milestone: M1
Explanation: one redefinition per line

Approved by the maintainer, Nils Lutz, on 2026-11-23, as ABAP Cleaner's
"Align METHODS … REDEFINITION" with its default options. Consecutive
`METHODS name [FINAL] REDEFINITION.` declarations are aligned like the test
methods of ABAP-ALIGN-005, with the same group, line and width rules. A
`METHODS:` chain of two or more redefinitions and nothing else always has
one item per line, aligned; other chains follow ABAP-CHAIN-001.

Deviations from ABAP Cleaner: it ignores the line width.

```abap
before: METHODS if_any_interface~medium_method_name REDEFINITION.
        METHODS if_any_interface~short_name
          REDEFINITION.
        METHODS if_other_interface~other_method_name FINAL REDEFINITION.
after:  METHODS if_any_interface~medium_method_name  REDEFINITION.
        METHODS if_any_interface~short_name          REDEFINITION.
        METHODS if_other_interface~other_method_name FINAL REDEFINITION.
```

### ABAP-ALIGN-007: Align `ALIASES … FOR`
Milestone: M1
Explanation: one alias per line

Approved by the maintainer, Nils Lutz, on 2026-11-23, as ABAP Cleaner's
"Align ALIASES … FOR" with its default options. Consecutive
`ALIASES name FOR intf~comp.` statements align `FOR`, with the same group,
line and width rules as ABAP-ALIGN-005. An `ALIASES:` chain of two or more
items always has one item per line, aligned after the colon.

Deviations from ABAP Cleaner: it ignores the line width.

```abap
before: ALIASES medium_method_name
          FOR if_any_interface~medium_method_name.
        ALIASES create FOR if_any_interface~create.
        ALIASES: first FOR if_other~first, second FOR if_other~second.
after:  ALIASES medium_method_name FOR if_any_interface~medium_method_name.
        ALIASES create             FOR if_any_interface~create.
        ALIASES: first  FOR if_other~first,
                 second FOR if_other~second.
```

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

Approved by the maintainer, Nils Lutz, on 2026-11-21. A `FORM` signature has
the sections `TABLES`, `USING`, `CHANGING` and `RAISING`: on one line if it
fits, otherwise each section on its own line at the continuation indent and
each parameter one level deeper. This is the compact form of
ABAP-METHODS-001, in both styles (the indent follows ABAP-INDENT-001).

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

Approved by the maintainer, Nils Lutz, on 2026-11-25. A `METHODS` or
`CLASS-METHODS` declaration with at most one parameter in one section
(`IMPORTING`, `EXPORTING`, `CHANGING`, `RETURNING`, `RAISING`, `EXCEPTIONS`)
is printed on one line if it fits. Any other signature is a table, as in
ABAP Cleaner: the first section keyword stays behind the method name, the
further keywords are aligned under it, and each parameter goes on its own
line. Keywords are padded to the longest one, so all parameter names start in
one column; the `TYPE`/`LIKE` of all parameters is aligned, and so are
`OPTIONAL` and `DEFAULT`. The exceptions after `RAISING` or `EXCEPTIONS` stay
on one line.

If the table does not fit behind the method name, the line breaks after the
name and the keyword column starts at the continuation indent.

```abap
METHODS calculate IMPORTING iv_amount        TYPE p
                            iv_currency      TYPE waers OPTIONAL
                  RETURNING VALUE(rv_result) TYPE p
                  RAISING   zcx_calc_error zcx_rate_error.
METHODS reset IMPORTING iv_hard TYPE abap_bool.
```

**Compact:** a declaration that fits is printed on one line. Otherwise each
section keyword goes on its own line at the continuation indent, and each
parameter one level deeper. Parameter types are not aligned.

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

### ABAP-COND-002: COND and SWITCH expressions
Milestone: M4
Explanation: each WHEN branch on its own line

Approved by the maintainer, Nils Lutz, on 2026-11-23 (ABAP Cleaner
"Align conditional expressions" with its default options). A `COND` or
`SWITCH` body hangs behind `#(` (or `type(`), with the fallback of
ABAP-CALL-001. `LET … IN` and the `SWITCH` operand come first; when the
expression breaks, the `WHEN` lines follow below them, aligned.

* **One `WHEN`:** on one line if it fits, else `WHEN`, `THEN` and `ELSE`
  below each other.
* **Several `WHEN` (tabular):** always one `WHEN … THEN …` per line, even if
  the whole expression would fit on one line; `THEN` and the values are
  aligned, and the `ELSE` value lines up with the `THEN` values.
* **Complex:** if a line of the tabular layout does not fit, `THEN` ends the
  `WHEN` line and each value goes on its own line, one level deeper than
  `WHEN`; `ELSE` gets its own line too.

A condition that breaks follows ABAP-COND-001 at the continuation indent of
its `WHEN`. `THROW` is a value like any other. A body the parser does not
understand keeps its line breaks.

```abap
lv_sign = COND #( WHEN iv_negative = abap_true THEN -1 ELSE 1 ).
lv_currency = COND #( WHEN ls_amounts-betrw <> 0 THEN ls_amounts-waers
                      WHEN ls_amounts-betr2 <> 0 THEN ls_amounts-hwae2
                      ELSE                            gc_default_currency ).
lv_text = SWITCH string( sy-index
                         WHEN 1 THEN 'one'
                         WHEN 2 THEN 'two'
                         ELSE        THROW cx_overflow( ) ).
lv_label = COND #( WHEN io_object->get_sub_object( )->get_number( ) <= if_object_boundaries=>co_maximum_number THEN
                     io_object->get_sub_object( )->get_number( )
                   WHEN io_object->is_empty( ) THEN
                     `empty`
                   ELSE
                     if_object_boundaries=>co_maximum_number ).
```

Deviations from ABAP Cleaner, so that the output depends only on the code
and stays within the line width:

* A `COND` with one `WHEN` that fits is joined onto one line; ABAP Cleaner
  keeps it on one line only if it was written that way.
* The tabular layout is used only if every line fits, the `ELSE` line
  included; ABAP Cleaner sometimes keeps it with a longer `ELSE` line.
* A comment or a nested `COND`/`SWITCH` inside a `WHEN` condition rules out
  the tabular layout (ABAP Cleaner aligns around the comment).
* Values are indented one level (`indent`, default 2 spaces) instead of
  always 2 spaces.

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
       SELECT carrid,
              connid,
              SUM( seatsocc ) AS seats
           FROM sflight
           GROUP BY carrid, connid
     )
SELECT * FROM +seats INTO TABLE @DATA(result).
```

### ABAP-SQL-003: Field lists
Milestone: M4
Explanation: one field per line

Approved by the maintainer, Nils Lutz, on 2026-11-23 (ABAP Cleaner
"Align SELECT lists", without "derive from majority"). This applies to the
field lists after `SELECT` (behind `SINGLE`, `DISTINCT`, …), `FIELDS`,
`GROUP BY` and `ORDER BY`, and to `INTO ( … )`, in `SELECT`, `WITH` and
`OPEN CURSOR` and in subqueries.

* A list with a complex field always has one field per line, aligned under
  the first field, and the `AS` of its aliases aligned. A field is complex
  when it has an alias (`AS`) or, in `ORDER BY`, `ASCENDING`, `DESCENDING`
  or `NULLS …` (not aligned), or when it is more than one name: a function
  or expression (`SUM( x )`, `@( … )`, `CASE … END`, `a + b`) or a column
  with a table alias (`tabalias~col`, `tabalias~*`). Names, literals, host
  variables (`@lv_x`, `@ls_s-comp`) and paths (`\_assoc-field`) are simple.
* A list with simple fields only stays on one line if it fits, otherwise it
  has one field per line, aligned under the first field. The targets of
  `INTO ( … )` are always treated this way.

A list on several lines breaks the statement: every clause starts its own
line (ABAP-SQL-001). Without commas (classic syntax) every operand is a
field. A comment inside a field keeps the single space before `AS`.

```abap
SELECT carrid, connid, fldate FROM sflight INTO TABLE @DATA(lt_flights).
SELECT t1~carrid,
       t1~connid          AS connection,
       SUM( t2~seatsocc ) AS seats
    FROM spfli AS t1
    INNER JOIN sflight AS t2 ON t1~carrid = t2~carrid
    GROUP BY t1~carrid,
             t1~connid
    ORDER BY t1~carrid DESCENDING,
             t1~connid
    INTO TABLE @DATA(lt_seats).
```

Deviations from ABAP Cleaner: a list of simple fields that does not fit
gets one field per line (ABAP Cleaner's "one line" style wraps it at the
line width instead), a `SELECT` written on one line is formatted like
any other (ABAP Cleaner leaves it alone), and a space before a comma inside
`INTO ( … )` stays, as for every comma inside brackets (ABAP-SPACE-001;
ABAP Cleaner removes it).
### ABAP-COMMENT-002: End-of-line comments
Milestone: M1

An end-of-line `"` comment keeps the spaces in front of it as written, at
least one, as ABAP Cleaner does; a comment glued to the code gets one space.
Comment columns that the author aligned stay aligned as long as the code in
front of them keeps its width.

```abap
before: lv_a = 1.          " first
        lv_b = 2." second
after:  lv_a = 1.          " first
        lv_b = 2. " second
```

**Compact:** exactly one space; hand-aligned comment columns are not kept
(`lv_a = 1. " first`).

### ABAP-COMMENT-003: A space after the comment sign
Milestone: M4

Approved by the maintainer, Nils Lutz, on 2026-11-25. A `"` comment whose
text starts directly with a letter or a digit gets one space after the `"`,
as ABAP Cleaner's rule 'Start " comment with space' does. Nothing else in the
comment changes. ABAP Doc (`"!`), pseudo-comments (`"#EC`), `""` and comments
starting with any other character (`"---`, `"*`, `"<`) are left alone. This is
the one rule that changes the text of a comment (INVARIANTS.md, section 3).

```abap
before: "tax is added later
        lv_tax = lv_total * lv_rate. "rounded
after:  " tax is added later
        lv_tax = lv_total * lv_rate. " rounded
```

**Compact:** comments are not changed.

### ABAP-CALL-003: CALL FUNCTION and CALL METHOD
Milestone: M1
Explanation: each parameter group on its own line

Approved by the maintainer, Nils Lutz, on 2026-11-25. In a `CALL FUNCTION`
or `CALL METHOD` statement with parameters, each parameter group
(`EXPORTING`, `IMPORTING`, `CHANGING`, `TABLES`, `EXCEPTIONS`, `RECEIVING`,
`PARAMETER-TABLE`, `EXCEPTION-TABLE`) starts a line at the continuation
indent, even when the statement would fit on one line, as ABAP Cleaner does.
The keywords are padded to the longest one, so all parameter names start in
one column, and the `=` signs of all groups are aligned together
(ABAP-ALIGN-001). Header additions (`DESTINATION`, `STARTING NEW TASK`,
`IN UPDATE TASK`, …) stay on the first line.

```abap
CALL FUNCTION 'Z_FLIGHT_TRAVEL_CREATE'
  EXPORTING  is_travel   = ls_travel
             it_booking  = lt_booking
  IMPORTING  es_travel   = ls_result
             et_messages = lt_messages
  EXCEPTIONS not_found   = 1
             OTHERS      = 2.
CALL FUNCTION 'Z_SHORT'
  EXPORTING iv_a = 1.
```

**Compact:** a statement that fits stays on one line; otherwise the groups
break at the continuation indent, keywords are not padded, and the `=` signs
are aligned per group:

```abap
CALL FUNCTION 'Z_FLIGHT_TRAVEL_CREATE'
    EXPORTING is_travel  = ls_travel
              it_booking = lt_booking
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

### ABAP-SPACE-001: No space before a period or comma
Milestone: M4

Approved by the maintainer, Nils Lutz, on 2026-11-23, as ABAP Cleaner's
"Remove space before commas and period" with its default options. The period
that ends a statement and every comma outside brackets follow the token
before them without a space, in class definitions too. These are the commas
of a chain (`DATA:`, `CLEAR:`, `WRITE:`, …) and, as in ABAP Cleaner, the
commas of ABAP SQL lists (`SELECT a, b`, `ORDER BY a, b`); commas inside
brackets (`IN ( 'AA' , 'LH' )`, SQL function arguments, embedded expressions
of string templates) keep their spacing. A period or comma on a line of its
own after an end-of-line comment or comment lines moves up behind the token
before it: the end-of-line comment then follows the period or comma, and the
comment lines stand before the next item or statement. A pragma counts as a
token (`##NEEDED.`).

Kept as in ABAP Cleaner: a period or comma after `/` (`WRITE / .`), a
period or comma with its own end-of-line comment that would have to move in
front of other comments, and the period of a `FUNCTION` statement behind
comment lines (the generated signature comment). Units printed as written
(W001, W002, W004) keep their spacing, and in statements without grammar
(W003) only spaces and line breaks are removed; comments do not move there.
One deviation: where a comma with its own end-of-line comment would move in
front of other comments, ABAP Cleaner turns that comment into a comment line;
origami keeps the comma where it is, because a comment never changes between
end of line and own line (INVARIANTS 1.3).

```abap
before: DATA lo_any TYPE REF TO object ##NEEDED .
        SELECT carrid , connid FROM spfli ORDER BY carrid , connid INTO TABLE @DATA(lt_conn) .
        lv_a = 42 " the answer
        .
        CLEAR: lv_a " first item
               , lv_b
        * comment line
               .
after:  DATA lo_any TYPE REF TO object ##NEEDED.
        SELECT carrid, connid FROM spfli ORDER BY carrid, connid INTO TABLE @DATA(lt_conn).
        lv_a = 42. " the answer
        CLEAR: lv_a, " first item
               lv_b.
        * comment line
```

### ABAP-DOC-001: Align ABAP Doc parameters and exceptions
Milestone: M4

Approved by the maintainer, Nils Lutz, on 2026-11-23, as ABAP Cleaner's
"Align parameters and exceptions in ABAP Doc" with its default options
(align across empty and non-empty ABAP Doc lines). In the ABAP Doc comment
lines (`"!`) in front of a statement, the lines `"! @parameter name | text`,
`"! @raising name | text` and `"! @exception name | text` are aligned: the
names start in one column, one space after the longest tag, and the `|`
signs line up one space after the longest name. Other ABAP Doc lines and
blank lines between them do not end the group; any other comment does. Only
the blanks after the tag and between the name and `|` change (INVARIANTS
1.3); everything else in the line stays as written.

As in ABAP Cleaner, a line is not aligned when it has no `|`, no text after
`|`, or more than one word before `|`, and ABAP Doc in front of a chain item
(`METHODS: "! … m1, …`) is not aligned. If padding would push a line past
`line_width` (and make it longer than as written), the ABAP Doc in front of
that statement keeps its spacing; ABAP Cleaner aligns whatever the width.
Other deviations: origami keeps the text between `"!` and the tag and after
`|` as written (ABAP Cleaner writes `"! @parameter` and one space after
`|`), and with other spacing before the tag the names still start in one
column.

```abap
before: "! @parameter iv_carrier | Airline
        "! @parameter iv_connection_id | Connection
        "! The connection must exist.
        "! @parameter rs_flight| Flight
        "! @raising zcx_not_found | Flight not found
after:  "! @parameter iv_carrier       | Airline
        "! @parameter iv_connection_id | Connection
        "! The connection must exist.
        "! @parameter rs_flight        | Flight
        "! @raising   zcx_not_found    | Flight not found
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
columns, for example annotation values, are not kept; CDS-ALIGN-001 aligns
`as` aliases and association targets.

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
after `{` and directly before `}` are removed. CDS-BLANK-002 adds blank lines
in view definitions.

### CDS-BLANK-002: Blank lines between the sections of a view
Milestone: M4

Approved by the maintainer, Nils Lutz, on 2026-11-25. In a DDLS view
definition (`define view`, `define view entity`, `define root view entity`,
projections), one blank line is added where none was written, as ABAP
Cleaner does:

* between the annotations at the top and `define`;
* between the view parameters and `as select from` (or `as projection on`);
* between `as select from` and the first join;
* between the data sources (joins) and the first association or composition;
* before the select list's `{` and after its `}`;
* in the select list: after the last `key` element, before the first element
  that exposes an association of the header (`_Agency`), and before and after
  an element that spans two or more lines, counting its annotations.

```cds
@EndUserText.label: 'Travel'

define view entity ZI_Travel
  as select from /dmo/travel

  association [0..1] to /DMO/I_Agency   as _Agency   on $projection.AgencyID = _Agency.AgencyID
  association [0..*] to /DMO/I_Customer as _Customer on $projection.CustomerID = _Customer.CustomerID

{
  key travel_id   as TravelID,

      @Semantics.amount.currencyCode: 'CurrencyCode'
      total_price as TotalPrice,

      agency_id   as AgencyID,

      _Agency,
      _Customer
}
```

**Compact:** no blank line is added.

### CDS-ALIGN-001: Aligned aliases and association targets
Milestone: M4

Approved by the maintainer, Nils Lutz, on 2026-11-25. In a select list, the
`as` aliases of one-line elements are aligned in one column, across blank
lines and annotations. Elements without an alias that are a name or a path
(`t.customer_id`) count for the column; elements that span several lines are
not aligned and do not count. In the header, consecutive one-line
associations and compositions align their `as` and `on`. A column that would
start after position 120 is not aligned. See the example in CDS-BLANK-002.

**Compact:** not aligned (CDS-SPACE-001).

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

### CDS-CLAUSE-001: Query clauses after the select list
Milestone: M2
Explanation: each query clause on its own line

Approved by the maintainer, Nils Lutz, on 2026-11-23. Adopted from ABAP Cleaner
1.29.2 ("Break before WHERE clause etc.", defaults). In a DDLS query, `where`,
`group by`, `having`, `union`, `except` and `intersect` start a line at the
level of the definition (the level of `define` and of the closing `}` of the
select list), also in every branch of a `union`. A `group` and `by` that the
author split over two lines are joined. Continuation lines of a clause keep
their position relative to the clause keyword, and never start left of one
indent level (CDS-CONT-001). Comments stay with the line they were written
on; a comment line before a clause moves with it. Blank lines are neither
added nor removed (CDS-BLANK-001). Not affected: clauses inside parentheses
(sub-selects, `exists`), `order by`, `union all` / `union distinct` (the second
word stays), the text after `union` (`select from …` stays on its line), and
DCL, DDLX and BDEF, which have no such clauses. Where the author wrote the
select list on one line (`{ key a, b }`), the clauses still go to their own
lines.

```cds
define view entity ZI_Flights
  as select from spfli { key carrid, key connid, cityfrom }
where carrid = 'LH' group by carrid, connid, cityfrom
having count(*) > 1
```

becomes

```cds
define view entity ZI_Flights
  as select from spfli { key carrid, key connid, cityfrom }
where carrid = 'LH'
group by carrid, connid, cityfrom
having count(*) > 1
```

Deviations from ABAP Cleaner: origami does not add the blank line that Cleaner
puts before `union` and `intersect`, and a continuation line never starts left
of one indent level, where Cleaner may put it in column 0.

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

