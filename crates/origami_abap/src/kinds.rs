//! ABAP token kinds. Node kinds for the syntax tree arrive with the M1 parser.

/// Any run of characters that is not punctuation: keywords, identifiers,
/// numbers, operators such as `=` or `+`, `/NS/NAME`, `%cid`, `<fs>`, `!name`.
pub const WORD: u16 = 1;
/// Character literal `'…'`, including a text symbol suffix only when glued as
/// a separate `(…)` group.
pub const CHAR_LITERAL: u16 = 2;
/// String literal `` `…` ``.
pub const STRING_LITERAL: u16 = 3;
/// A complete string template without embedded expressions: `|…|`.
pub const TEMPLATE_FULL: u16 = 4;
/// Template part up to the first embedded expression: `|…{`.
pub const TEMPLATE_HEAD: u16 = 5;
/// Template part between two embedded expressions: `}…{`.
pub const TEMPLATE_MIDDLE: u16 = 6;
/// Template part after the last embedded expression: `}…|`.
pub const TEMPLATE_TAIL: u16 = 7;
pub const PERIOD: u16 = 8;
pub const COMMA: u16 = 9;
pub const COLON: u16 = 10;
pub const LPAREN: u16 = 11;
pub const RPAREN: u16 = 12;
pub const LBRACKET: u16 = 13;
pub const RBRACKET: u16 = 14;
pub const LBRACE: u16 = 15;
pub const RBRACE: u16 = 16;
/// `@` host-expression prefix.
pub const AT: u16 = 17;
/// Component and selector operators between names: `->`, `=>`, `~`.
pub const SELECTOR: u16 = 18;
/// Pragma `##NAME` or `##NAME[…]`.
pub const PRAGMA: u16 = 19;
/// Body of an AMDP method, `EXEC SQL` or macro definition, kept byte for byte.
pub const OPAQUE: u16 = 20;
