//! Config discovery and merging: defaults, then `origami.toml`, then flags.

use crate::args::{Args, CaseArg, StyleArg};
use origami_core::{Case, Code, Edition, Options, Position, Style};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

pub const FILE_NAME: &str = "origami.toml";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileConfig {
    pub style_edition: Option<String>,
    pub required_version: Option<String>,
    pub line_width: Option<u16>,
    pub indent_width: Option<u8>,
    pub preview: Option<bool>,
    pub style: Option<String>,
    pub keyword_case: Option<String>,
    pub unknown_keys: Vec<String>,
}

/// A config problem: E020 (aborts the run) or W101 (unknown key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigIssue {
    pub code: Code,
    pub path: PathBuf,
    pub message: String,
    /// Where in the config file, if known (else line 1, column 1).
    pub at: Option<Position>,
    /// The config text, for a code frame.
    pub source: Option<String>,
}

pub fn parse(text: &str, path: &Path) -> Result<FileConfig, ConfigIssue> {
    let issue = |m: String| ConfigIssue {
        code: Code::E020,
        path: path.to_path_buf(),
        message: m,
        at: None,
        source: None,
    };
    let table: toml::Table = text.parse().map_err(|e: toml::de::Error| ConfigIssue {
        at: e.span().map(|s| crate::report::position(text, s.start)),
        source: Some(text.to_string()),
        ..issue(format!("Invalid TOML: {}", e.message().trim_end()))
    })?;
    let mut c = FileConfig::default();
    for (k, v) in &table {
        let wrong = |t: &str| issue(format!("`{k}` must be {t}"));
        match k.as_str() {
            "style_edition" => {
                c.style_edition = Some(v.as_str().ok_or_else(|| wrong("a string"))?.into())
            }
            "required_version" => {
                c.required_version = Some(v.as_str().ok_or_else(|| wrong("a string"))?.into())
            }
            "line_width" => {
                let n = v.as_integer().ok_or_else(|| wrong("an integer"))?;
                c.line_width = Some(u16::try_from(n).map_err(|_| wrong("40..=255"))?);
            }
            "indent_width" => {
                let n = v.as_integer().ok_or_else(|| wrong("an integer"))?;
                c.indent_width = Some(u8::try_from(n).map_err(|_| wrong("1..=8"))?);
            }
            "preview" => c.preview = Some(v.as_bool().ok_or_else(|| wrong("a boolean"))?),
            "style" => c.style = Some(v.as_str().ok_or_else(|| wrong("a string"))?.into()),
            "abap" => {
                let t = v.as_table().ok_or_else(|| wrong("a table"))?;
                for (ak, av) in t {
                    if ak == "keyword_case" {
                        let s = av
                            .as_str()
                            .ok_or_else(|| issue("`abap.keyword_case` must be a string".into()))?;
                        c.keyword_case = Some(s.into());
                    } else {
                        c.unknown_keys.push(format!("abap.{ak}"));
                    }
                }
            }
            _ => c.unknown_keys.push(k.clone()),
        }
    }
    Ok(c)
}

/// Checks `required_version` against this binary.
pub fn satisfies(required: &str) -> Result<bool, String> {
    let req = semver::VersionReq::parse(required)
        .map_err(|e| format!("Invalid required_version: {e}"))?;
    let me = semver::Version::parse(env!("CARGO_PKG_VERSION")).expect("valid crate version");
    Ok(req.matches(&me))
}

/// Builds options from defaults, a config file and flags (flags win).
pub fn options(file: &FileConfig, args: &Args, path: &Path) -> Result<Options, ConfigIssue> {
    let issue = |m: String| ConfigIssue {
        code: Code::E020,
        path: path.to_path_buf(),
        message: m,
        at: None,
        source: None,
    };
    let mut o = Options::default();
    if let Some(e) = &file.style_edition {
        o.style_edition =
            Edition::parse(e).ok_or_else(|| issue(format!("Unknown style_edition `{e}`")))?;
    }
    if let Some(r) = &file.required_version
        && !satisfies(r).map_err(issue)?
    {
        return Err(issue(format!(
            "This is origami {}, but {} requires `{r}`",
            env!("CARGO_PKG_VERSION"),
            path.display()
        )));
    }
    if let Some(w) = file.line_width {
        o.lang.line_width = w;
    }
    if let Some(i) = file.indent_width {
        o.lang.indent_width = i;
    }
    if let Some(p) = file.preview {
        o.lang.preview = p;
    }
    if let Some(s) = &file.style {
        o.lang.style = Style::parse(s).ok_or_else(|| {
            issue(format!(
                "`style` must be \"aligned\" or \"compact\", got `{s}`"
            ))
        })?;
    }
    if let Some(k) = &file.keyword_case {
        o.lang.abap_keyword_case = match k.as_str() {
            "upper" => Case::Upper,
            "lower" => Case::Lower,
            _ => {
                return Err(issue(format!(
                    "`abap.keyword_case` must be \"upper\" or \"lower\", got `{k}`"
                )));
            }
        };
    }
    apply_flags(&mut o, args);
    o.validate().map_err(|d| issue(d.message))?;
    Ok(o)
}

/// Checks the option flags on their own, so that an error names the flag.
pub fn check_flags(args: &Args) -> Result<(), String> {
    let check = |flag: &str, value: String, set: &dyn Fn(&mut Options)| {
        let mut o = Options::default();
        set(&mut o);
        o.validate()
            .map_err(|d| format!("{flag} {value} is out of range ({})", d.message))
    };
    if let Some(w) = args.line_width {
        check("--line-width", w.to_string(), &|o| o.lang.line_width = w)?;
    }
    if let Some(i) = args.indent_width {
        check("--indent-width", i.to_string(), &|o| {
            o.lang.indent_width = i
        })?;
    }
    Ok(())
}

pub fn apply_flags(o: &mut Options, args: &Args) {
    if let Some(w) = args.line_width {
        o.lang.line_width = w;
    }
    if let Some(i) = args.indent_width {
        o.lang.indent_width = i;
    }
    if args.preview {
        o.lang.preview = true;
    }
    if let Some(s) = args.style {
        o.lang.style = match s {
            StyleArg::Aligned => Style::Aligned,
            StyleArg::Compact => Style::Compact,
        };
    }
    if let Some(k) = args.keyword_case {
        o.lang.abap_keyword_case = match k {
            CaseArg::Upper => Case::Upper,
            CaseArg::Lower => Case::Lower,
        };
    }
}

/// Finds and caches `origami.toml` files.
#[derive(Default)]
pub struct Resolver {
    by_dir: Mutex<HashMap<PathBuf, Option<PathBuf>>>,
}

impl Resolver {
    /// Walks up from `dir`, checking each directory including the one that
    /// contains `.git`, and stops after it.
    pub fn discover(&self, dir: &Path) -> Option<PathBuf> {
        if let Some(hit) = self
            .by_dir
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(dir)
        {
            return hit.clone();
        }
        let mut found = None;
        let mut cur = Some(dir);
        while let Some(d) = cur {
            let candidate = d.join(FILE_NAME);
            if candidate.is_file() {
                found = Some(candidate);
                break;
            }
            if d.join(".git").exists() {
                break;
            }
            cur = d.parent();
        }
        self.by_dir
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(dir.to_path_buf(), found.clone());
        found
    }
}
