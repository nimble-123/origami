//! Language detection: explicit language, file name, then content sniffing.

use origami_syntax::{Code, Diagnostic, Language, SourceText, TokenStream};

/// What the caller knows about the input's language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageHint {
    /// Nothing known: sniff the content.
    None,
    /// A file path in path mode: an unmatched name is E011.
    Path(String),
    /// A file name in stdin mode: an unmatched name falls back to sniffing.
    StdinPath(String),
    /// A VS Code language ID (`abap`, `acds`).
    LanguageId(String),
    /// `--language <lang>`.
    Explicit(Language),
    /// `--language cds`: the CDS family, detected from content.
    CdsFamily,
}

/// What a file name says about its language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Lang(Language),
    /// `.acds`: the CDS family, detected from content.
    CdsFamily,
    Unsupported,
}

pub fn by_file_name(name: &str) -> FileKind {
    let lower = name.to_ascii_lowercase();
    let ext = |s: &str| lower.ends_with(s);
    let lang = if ext(".abap") {
        Language::Abap
    } else if ext(".asddls") {
        Language::Ddls
    } else if ext(".asddlxs") {
        Language::Ddlx
    } else if ext(".asdcls") {
        Language::Dcl
    } else if ext(".asbdef") {
        Language::Bdef
    } else if ext(".srvdsrv") {
        Language::Srvd
    } else if ext(".acds") {
        return FileKind::CdsFamily;
    } else {
        return FileKind::Unsupported;
    };
    FileKind::Lang(lang)
}

pub fn detect(src: &SourceText, hint: &LanguageHint) -> Result<Language, Diagnostic> {
    let sniffed =
        || sniff(src).ok_or_else(|| Diagnostic::new(Code::E010, "Language not detected", 0..0));
    match hint {
        LanguageHint::Explicit(l) => Ok(*l),
        LanguageHint::CdsFamily | LanguageHint::None => sniffed(),
        LanguageHint::LanguageId(id) => match id.as_str() {
            "abap" => Ok(Language::Abap),
            _ => sniffed(),
        },
        LanguageHint::Path(p) => match by_file_name(p) {
            FileKind::Lang(l) => Ok(l),
            FileKind::CdsFamily => sniffed(),
            FileKind::Unsupported => Err(Diagnostic::new(
                Code::E011,
                format!("Unsupported file type: {p}"),
                0..0,
            )),
        },
        LanguageHint::StdinPath(p) => match by_file_name(p) {
            FileKind::Lang(l) => Ok(l),
            _ => sniffed(),
        },
    }
}

/// Detects a CDS-family language from the first keywords after leading
/// annotations and comments.
pub fn sniff(src: &SourceText) -> Option<Language> {
    let stream = origami_cds::lex(src, Language::Ddls).ok()?;
    let words = leading_words(src.text(), &stream, 4);
    let w = |k: usize| words.get(k).map(String::as_str).unwrap_or("");
    match w(0) {
        "define" => match w(1) {
            "view" | "root" | "transient" | "table" | "scalar" | "external" | "hierarchy"
            | "custom" | "abstract" | "type" => Some(Language::Ddls),
            "role" | "accesspolicy" | "aspect" | "pfcg_mapping" => Some(Language::Dcl),
            "service" => Some(Language::Srvd),
            _ => None,
        },
        "extend" => match w(1) {
            "behavior" => Some(Language::Bdef),
            "service" => Some(Language::Srvd),
            _ => Some(Language::Ddls),
        },
        "annotate" => Some(Language::Ddlx),
        "managed" | "unmanaged" | "abstract" | "projection" | "interface" => Some(Language::Bdef),
        _ => None,
    }
}

/// The first `n` identifiers (lower case) after skipping annotations.
fn leading_words(text: &str, stream: &TokenStream, n: usize) -> Vec<String> {
    use origami_cds::kinds::*;
    let toks = &stream.tokens;
    let mut words = Vec::new();
    let mut k = 0;
    while k < toks.len() && words.len() < n {
        match toks[k].kind {
            AT | AT_LT => {
                k += 1;
                // Dotted annotation name.
                while k < toks.len() && matches!(toks[k].kind, IDENT | DOT) {
                    k += 1;
                }
                // Optional value.
                if k < toks.len() && toks[k].kind == COLON {
                    k += 1;
                    k = skip_value(toks, k);
                }
            }
            IDENT => {
                words.push(toks[k].text(text).to_ascii_lowercase());
                k += 1;
            }
            _ => k += 1,
        }
    }
    words
}

fn skip_value(toks: &[origami_syntax::Token], mut k: usize) -> usize {
    use origami_cds::kinds::*;
    if k >= toks.len() {
        return k;
    }
    match toks[k].kind {
        LBRACE | LBRACKET => {
            let mut depth = 0i32;
            while k < toks.len() {
                match toks[k].kind {
                    LBRACE | LBRACKET => depth += 1,
                    RBRACE | RBRACKET => {
                        depth -= 1;
                        if depth == 0 {
                            return k + 1;
                        }
                    }
                    _ => {}
                }
                k += 1;
            }
            k
        }
        _ => {
            // Scalar value, possibly dotted (`#X`, `'a'`, `true`, `a.b`).
            k += 1;
            while k + 1 < toks.len() && toks[k].kind == DOT && toks[k].glued {
                k += 2;
            }
            k
        }
    }
}
