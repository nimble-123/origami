//! Self-check, quarantine and the 255-character rule.
//!
//! 1. Format with no quarantine.
//! 2. Re-lex the output and compare it with the input (`equivalent`).
//! 3. Collect the units of every mismatch and, for ABAP, the units of output
//!    lines over 255 characters that were not over 255 in the input.
//! 4. Format again with those units quarantined (W001 for mismatches, W002 for
//!    length) and check again. Units still over 255 are printed opaque; if a
//!    new overlong line remains even then, the whole file stays as written.
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
    /// Units printed as written in the final pass.
    pub quarantine: BTreeSet<UnitId>,
    /// Units printed byte for byte in the final pass.
    pub opaque: BTreeSet<UnitId>,
    /// The whole file stayed as written (255-character rule).
    pub whole_file_fallback: bool,
}

/// Formats with the self-check. With `explain`, every pass also records rule
/// spans; the output is the same.
pub fn run(
    src: &SourceText,
    lang: Language,
    opts: &Options,
    identity: bool,
    explain: bool,
) -> Checked {
    let empty = BTreeSet::new();
    let done = |result, quarantine, opaque, whole_file_fallback| Checked {
        result,
        quarantine,
        opaque,
        whole_file_fallback,
    };
    let first = format(src, lang, opts, &empty, &empty, identity, explain);
    if first.has_error() {
        return done(first, empty.clone(), empty.clone(), false);
    }
    let output = apply_fault(first.output.clone(), opts, 0);
    let (mismatch_units, length_units) = problems(src, lang, &first, &output);
    if mismatch_units.is_empty() && length_units.is_empty() {
        return done(
            LangResult { output, ..first },
            empty.clone(),
            empty.clone(),
            false,
        );
    }

    // Quarantine.
    let quarantine: BTreeSet<UnitId> = mismatch_units.union(&length_units).copied().collect();
    let mut second = format(src, lang, opts, &quarantine, &empty, identity, explain);
    let mut output = apply_fault(second.output.clone(), opts, 1);
    let mut opaque = BTreeSet::new();
    let (still_bad, still_long) = problems(src, lang, &second, &output);
    if still_bad.is_empty() && !still_long.is_empty() {
        let third = format(src, lang, opts, &quarantine, &still_long, identity, explain);
        output = apply_fault(third.output.clone(), opts, 2);
        second = third;
        opaque = still_long;
    }
    let (final_bad, final_long) = problems(src, lang, &second, &output);
    // Last resort for the 255 rule: the joins between opaque units can still
    // lengthen a line. Then the whole file stays as written (INVARIANTS 1.5).
    let whole_file_fallback = final_bad.is_empty() && !final_long.is_empty();
    if whole_file_fallback {
        output = src.text().to_string();
    }
    let mut result = LangResult { output, ..second };
    if !final_bad.is_empty() {
        result.diagnostics.push(Diagnostic::new(
            Code::X001,
            "Self-check failed after quarantine; no output written",
            0..0,
        ));
        return done(result, quarantine, opaque, whole_file_fallback);
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
    done(result, quarantine, opaque, whole_file_fallback)
}

/// The final pass of `checked` printed again without a line-width limit
/// (same quarantine and opaque units). Not subject to the 255 rule.
pub(crate) fn print_wide(
    src: &SourceText,
    lang: Language,
    opts: &Options,
    identity: bool,
    checked: &Checked,
) -> String {
    let mut wide = opts.clone();
    wide.lang.line_width = u16::MAX;
    let r = format(
        src,
        lang,
        &wide,
        &checked.quarantine,
        &checked.opaque,
        identity,
        false,
    );
    apply_fault(r.output, opts, 3)
}

fn format(
    src: &SourceText,
    lang: Language,
    opts: &Options,
    quarantine: &BTreeSet<UnitId>,
    opaque: &BTreeSet<UnitId>,
    identity: bool,
    explain: bool,
) -> LangResult {
    #[cfg(feature = "fault-injection")]
    if opts.fault == Some(crate::options::Fault::Panic) {
        panic!("injected fault");
    }
    let o = &opts.lang;
    match (lang, explain) {
        (Language::Abap, false) => origami_abap::format(src, o, quarantine, opaque, identity),
        (Language::Abap, true) => {
            origami_abap::format_explained(src, o, quarantine, opaque, identity)
        }
        (cds, false) => origami_cds::format(src, cds, o, quarantine, opaque, identity),
        (cds, true) => origami_cds::format_explained(src, cds, o, quarantine, opaque, identity),
    }
}

pub(crate) fn relex(lang: Language, text: &str) -> Result<TokenStream, Diagnostic> {
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

/// Where the lines of the full output ended up in a partly formatted text.
#[derive(Debug, Clone, Default)]
pub struct LineMap {
    /// For each output line (0-based): its line in the mixed text, if kept.
    pub mixed: Vec<Option<u32>>,
    /// For each output line: it belongs to a change that was kept.
    pub kept_change: Vec<bool>,
}

/// Keeps only the changes of `r.output` that touch `ranges` (1-based,
/// inclusive input lines). First tries line by line within changed blocks
/// of equal length, then whole changed blocks; returns `None` if neither
/// result is token-equivalent to the input.
pub fn select_lines(
    src: &SourceText,
    lang: Language,
    r: &LangResult,
    ranges: &[(u32, u32)],
) -> Option<(String, LineMap)> {
    for fine in [true, false] {
        let (mixed, map) = mix_lines(src.text(), &r.output, ranges, fine);
        if let Ok(stream) = relex(lang, &mixed)
            && equivalent(&r.input_tokens, src.text(), &stream, &mixed).is_ok()
        {
            return Some((mixed, map));
        }
    }
    None
}

fn mix_lines(old: &str, new: &str, ranges: &[(u32, u32)], fine: bool) -> (String, LineMap) {
    use similar::{DiffTag, TextDiff};
    let diff = TextDiff::from_lines(old, new);
    let o: Vec<&str> = diff.iter_old_slices().collect();
    let n: Vec<&str> = diff.iter_new_slices().collect();
    // Old lines `a..b` (0-based); an insertion (a == b) touches the lines
    // on both sides.
    let touches = |a: usize, b: usize| {
        ranges.iter().any(|&(s, e)| {
            let (s, e) = (s as usize, e as usize);
            if a == b {
                s <= a + 1 && e >= a
            } else {
                s <= b && e > a
            }
        })
    };
    let mut out = String::with_capacity(new.len());
    let mut map = LineMap {
        mixed: vec![None; n.len()],
        kept_change: vec![false; n.len()],
    };
    let mut line = 0u32;
    for op in diff.ops() {
        let (tag, or, nr) = op.as_tag_tuple();
        if tag == DiffTag::Equal {
            for (j, l) in nr.zip(&o[or]) {
                map.mixed[j] = Some(line);
                line += 1;
                out.push_str(l);
            }
        } else if fine && or.len() == nr.len() {
            for (i, (ol, nl)) in o[or.clone()].iter().zip(&n[nr.clone()]).enumerate() {
                let k = or.start + i;
                if touches(k, k + 1) {
                    map.mixed[nr.start + i] = Some(line);
                    map.kept_change[nr.start + i] = true;
                    out.push_str(nl);
                } else {
                    out.push_str(ol);
                }
                line += 1;
            }
        } else if touches(or.start, or.end) {
            for j in nr {
                map.mixed[j] = Some(line);
                map.kept_change[j] = true;
                line += 1;
                out.push_str(n[j]);
            }
        } else {
            line += or.len() as u32;
            o[or].iter().for_each(|l| out.push_str(l));
        }
    }
    (out, map)
}
