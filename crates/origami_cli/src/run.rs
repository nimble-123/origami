//! Execution of stdin mode and path mode, and exit-code aggregation.

use crate::args::{Args, DiagFormat};
use crate::config::{self, ConfigIssue, FileConfig, Resolver};
use crate::report::{self, FileReport, Run};
use origami_core::detect::LanguageHint;
use origami_core::{
    Code, Edition, Explained, FormatResult, Language, Options, Reported, ReportedRange, Severity,
    Status,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

/// Rank for exit-code precedence: 70 > 4 > 3 > 2 > 5 > 1 > 0.
fn rank(code: i32) -> u8 {
    match code {
        70 => 6,
        4 => 5,
        3 => 4,
        2 => 3,
        5 => 2,
        1 => 1,
        _ => 0,
    }
}

fn worst(a: i32, b: i32) -> i32 {
    if rank(b) > rank(a) { b } else { a }
}

fn exit_code(f: &FileReport, check: bool, strict: bool) -> i32 {
    let mut code = if check && f.changed { 1 } else { 0 };
    for d in &f.diagnostics {
        let c = if d.severity == Severity::Warning && strict {
            5
        } else {
            d.code.exit_code()
        };
        code = worst(code, c);
    }
    code
}

/// When the run started (for the human summary).
pub static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub fn run(args: &Args) -> i32 {
    START.get_or_init(std::time::Instant::now);
    if args.mcp {
        return crate::mcp::serve();
    }
    if args.version {
        return version(args);
    }
    if let Some(l) = &args.language
        && l != "cds"
        && Language::parse(l).is_none()
    {
        return usage_error(args, &format!("Unknown --language `{l}`"));
    }
    if let Err(message) = config::check_flags(args) {
        return config_error(
            args,
            ConfigIssue {
                code: Code::E020,
                path: PathBuf::from("<args>"),
                message,
                at: None,
                source: None,
            },
        );
    }
    if args.stdin_mode() {
        if !args.paths.is_empty() {
            return usage_error(args, "Paths cannot be combined with --stdin");
        }
        stdin_mode(args)
    } else {
        path_mode(args)
    }
}

/// Writes to stdout and ignores a failure: a reader that has gone away
/// (`| head -c1`) is a normal end, as in path mode.
fn print_stdout(text: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = stdout
        .write_all(text.as_bytes())
        .and_then(|_| stdout.flush());
}

fn usage_error(args: &Args, msg: &str) -> i32 {
    print_diagnostics(args, &[run_error("<args>", Code::E021, msg)]);
    3
}

/// A run-level error that is not tied to a file.
fn run_error(path: &str, code: Code, msg: &str) -> FileReport {
    FileReport {
        path: path.into(),
        language: None,
        changed: false,
        status: Status::Error,
        diagnostics: vec![Reported::at_start(code, msg)],
        source: None,
    }
}

/// E021 for a command line that clap rejected, in a format other than human.
pub fn report_usage_error(format: DiagFormat, msg: &str) {
    write_diagnostics(format, 200, false, &[run_error("<args>", Code::E021, msg)]);
}

/// A panic outside the core: X002 in the requested format, exit 70.
pub fn internal_error(args: &Args, msg: &str) -> i32 {
    let f = run_error("<internal>", Code::X002, &format!("Internal error: {msg}"));
    print_diagnostics(args, &[f]);
    70
}

fn version(args: &Args) -> i32 {
    let v = env!("CARGO_PKG_VERSION");
    let edition = Edition::E2026.as_str();
    if !args.json {
        print_stdout(&format!("origami {v} (style edition {edition})\n"));
        return 0;
    }
    let (required, satisfies) = match &args.config {
        Some(p) => match std::fs::read_to_string(p)
            .map_err(|e| e.to_string())
            .and_then(|t| config::parse(&t, p).map_err(|i| i.message))
        {
            Ok(c) => {
                let s = c.required_version.as_deref().map(config::satisfies);
                match s {
                    Some(Err(e)) => {
                        eprintln!("origami: {e}");
                        return 3;
                    }
                    Some(Ok(b)) => (c.required_version.clone(), Some(b)),
                    None => (None, None),
                }
            }
            Err(e) => {
                eprintln!("origami: {e}");
                return 3;
            }
        },
        None => (None, None),
    };
    let doc = serde_json::json!({
        "version": v,
        "style_edition": edition,
        "json_schema": 1,
        "required_version": required,
        "satisfies_required_version": satisfies,
    });
    print_stdout(&format!("{doc}\n"));
    0
}

type Loaded = Result<(Options, FileConfig), ConfigIssue>;

/// Loads, caches and resolves options per directory.
pub(crate) struct Configs<'a> {
    args: &'a Args,
    resolver: Resolver,
    loaded: Mutex<HashMap<PathBuf, Loaded>>,
}

impl<'a> Configs<'a> {
    pub(crate) fn new(args: &'a Args) -> Configs<'a> {
        Configs {
            args,
            resolver: Resolver::default(),
            loaded: Mutex::new(HashMap::new()),
        }
    }

    /// Options for files in `dir`, plus the config file used.
    pub(crate) fn for_dir(&self, dir: &Path) -> Result<(Options, Option<PathBuf>), ConfigIssue> {
        let path = if self.args.no_config {
            None
        } else if let Some(p) = &self.args.config {
            Some(p.clone())
        } else {
            self.resolver.discover(dir)
        };
        let Some(path) = path else {
            let mut o = Options::default();
            config::apply_flags(&mut o, self.args);
            o.validate().map_err(|d| ConfigIssue {
                code: Code::E020,
                path: PathBuf::from("<args>"),
                message: d.message,
                at: None,
                source: None,
            })?;
            return Ok((o, None));
        };
        let mut cache = self.loaded.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = cache.entry(path.clone()).or_insert_with(|| {
            let text = std::fs::read_to_string(&path).map_err(|e| ConfigIssue {
                code: Code::E020,
                path: path.clone(),
                message: format!("Cannot read config: {e}"),
                at: None,
                source: None,
            })?;
            let file = config::parse(&text, &path)?;
            let opts = config::options(&file, self.args, &path)?;
            Ok((opts, file))
        });
        entry.clone().map(|(o, _)| (o, Some(path)))
    }

    /// W101 reports for unknown keys, one per config file.
    fn warnings(&self) -> Vec<FileReport> {
        let cache = self.loaded.lock().unwrap_or_else(PoisonError::into_inner);
        let mut out: Vec<FileReport> = cache
            .iter()
            .filter_map(|(p, r)| r.as_ref().ok().map(|(_, f)| (p, f)))
            .filter(|(_, f)| !f.unknown_keys.is_empty())
            .map(|(p, f)| FileReport {
                path: p.display().to_string(),
                language: None,
                changed: false,
                status: Status::Skipped,
                diagnostics: f
                    .unknown_keys
                    .iter()
                    .map(|k| Reported::at_start(Code::W101, format!("Unknown config key `{k}`")))
                    .collect(),
                source: None,
            })
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }
}

fn config_error(args: &Args, issue: ConfigIssue) -> i32 {
    let mut d = Reported::at_start(issue.code, issue.message);
    if let Some(p) = issue.at {
        d.range = ReportedRange { start: p, end: p };
    }
    let f = FileReport {
        path: issue.path.display().to_string(),
        language: None,
        changed: false,
        status: Status::Error,
        diagnostics: vec![d],
        source: issue.source,
    };
    print_diagnostics(args, &[f]);
    3
}

fn hint_from_args(args: &Args) -> Option<LanguageHint> {
    args.language.as_deref().map(|l| match l {
        "cds" => LanguageHint::CdsFamily,
        other => LanguageHint::Explicit(Language::parse(other).expect("validated")),
    })
}

/// `format_lines` of the core, with the diagnostics completed for users.
pub(crate) fn format_source(
    input: &[u8],
    hint: &LanguageHint,
    opts: &Options,
    lines: &[(u32, u32)],
) -> FormatResult {
    let mut r = crate::panic::in_core(|| origami_core::format_lines(input, hint, opts, lines));
    complete(input, &mut r.diagnostics);
    r
}

/// `explain` of the core, with the diagnostics completed for users.
fn explain_source(
    input: &[u8],
    hint: &LanguageHint,
    opts: &Options,
    lines: &[(u32, u32)],
) -> Explained {
    let mut e = crate::panic::in_core(|| origami_core::explain(input, hint, opts, lines));
    complete(input, &mut e.result.diagnostics);
    e
}

/// Adds the panic location to X002 and the position of the first invalid
/// byte to E005 (the core reports E005 at the start).
fn complete(input: &[u8], diags: &mut [Reported]) {
    if let Some(loc) = crate::panic::take_core_panic() {
        for d in diags.iter_mut().filter(|d| d.code == Code::X002) {
            d.message = format!("{} (at {loc})", d.message);
        }
    }
    if diags.iter().any(|d| d.code == Code::E005) {
        let body = input.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(input);
        if let Err(e) = std::str::from_utf8(body) {
            let valid = std::str::from_utf8(&body[..e.valid_up_to()]).unwrap_or_default();
            let p = report::position(valid, valid.len());
            for d in diags.iter_mut().filter(|d| d.code == Code::E005) {
                d.range = ReportedRange { start: p, end: p };
            }
        }
    }
}

/// `--lines START:END` values as 1-based inclusive ranges.
fn parse_lines(args: &Args) -> Result<Vec<(u32, u32)>, String> {
    args.lines
        .iter()
        .map(|v| {
            let (a, b) = v
                .split_once(':')
                .ok_or(format!("--lines {v}: expected START:END"))?;
            let a: u32 = a
                .trim()
                .parse()
                .map_err(|_| format!("--lines {v}: bad start"))?;
            let b: u32 = b
                .trim()
                .parse()
                .map_err(|_| format!("--lines {v}: bad end"))?;
            if a == 0 || b < a {
                return Err(format!("--lines {v}: need 1 <= START <= END"));
            }
            Ok((a, b))
        })
        .collect()
}

fn stdin_mode(args: &Args) -> i32 {
    if args.changed.is_some() {
        return usage_error(args, "--changed works on files, not on stdin");
    }
    let ranges = match parse_lines(args) {
        Ok(r) => r,
        Err(m) => return usage_error(args, &m),
    };
    let mut input = Vec::new();
    if let Err(e) = std::io::stdin().read_to_end(&mut input) {
        eprintln!("origami: cannot read stdin: {e}");
        return 4;
    }
    let name = args.stdin_filepath.clone();
    let dir = name
        .as_deref()
        .and_then(|n| Path::new(n).parent().map(Path::to_path_buf))
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let configs = Configs::new(args);
    let (opts, _) = match configs.for_dir(&dir) {
        Ok(o) => o,
        Err(issue) => return config_error(args, issue),
    };
    let hint = hint_from_args(args).unwrap_or_else(|| match &name {
        Some(n) => LanguageHint::StdinPath(n.clone()),
        None => LanguageHint::None,
    });
    let (r, view) = if args.review {
        let e = explain_source(&input, &hint, &opts, &ranges);
        let path = name.as_deref().unwrap_or("<stdin>");
        let color = report::color_enabled(report::Stream::Stdout);
        let view = report::review::render(path, &input, &e, color);
        (e.result, Some(view))
    } else {
        (format_source(&input, &hint, &opts, &ranges), None)
    };
    let report = FileReport {
        path: name.unwrap_or_else(|| "<stdin>".into()),
        language: r.language,
        changed: r.changed,
        status: r.status,
        diagnostics: r.diagnostics,
        source: (r.status == Status::Error).then(|| String::from_utf8_lossy(&input).into_owned()),
    };
    let success = matches!(report.status, Status::Formatted | Status::Unchanged);
    if success
        && !args.check_mode()
        && let Some(out) = &r.output
    {
        let mut stdout = std::io::stdout().lock();
        if stdout.write_all(out).and_then(|_| stdout.flush()).is_err() {
            return 4;
        }
    }
    if args.check
        && args.diff
        && report.changed
        && let Some(out) = &r.output
    {
        print_stdout(&report::diff::unified(&report.path, &input, out));
    }
    if let Some(view) = view.filter(|_| success) {
        print_stdout(&view);
    }
    let mut all = configs.warnings();
    all.push(report);
    print_diagnostics(args, &all);
    all.iter().fold(0, |acc, f| {
        worst(acc, exit_code(f, args.check_mode(), args.strict))
    })
}

fn path_mode(args: &Args) -> i32 {
    let roots = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths.clone()
    };
    let line_ranges = match parse_lines(args) {
        Ok(r) => r,
        Err(m) => return usage_error(args, &m),
    };
    // Files with the line ranges to format (`None`: the whole file).
    let selected: Vec<(PathBuf, crate::changed::Ranges)> = if let Some(rev) = &args.changed {
        match crate::changed::files(rev, &args.paths) {
            Ok(f) => f,
            Err(m) => return usage_error(args, &m),
        }
    } else if !line_ranges.is_empty() {
        if args.paths.len() != 1 || !args.paths[0].is_file() {
            return usage_error(args, "--lines needs stdin mode or exactly one file");
        }
        vec![(args.paths[0].clone(), Some(line_ranges))]
    } else {
        crate::walk::files(&roots)
            .into_iter()
            .map(|f| (f, None))
            .collect()
    };
    let ranges_of: HashMap<PathBuf, crate::changed::Ranges> = selected.iter().cloned().collect();
    let files: Vec<PathBuf> = selected.into_iter().map(|(f, _)| f).collect();
    let configs = Configs::new(args);

    // Resolve every config before touching any file: config errors abort.
    let dir_of = |p: &Path| {
        p.parent()
            .filter(|d| !d.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };
    let mut dirs: Vec<PathBuf> = files.iter().map(|f| dir_of(f)).collect();
    dirs.sort();
    dirs.dedup();
    let mut opts_by_dir = HashMap::new();
    for d in dirs {
        match configs.for_dir(&d) {
            Ok((o, _)) => {
                opts_by_dir.insert(d, o);
            }
            Err(issue) => return config_error(args, issue),
        }
    }

    let color = report::color_enabled(report::Stream::Stdout);
    // `diff`: the unified diff (`--diff`) or the review view (`--review`).
    let process = |path: &PathBuf| -> (FileReport, Option<String>) {
        // Paths found by walking `.` are shown without the `./` prefix.
        let display = path.strip_prefix(".").unwrap_or(path).display().to_string();
        let fail = |code: Code, msg: String| FileReport {
            path: display.clone(),
            language: None,
            changed: false,
            status: Status::Error,
            diagnostics: vec![Reported::at_start(code, msg)],
            source: None,
        };
        let input = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return (fail(Code::E030, format!("Cannot read file: {e}")), None),
        };
        let opts = &opts_by_dir[&dir_of(path)];
        let ranges = ranges_of[path].as_deref().unwrap_or(&[]);
        let hint = LanguageHint::Path(display.clone());
        let mut diff = None;
        let r = if args.review {
            let e = explain_source(&input, &hint, opts, ranges);
            let view = report::review::render(&display, &input, &e, color);
            diff = Some(view).filter(|v| !v.is_empty());
            e.result
        } else {
            format_source(&input, &hint, opts, ranges)
        };
        let diagnostics = r.diagnostics;
        if let (true, Some(out)) = (r.changed, &r.output) {
            if args.check_mode() {
                if args.diff {
                    diff = Some(report::diff::unified(&display, &input, out));
                }
            } else if let Err(e) = crate::io::write_atomic(path, out) {
                return (fail(Code::E031, format!("Cannot write file: {e}")), None);
            }
        }
        let report = FileReport {
            path: display,
            language: r.language,
            changed: r.changed,
            status: r.status,
            diagnostics,
            source: (r.status == Status::Error)
                .then(|| String::from_utf8_lossy(&input).into_owned()),
        };
        (report, diff)
    };

    let results: Vec<(FileReport, Option<String>)> = match args.jobs {
        Some(n) => match rayon::ThreadPoolBuilder::new()
            .num_threads(n.max(1))
            .build()
        {
            Ok(pool) => pool.install(|| files.par_iter().map(process).collect()),
            Err(_) => files.iter().map(process).collect(),
        },
        None => files.par_iter().map(process).collect(),
    };

    {
        let mut stdout = std::io::stdout().lock();
        for d in results.iter().filter_map(|(_, d)| d.as_ref()) {
            let _ = stdout.write_all(d.as_bytes());
        }
    }
    let mut reports = configs.warnings();
    reports.extend(results.into_iter().map(|(r, _)| r));
    print_diagnostics(args, &reports);
    reports.iter().fold(0, |acc, f| {
        worst(acc, exit_code(f, args.check_mode(), args.strict))
    })
}

fn print_diagnostics(args: &Args, files: &[FileReport]) {
    write_diagnostics(
        args.diag_format(),
        args.max_diagnostics,
        args.check_mode(),
        files,
    );
}

fn write_diagnostics(
    format: DiagFormat,
    max_diagnostics: usize,
    check: bool,
    files: &[FileReport],
) {
    let run = Run {
        files,
        max_diagnostics,
        check,
    };
    let mut err = std::io::stderr().lock();
    let _ = match format {
        DiagFormat::Human => report::human::write(&run, &mut err),
        DiagFormat::Json => report::json::write(&run, &mut err),
        DiagFormat::Sarif => report::sarif::write(&run, &mut err),
        DiagFormat::Agent => report::agent::write(&run, &mut err),
    };
}
