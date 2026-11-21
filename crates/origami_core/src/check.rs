//! Self-check, quarantine and the 255-character rule.
//!
//! 1. Format with no quarantine.
//! 2. Re-lex the output and compare it with the input (`equivalent`).
//! 3. Collect the units of every mismatch and, for ABAP, the units of output
//!    lines over 255 characters that were not over 255 in the input.
//! 4. Format again with those units quarantined (W001 for mismatches, W002 for
//!    length) and check again. Units still over 255 are printed opaque.
//! 5. If the comparison still fails, report X001.

use crate::options::Options;
use origami_syntax::{
    Code, Diagnostic, LangResult, Language, LineIndex, SourceText, TokenIndex, TokenStream, UnitId,
    equivalent,
};
use std::collections::BTreeSet;

const ABAP_MAX_LINE: usize = 255;

pub struct Checked {
    pub result: LangResult,
}

pub fn run(src: &SourceText, lang: Language, opts: &Options, identity: bool) -> Checked {
    let empty = BTreeSet::new();
    let first = format(src, lang, opts, &empty, &empty, identity);
    if first.has_error() {
        return Checked { result: first };
    }
    let output = apply_fault(first.output.clone(), opts, 0);
    let (mismatch_units, length_units) = problems(src, lang, &first, &output);
    if mismatch_units.is_empty() && length_units.is_empty() {
        return Checked {
            result: LangResult { output, ..first },
        };
    }

    // Quarantine.
    let quarantine: BTreeSet<UnitId> = mismatch_units.union(&length_units).copied().collect();
    let mut second = format(src, lang, opts, &quarantine, &empty, identity);
    let mut output = apply_fault(second.output.clone(), opts, 1);
    let (still_bad, still_long) = problems(src, lang, &second, &output);
    if still_bad.is_empty() && !still_long.is_empty() {
        let third = format(src, lang, opts, &quarantine, &still_long, identity);
        output = apply_fault(third.output.clone(), opts, 2);
        second = third;
    }
    let (final_bad, _) = problems(src, lang, &second, &output);
    let mut result = LangResult { output, ..second };
    if !final_bad.is_empty() {
        result.diagnostics.push(Diagnostic::new(
            Code::X001,
            "Self-check failed after quarantine; no output written",
            0..0,
        ));
        return Checked { result };
    }
    for u in &mismatch_units {
        result.diagnostics.push(unit_diagnostic(&result, *u, Code::W001, "Unit left as written: the self-check found a layout change that would alter tokens or comments"));
    }
    for u in length_units.difference(&mismatch_units) {
        result.diagnostics.push(unit_diagnostic(
            &result,
            *u,
            Code::W002,
            "Unit left as written: formatting would exceed 255 characters per line",
        ));
    }
    result.stats.quarantined = quarantine.len() as u32;
    Checked { result }
}

fn format(
    src: &SourceText,
    lang: Language,
    opts: &Options,
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    identity: bool,
) -> LangResult {
    #[cfg(feature = "fault-injection")]
    if opts.fault == Some(crate::options::Fault::Panic) {
        panic!("injected fault");
    }
    match lang {
        Language::Abap => origami_abap::format(src, &opts.lang, quarantine, opaque, identity),
        cds => origami_cds::format(src, cds, &opts.lang, quarantine, opaque, identity),
    }
}

fn relex(lang: Language, text: &str) -> Result<TokenStream, Diagnostic> {
    let src = SourceText::from_normalized(text);
    match lang {
        Language::Abap => origami_abap::lex(&src),
        cds => origami_cds::lex(&src, cds),
    }
}

/// Units with token mismatches, and (ABAP) units with new overlong lines.
fn problems(
    src: &SourceText,
    lang: Language,
    r: &LangResult,
    output: &str,
) -> (BTreeSet<UnitId>, BTreeSet<UnitId>) {
    let mut bad = BTreeSet::new();
    let mut long = BTreeSet::new();
    let out_stream = match relex(lang, output) {
        Ok(s) => s,
        Err(_) => {
            bad.extend(r.units.iter().map(|u| u.id));
            return (bad, long);
        }
    };
    if let Err(ms) = equivalent(&r.input_tokens, src.text(), &out_stream, output) {
        for m in ms {
            if let Some(u) = r.unit_of(m.input_index) {
                bad.insert(u);
            }
        }
    }
    if lang == Language::Abap && bad.is_empty() {
        let out_index = LineIndex::new(output);
        let in_index = LineIndex::new(src.text());
        for line in 0..out_index.line_count() {
            let range = out_index.line_range(line);
            if output[range.clone()].chars().count() <= ABAP_MAX_LINE {
                continue;
            }
            // Token streams are equivalent, so output token k is input token k.
            for (k, t) in out_stream.tokens.iter().enumerate() {
                if (t.start as usize) < range.start || (t.start as usize) >= range.end {
                    continue;
                }
                let input_tok = &r.input_tokens.tokens[k];
                let in_line = in_index.line_range(in_index.line_of(input_tok.start));
                if src.text()[in_line].chars().count() > ABAP_MAX_LINE {
                    continue;
                }
                if let Some(u) = r.unit_of(TokenIndex(k as u32)) {
                    long.insert(u);
                }
            }
        }
    }
    (bad, long)
}

fn unit_diagnostic(r: &LangResult, unit: UnitId, code: Code, msg: &str) -> Diagnostic {
    let range = r
        .units
        .iter()
        .find(|u| u.id == unit)
        .and_then(|u| {
            let a = r.input_tokens.tokens.get(u.tokens.start.0 as usize)?;
            let b = r.input_tokens.tokens.get(u.tokens.end.0 as usize - 1)?;
            Some(a.start..b.end)
        })
        .unwrap_or(0..0);
    Diagnostic::new(code, msg, range)
}

#[cfg(feature = "fault-injection")]
fn apply_fault(output: String, opts: &Options, pass: u8) -> String {
    use crate::options::Fault;
    let drop = match opts.fault {
        Some(Fault::DropSpaceOnce) => pass == 0,
        Some(Fault::DropSpaceAlways) => true,
        _ => false,
    };
    if !drop {
        return output;
    }
    // Remove the first single space that separates two non-blank characters.
    let b = output.as_bytes();
    for k in 1..b.len().saturating_sub(1) {
        if b[k] == b' ' && !b[k - 1].is_ascii_whitespace() && !b[k + 1].is_ascii_whitespace() {
            let mut s = output.clone();
            s.remove(k);
            return s;
        }
    }
    output
}

#[cfg(not(feature = "fault-injection"))]
fn apply_fault(output: String, _opts: &Options, _pass: u8) -> String {
    output
}
