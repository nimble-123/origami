//! Lossless ABAP lexer.
//!
//! Every byte of the input ends up in exactly one token or one trivia piece.
//! ABAP requires whitespace around arithmetic and comparison operators, so the
//! lexer splits only at whitespace, at the characters `. , : ( ) [ ] { } ' ` | "`,
//! at the selectors `->`, `=>` and `~` when they stand between two non-blank
//! characters, and after a leading `@`. Everything else is one word, including
//! `/NS/NAME`, `%cid`, `<fs>`, `!name`, `ls_struc-comp` and `#`.
//!
//! The component selector `-` is not split: `ls_struc-comp` is one word, and
//! the M1 parser splits it. Keywords such as `END-OF-DEFINITION` therefore stay
//! one word as well.

use crate::kinds::*;
use crate::verbatim::{self, RegionKind};
use origami_syntax::trivia::attach;
use origami_syntax::{
    Code, CommentSyntax, Diagnostic, Placement, SourceText, Token, TokenStream, Trivia, TriviaKind,
};

pub fn lex(src: &SourceText) -> Result<TokenStream, Diagnostic> {
    let mut lx = Lexer::new(src.text());
    lx.run()?;
    let Lexer {
        tokens, mut trivia, ..
    } = lx;
    attach(&tokens, &mut trivia);
    Ok(TokenStream { tokens, trivia })
}

struct Lexer<'a> {
    text: &'a str,
    b: &'a [u8],
    i: usize,
    tokens: Vec<Token>,
    trivia: Vec<Trivia>,
    gap: bool,
    token_on_line: bool,
    /// Index of the first token of the current statement.
    stmt_start: usize,
    /// One entry per open template expression: depth of `{` inside it.
    templates: Vec<u32>,
}

fn is_delimiter(c: u8) -> bool {
    matches!(
        c,
        b' ' | b'\t'
            | b'\r'
            | b'\n'
            | b'.'
            | b','
            | b':'
            | b'('
            | b')'
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'\''
            | b'`'
            | b'|'
            | b'"'
    )
}

impl<'a> Lexer<'a> {
    fn new(text: &'a str) -> Lexer<'a> {
        Lexer {
            text,
            b: text.as_bytes(),
            i: 0,
            tokens: Vec::new(),
            trivia: Vec::new(),
            gap: true,
            token_on_line: false,
            stmt_start: 0,
            templates: Vec::new(),
        }
    }

    fn at(&self, k: usize) -> u8 {
        self.b.get(k).copied().unwrap_or(0)
    }

    fn starts_with(&self, s: &str) -> bool {
        self.b[self.i..].starts_with(s.as_bytes())
    }

    fn push(&mut self, kind: u16, end: usize) {
        let glued = !self.gap && !self.tokens.is_empty();
        self.tokens.push(Token {
            kind,
            start: self.i as u32,
            end: end as u32,
            glued,
            keyword: false,
        });
        self.i = end;
        self.gap = false;
        self.token_on_line = true;
    }

    fn push_trivia(&mut self, kind: TriviaKind, end: usize) {
        self.trivia.push(Trivia::new(kind, self.i..end));
        self.i = end;
        self.gap = true;
    }

    fn error(&self, at: usize, message: &str) -> Diagnostic {
        Diagnostic::new(Code::E001, message, at as u32..at as u32 + 1)
    }

    fn run(&mut self) -> Result<(), Diagnostic> {
        while self.i < self.b.len() {
            let c = self.b[self.i];
            let col0 = self.i == 0 || self.b[self.i - 1] == b'\n';
            match c {
                b'\n' => {
                    self.push_trivia(TriviaKind::Newline, self.i + 1);
                    self.token_on_line = false;
                }
                b' ' | b'\t' | b'\r' => {
                    let mut j = self.i;
                    while matches!(self.at(j), b' ' | b'\t' | b'\r') {
                        j += 1;
                    }
                    self.push_trivia(TriviaKind::Whitespace, j);
                }
                b'*' if col0 => {
                    let end = self.line_end(self.i);
                    let kind = TriviaKind::Comment {
                        syntax: CommentSyntax::AbapStar,
                        placement: Placement::OwnLine,
                    };
                    self.push_trivia(kind, end);
                }
                b'"' => {
                    let end = self.line_end(self.i);
                    let syntax = if self.starts_with("\"!") {
                        CommentSyntax::AbapDoc
                    } else if self.starts_with("\"#EC") {
                        CommentSyntax::AbapPseudo
                    } else {
                        CommentSyntax::AbapQuote
                    };
                    let placement = if self.token_on_line {
                        Placement::EndOfLine
                    } else {
                        Placement::OwnLine
                    };
                    self.push_trivia(TriviaKind::Comment { syntax, placement }, end);
                }
                b'\'' | b'`' => {
                    let end = self.quoted(c)?;
                    self.push(
                        if c == b'\'' {
                            CHAR_LITERAL
                        } else {
                            STRING_LITERAL
                        },
                        end,
                    );
                }
                b'|' => self.template(true)?,
                b'}' if self.templates.last() == Some(&0) => {
                    self.templates.pop();
                    self.template(false)?;
                }
                b'{' => {
                    if let Some(d) = self.templates.last_mut() {
                        *d += 1;
                    }
                    self.push(LBRACE, self.i + 1);
                }
                b'}' => {
                    if let Some(d) = self.templates.last_mut() {
                        *d -= 1;
                    }
                    self.push(RBRACE, self.i + 1);
                }
                b'.' => {
                    self.push(PERIOD, self.i + 1);
                    self.after_period()?;
                }
                b',' => self.push(COMMA, self.i + 1),
                b':' => self.push(COLON, self.i + 1),
                b'(' => self.push(LPAREN, self.i + 1),
                b')' => self.push(RPAREN, self.i + 1),
                b'[' => self.push(LBRACKET, self.i + 1),
                b']' => self.push(RBRACKET, self.i + 1),
                b'@' if !is_delimiter(self.at(self.i + 1)) => self.push(AT, self.i + 1),
                _ if self.starts_with("##") => {
                    let mut j = self.i + 2;
                    while self.at(j).is_ascii_alphanumeric() || self.at(j) == b'_' {
                        j += 1;
                    }
                    // Parameters `[p]…`; an unclosed `[` (blank or line end
                    // first) is not part of the pragma, so the token never
                    // ends in a blank the printer would trim.
                    while self.at(j) == b'[' {
                        let mut k = j + 1;
                        while k < self.b.len()
                            && !matches!(self.b[k], b']' | b' ' | b'\t' | b'\r' | b'\n')
                        {
                            k += 1;
                        }
                        if self.at(k) != b']' {
                            break;
                        }
                        j = k + 1;
                    }
                    self.push(PRAGMA, j);
                }
                _ if !self.gap && self.selector_len(self.i) > 0 => {
                    let n = self.selector_len(self.i);
                    self.push(SELECTOR, self.i + n);
                }
                _ => {
                    let mut j = self.i + 1;
                    while j < self.b.len() && !is_delimiter(self.b[j]) && self.selector_len(j) == 0
                    {
                        j += 1;
                    }
                    // Keep multi-byte characters whole.
                    while !self.text.is_char_boundary(j) {
                        j += 1;
                    }
                    self.push(WORD, j);
                }
            }
        }
        if !self.templates.is_empty() {
            return Err(self.error(
                self.b.len().saturating_sub(1),
                "Unterminated string template",
            ));
        }
        Ok(())
    }

    /// Length of a selector (`->`, `=>`, `~`) at `k` that stands between two
    /// non-blank characters (for example `obj->m`, `meth( )->m`), or 0.
    fn selector_len(&self, k: usize) -> usize {
        if k == 0 || matches!(self.at(k - 1), b' ' | b'\t' | b'\r' | b'\n') {
            return 0;
        }
        let n = match (self.at(k), self.at(k + 1)) {
            (b'-', b'>') | (b'=', b'>') => 2,
            (b'~', _) => 1,
            _ => return 0,
        };
        let next = self.at(k + n);
        if next == 0 || is_delimiter(next) {
            0
        } else {
            n
        }
    }

    /// End of a line comment: the line end, without trailing blanks (they are
    /// whitespace, so the printer may drop them).
    fn line_end(&self, from: usize) -> usize {
        let mut end = self.b[from..]
            .iter()
            .position(|&c| c == b'\n')
            .map(|p| from + p)
            .unwrap_or(self.b.len());
        while end > from + 1 && matches!(self.b[end - 1], b' ' | b'\t' | b'\r') {
            end -= 1;
        }
        end
    }

    /// End of a `'…'` or `` `…` `` literal; a doubled delimiter is an escape.
    fn quoted(&self, q: u8) -> Result<usize, Diagnostic> {
        let mut j = self.i + 1;
        loop {
            match self.at(j) {
                0 if j >= self.b.len() => return Err(self.error(self.i, "Unterminated literal")),
                b'\n' => return Err(self.error(self.i, "Unterminated literal")),
                c if c == q => {
                    if self.at(j + 1) == q {
                        j += 2;
                    } else {
                        return Ok(j + 1);
                    }
                }
                _ => j += 1,
            }
        }
    }

    /// Lexes one literal part of a string template, starting at `|` (start) or
    /// at the `}` that ends an embedded expression.
    fn template(&mut self, start: bool) -> Result<(), Diagnostic> {
        let mut j = self.i + 1;
        loop {
            if j >= self.b.len() {
                return Err(self.error(self.i, "Unterminated string template"));
            }
            match self.b[j] {
                b'\\' => j += 2,
                b'|' => {
                    self.push(if start { TEMPLATE_FULL } else { TEMPLATE_TAIL }, j + 1);
                    return Ok(());
                }
                b'{' => {
                    self.push(
                        if start {
                            TEMPLATE_HEAD
                        } else {
                            TEMPLATE_MIDDLE
                        },
                        j + 1,
                    );
                    self.templates.push(0);
                    return Ok(());
                }
                _ => j += 1,
            }
        }
    }

    /// After a period: start a new statement, and emit an opaque body if the
    /// finished statement opens an AMDP method, `EXEC SQL` or a macro.
    fn after_period(&mut self) -> Result<(), Diagnostic> {
        let words: Vec<String> = self.tokens[self.stmt_start..]
            .iter()
            .filter(|t| t.kind == WORD)
            .take(8)
            .map(|t| t.text(self.text).to_ascii_uppercase())
            .collect();
        self.stmt_start = self.tokens.len();
        let Some(kind) = RegionKind::of_statement(&words) else {
            return Ok(());
        };
        let opener_at = self.tokens[self.tokens.len() - 1].start;
        let end = verbatim::scan_body(self.text, self.i, kind).ok_or_else(|| {
            Diagnostic::new(
                Code::E004,
                format!("Missing {} for this block", kind.closer()),
                opener_at..opener_at + 1,
            )
        })?;
        if end > self.i {
            self.push(OPAQUE, end);
            self.stmt_start = self.tokens.len();
        }
        Ok(())
    }
}
