# origami invariants

origami changes layout only. This document defines exactly what "layout only"
means and how it is enforced. Every rule names the test that covers it.

## 1. Equality checked on every run (self-check)

After formatting, origami lexes its own output again with the same lexer and
compares it with the input (`origami_syntax::compare::equivalent`). The two are
equal when all of these hold:

| # | Rule | Test |
|---|---|---|
| 1.1 | Same number of tokens, same kinds, same texts. Only tokens a parser marked as keywords may differ in letter case. | `crates/origami_syntax/tests/compare.rs` |
| 1.2 | Same `glued` flag on every token: two tokens that touch in the input touch in the output, and two tokens separated by whitespace stay separated. | `compare.rs`, `crates/origami_abap/tests/lexer.rs::glued_flags` |
| 1.3 | Same comments: same text, same syntax (`*`, `"`, `"!`, `"#EC`, `//`/`--`, `/* */`), same placement (own line / end of line), attached to the same token in the same position. | `compare.rs`, `crates/origami_syntax/tests/trivia.rs` |
| 1.4 | Verbatim regions (AMDP, `EXEC SQL`, macro bodies) are single opaque tokens, so rule 1.1 compares them byte for byte. | `lexer.rs::verbatim_regions` |
| 1.5 | No ABAP output line exceeds 255 characters unless the same input line already did. | `crates/origami_core/src/check.rs` |

## 2. What happens on a violation

1. The core finds the units (ABAP: one statement or chain; CDS family: see
   below) that contain the mismatching tokens or the new overlong lines.
2. It prints them again as written (quarantine), reports W001 (mismatch) or
   W002 (length), and checks again. A unit still over 255 characters is
   printed byte for byte.
3. If the check still fails, the result is X001: no output is written, exit 70.

Tests: `crates/origami_core/tests/safety.rs` (run with
`--features fault-injection`).

CDS-family units at M0 are the token runs between `;`, `,`, `{` and `}`. M2
refines them to annotations, list elements, statements and headers.

## 3. Comment placement and anchors

| Comment | Placement | Anchor |
|---|---|---|
| any comment with a token before it on the same line | end of line | trails the preceding token |
| any other comment | own line | leads the next token |
| own-line comment with no token after it | own line | virtual end-of-file anchor |

Printing rules (from M1): `*` comments stay in column 1 unchanged; other own-line
comments are printed at the indent of the token they lead; only the first line
of a multi-line `/* */` comment is re-indented; an end-of-line comment forces a
line break after it.

## 4. Re-indentation of unknown and quarantined statements (from M1)

* shift = target indent − original indent of the statement's first line;
* the shift applies to every line except `*` comment lines, lines inside a
  block comment, and lines inside a multi-line literal or template;
* after the shift, a continuation line that is not a comment is indented by at
  least one indent step, so it can never start in column 1 (where a `*` would
  turn code into a comment);
* a tab in leading indentation counts as one indent step; shifted lines are
  written with spaces;
* verbatim regions are never shifted;
* a gap that contains a multi-line `/* */` comment is written unchanged;
* a token that starts with `*` is never printed in column 1, and the other
  lines of its unit are shifted by the same amount;
* a statement glued to the previous one without a separator (only possible
  in invalid code, such as `x.y` split at the period) keeps its bytes.

## 5. Structural errors

Only these leave a file untouched. Everything else is formatted, generically
if the grammar does not know a statement.

| Code | Trigger | Test |
|---|---|---|
| E001 | unterminated literal, string template or block comment | `lexer.rs::e001_cases` (ABAP and CDS) |
| E002 | unbalanced bracket | `lexer.rs::structure_errors`, CDS `e002_cases` |
| E003 | ABAP statement without terminating period (the CDS family has no E003) | `lexer.rs::structure_errors` |
| E004 | ABAP block-pair mismatch, or a verbatim region without its closer | `lexer.rs::structure_errors`, `missing_amdp_closer_is_e004` |
| E005 | input is not valid UTF-8 | `crates/origami_syntax/tests/source.rs` |

The block-pair table and the `SELECT`/`WITH` loop rule are in
`crates/origami_abap/src/block_pairs.rs`. An E004 on valid code is a bug there.

## 6. Encoding

Input must be UTF-8. A BOM, the line-break style of the first line break, and
the final-newline state are recorded and restored. Internally every line
break becomes LF; a line break is an LF together with all CRs directly before
it (so `\r\r\n` is one line break). The printer never writes a CR directly
before a line break. Both rules keep the line-break style stable from one run
to the next. Positions and offsets refer to that normalized text. Test:
`crates/origami_syntax/tests/source.rs`.
