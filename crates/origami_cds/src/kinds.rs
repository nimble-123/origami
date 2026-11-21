//! CDS-family token kinds. Node kinds arrive with the M2/M3 parsers.

/// Identifier or keyword, optionally with namespace prefixes: `/DMO/I_Travel`.
pub const IDENT: u16 = 1;
/// `$` followed by an identifier: `$projection`, `$parameters`, `$session`.
pub const DOLLAR_IDENT: u16 = 2;
/// `#` followed by an identifier: enum value `#TEXT`.
pub const ENUM_VALUE: u16 = 3;
/// String literal `'…'`.
pub const STRING: u16 = 4;
/// Number literal.
pub const NUMBER: u16 = 5;
/// `@` (annotation start).
pub const AT: u16 = 6;
/// `@<` (post-annotation start).
pub const AT_LT: u16 = 7;
pub const LBRACE: u16 = 8;
pub const RBRACE: u16 = 9;
pub const LPAREN: u16 = 10;
pub const RPAREN: u16 = 11;
pub const LBRACKET: u16 = 12;
pub const RBRACKET: u16 = 13;
pub const SEMICOLON: u16 = 14;
pub const COMMA: u16 = 15;
pub const DOT: u16 = 16;
pub const COLON: u16 = 17;
/// Any other punctuation or operator character: `= < > ! + - * / :` and so on.
pub const PUNCT: u16 = 18;
