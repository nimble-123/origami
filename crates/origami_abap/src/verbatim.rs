//! Verbatim regions (design decision D8): the bodies of AMDP methods,
//! `EXEC SQL` blocks and macro definitions are copied byte for byte.
//!
//! The lexer lexes normally. When it emits the period that ends an opener
//! statement, it calls [`scan_body`], which returns the offset of the first
//! byte of the closer keyword. The body (from directly after the opener's
//! period up to the closer) becomes one opaque token, including the newline
//! after the period and the closer line's indentation.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    Amdp,
    ExecSql,
    Macro,
}

impl RegionKind {
    /// Recognizes an opener statement from its leading words (upper case).
    pub fn of_statement(words: &[String]) -> Option<RegionKind> {
        let w = |k: usize| words.get(k).map(String::as_str).unwrap_or("");
        match w(0) {
            "METHOD" => {
                let by = words.iter().position(|x| x == "BY")?;
                if w(by + 1) == "DATABASE"
                    && matches!(w(by + 2), "PROCEDURE" | "FUNCTION" | "GRAPH")
                {
                    Some(RegionKind::Amdp)
                } else {
                    None
                }
            }
            "EXEC" if w(1) == "SQL" => Some(RegionKind::ExecSql),
            "DEFINE" if !w(1).is_empty() => Some(RegionKind::Macro),
            _ => None,
        }
    }

    pub fn closer(self) -> &'static str {
        match self {
            RegionKind::Amdp => "ENDMETHOD",
            RegionKind::ExecSql => "ENDEXEC",
            RegionKind::Macro => "END-OF-DEFINITION",
        }
    }
}

fn is_word_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'/' | b'%' | b'$')
}

/// Returns the offset of the closer keyword, or `None` if there is none.
pub fn scan_body(text: &str, from: usize, kind: RegionKind) -> Option<usize> {
    let b = text.as_bytes();
    let closer = kind.closer().as_bytes();
    let mut j = from;
    while j < b.len() {
        let line_start = j == 0 || b[j - 1] == b'\n';
        let c = b[j];
        if line_start && c == b'*' {
            j = skip_to_eol(b, j);
            continue;
        }
        match (kind, c) {
            (RegionKind::Amdp, b'"') => j = skip_quoted(b, j, b'"'),
            (_, b'\'') => j = skip_quoted(b, j, b'\''),
            // ABAP text only: in SQLScript and native SQL `|` is the
            // concatenation `||` and a backquote is not a delimiter.
            (RegionKind::Macro, b'`') => j = skip_quoted(b, j, b'`'),
            (RegionKind::Macro, b'|') => j = skip_template(b, j),
            (RegionKind::Amdp, b'-') if b.get(j + 1) == Some(&b'-') => j = skip_to_eol(b, j),
            (RegionKind::Amdp, b'/') if b.get(j + 1) == Some(&b'*') => {
                j = text[j + 2..]
                    .find("*/")
                    .map(|p| j + 2 + p + 2)
                    .unwrap_or(b.len());
            }
            (RegionKind::ExecSql | RegionKind::Macro, b'"') => j = skip_to_eol(b, j),
            _ => {
                let boundary_before = j == from || !is_word_char(b[j - 1]);
                if boundary_before && matches_closer(b, j, closer) {
                    return Some(j);
                }
                j += 1;
            }
        }
    }
    None
}

fn matches_closer(b: &[u8], j: usize, closer: &[u8]) -> bool {
    let end = j + closer.len();
    if end > b.len() || !b[j..end].eq_ignore_ascii_case(closer) {
        return false;
    }
    let mut k = end;
    while matches!(b.get(k), Some(b' ' | b'\t')) {
        k += 1;
    }
    b.get(k) == Some(&b'.')
}

fn skip_to_eol(b: &[u8], j: usize) -> usize {
    b[j..]
        .iter()
        .position(|&c| c == b'\n')
        .map(|p| j + p)
        .unwrap_or(b.len())
}

/// End of a string template that starts at `|`: `\\` escapes the next byte and
/// `{ … }` embeds an expression, which may hold literals and templates.
/// Iterative, so that deep nesting cannot exhaust the stack: `open` holds one
/// entry per open template (`false`) or embedded expression (`true`).
fn skip_template(b: &[u8], j: usize) -> usize {
    let mut open = vec![false];
    let mut k = j + 1;
    while k < b.len() {
        let embedded = *open.last().unwrap_or(&false);
        match b[k] {
            b'\\' if !embedded => k += 2,
            b'|' if !embedded => {
                open.pop();
                k += 1;
                if open.is_empty() {
                    return k;
                }
            }
            b'{' if !embedded => {
                open.push(true);
                k += 1;
            }
            b'}' if embedded => {
                open.pop();
                k += 1;
            }
            b'|' => {
                open.push(false);
                k += 1;
            }
            b'\'' if embedded => k = skip_quoted(b, k, b'\''),
            b'`' if embedded => k = skip_quoted(b, k, b'`'),
            _ => k += 1,
        }
    }
    b.len()
}

fn skip_quoted(b: &[u8], j: usize, q: u8) -> usize {
    let mut k = j + 1;
    while k < b.len() {
        if b[k] == q {
            if b.get(k + 1) == Some(&q) {
                k += 2;
                continue;
            }
            return k + 1;
        }
        k += 1;
    }
    b.len()
}
