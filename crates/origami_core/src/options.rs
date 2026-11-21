//! Formatter options and their validation.

use origami_syntax::{Code, Diagnostic, Edition, LangOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub lang: LangOptions,
    pub style_edition: Edition,
    #[cfg(feature = "fault-injection")]
    pub fault: Option<Fault>,
}

/// Test-only faults, applied between printing and the self-check.
#[cfg(feature = "fault-injection")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    /// Drop one space on the first pass only: quarantine fixes it (W001).
    DropSpaceOnce,
    /// Drop one space on every pass: the self-check fails (X001).
    DropSpaceAlways,
    /// Panic inside the core (X002).
    Panic,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            lang: LangOptions::default(),
            style_edition: Edition::E2026,
            #[cfg(feature = "fault-injection")]
            fault: None,
        }
    }
}

impl Options {
    /// Rejects out-of-range values with E020.
    pub fn validate(&self) -> Result<(), Diagnostic> {
        let bad = |m: String| Err(Diagnostic::new(Code::E020, m, 0..0));
        if !(40..=255).contains(&self.lang.line_width) {
            return bad(format!(
                "line_width must be 40..=255, got {}",
                self.lang.line_width
            ));
        }
        if !(1..=8).contains(&self.lang.indent_width) {
            return bad(format!(
                "indent_width must be 1..=8, got {}",
                self.lang.indent_width
            ));
        }
        Ok(())
    }
}
