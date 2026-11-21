//! Printer.
//!
//! In identity mode the printer returns the normalized source text unchanged.
//! This is the M0 behavior: the whole pipeline, including the self-check, runs
//! end to end while the output equals the input. The full printer (break
//! propagation, fits check, re-indent rules, alignment) arrives in M1.

use crate::ir::Doc;
use origami_syntax::{Case, SourceText, TokenStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintOptions {
    pub width: u16,
    pub indent: u8,
    /// `None` leaves every token as written (CDS family).
    pub keyword_case: Option<Case>,
    /// Return the source text unchanged.
    pub identity: bool,
}

/// Prints `doc`. Panics if `identity` is false: the full printer is not part of
/// M0, and the core catches the panic as X002.
pub fn print(_doc: &Doc, src: &SourceText, _tokens: &TokenStream, opts: &PrintOptions) -> String {
    if opts.identity {
        return src.text().to_string();
    }
    unimplemented!("the full printer arrives in milestone M1")
}
