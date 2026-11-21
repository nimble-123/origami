//! Input decoding, output encoding and position mapping.
//!
//! `decode` validates UTF-8, strips a byte-order mark and converts every line
//! break to LF. A line break is an LF together with all CRs directly before
//! it (`\r\n`, also `\r\r\n`), so the normalized text never has a CR before
//! an LF and a second run sees the same line breaks. All spans in origami
//! refer to this normalized text. `encode` restores
//! the BOM, writes every line break in the style of the input's first line
//! break, and restores whether the input ended with a newline.

use crate::diagnostic::{Code, Diagnostic};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

#[derive(Debug, Clone)]
pub struct SourceText {
    text: String,
    bom: bool,
    line_ending: LineEnding,
    final_newline: bool,
}

const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

impl SourceText {
    /// Decodes raw bytes. Invalid UTF-8 produces E005.
    pub fn decode(bytes: &[u8]) -> Result<SourceText, Diagnostic> {
        let (bom, body) = match bytes.strip_prefix(BOM) {
            Some(rest) => (true, rest),
            None => (false, bytes),
        };
        let raw = std::str::from_utf8(body).map_err(|e| {
            let at = e.valid_up_to() as u32;
            Diagnostic::new(
                Code::E005,
                format!("Input is not valid UTF-8 (byte offset {at})"),
                at..at,
            )
        })?;
        let line_ending = match raw.find('\n') {
            Some(i) if i > 0 && raw.as_bytes()[i - 1] == b'\r' => LineEnding::CrLf,
            _ => LineEnding::Lf,
        };
        let text = if raw.contains("\r\n") {
            let mut t = String::with_capacity(raw.len());
            for c in raw.chars() {
                if c == '\n' {
                    while t.ends_with('\r') {
                        t.pop();
                    }
                }
                t.push(c);
            }
            t
        } else {
            raw.to_string()
        };
        let final_newline = text.ends_with('\n');
        Ok(SourceText {
            text,
            bom,
            line_ending,
            final_newline,
        })
    }

    /// Builds a source from already normalized text (LF line breaks, no BOM).
    pub fn from_normalized(text: impl Into<String>) -> SourceText {
        let text = text.into();
        let final_newline = text.ends_with('\n');
        SourceText {
            text,
            bom: false,
            line_ending: LineEnding::Lf,
            final_newline,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn has_bom(&self) -> bool {
        self.bom
    }

    pub fn line_ending(&self) -> LineEnding {
        self.line_ending
    }

    pub fn has_final_newline(&self) -> bool {
        self.final_newline
    }

    /// Encodes normalized output text in the conventions of this input.
    pub fn encode(&self, out: &str) -> Vec<u8> {
        let mut body = out.to_string();
        if self.final_newline && !body.ends_with('\n') {
            body.push('\n');
        } else if !self.final_newline && body.ends_with('\n') {
            body.pop();
        }
        if self.line_ending == LineEnding::CrLf {
            body = body.replace('\n', "\r\n");
        }
        let mut bytes = Vec::with_capacity(body.len() + 3);
        if self.bom {
            bytes.extend_from_slice(BOM);
        }
        bytes.extend_from_slice(body.as_bytes());
        bytes
    }
}

/// A 1-based line and a 1-based column counted in UTF-16 code units, plus the
/// 0-based byte offset in the normalized text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub offset: u32,
}

/// Maps byte offsets of a text to line/column positions.
#[derive(Debug, Clone)]
pub struct LineIndex<'a> {
    text: &'a str,
    line_starts: Vec<u32>,
}

impl<'a> LineIndex<'a> {
    pub fn new(text: &'a str) -> LineIndex<'a> {
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i as u32 + 1);
            }
        }
        LineIndex { text, line_starts }
    }

    pub fn position(&self, offset: u32) -> Position {
        let offset = offset.min(self.text.len() as u32);
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let start = self.line_starts[line] as usize;
        let mut end = offset as usize;
        while !self.text.is_char_boundary(end) {
            end -= 1;
        }
        let column = self.text[start..end].encode_utf16().count() as u32 + 1;
        Position {
            line: line as u32 + 1,
            column,
            offset,
        }
    }

    /// Byte range of the given 0-based line, without its line break.
    pub fn line_range(&self, line: usize) -> std::ops::Range<usize> {
        let start = self.line_starts[line] as usize;
        let end = self
            .line_starts
            .get(line + 1)
            .map(|s| *s as usize - 1)
            .unwrap_or(self.text.len());
        start..end
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// 0-based line of a byte offset.
    pub fn line_of(&self, offset: u32) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        }
    }
}
