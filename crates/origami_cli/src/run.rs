//! Execution of stdin mode and path mode, and exit-code aggregation.

use crate::args::{Args, DiagFormat};
use crate::config::{self, ConfigIssue, FileConfig, Resolver};
use crate::report::{self, FileReport, Run};
use origami_core::detect::LanguageHint;
use origami_core::{Code, Edition, Language, Options, Reported, Severity, Status, format};
use rayon::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

thread_local! {
    /// Location of the last panic on this thread, set by the panic hook.
    pub static LAST_PANIC: RefCell<Option<String>> = const { RefCell::new(None) };
}

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

pub fn run(args: &Args) -> i32 {
    if args.version {
        return version(args);
    }
    if let Some(l) = &args.language
        && l != "cds"
        && Language::parse(l).is_none()
    {
        return usage_error(args, &format!("Unknown --language `{l}`"));
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

fn usage_error(args: &Args, msg: &str) -> i32 {
    let f = FileReport {
        path: "<args>".into(),
        language: None,
        changed: false,
        status: Status::Error,
        diagnostics: vec![Reported::at_start(Code::E021, msg)],
    };
    print_diagnostics(args, &[f]);
    3
}

fn version(args: &Args) -> i32 {
    let v = env!("CARGO_PKG_VERSION");
    let edition = Edition::E2026.as_str();
    if !args.json {
        println!("origami {v} (style edition {edition})");
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
    println!("{doc}");
    0
}

type Loaded = Result<(Options, FileConfig), ConfigIssue>;

/// Loads, caches and resolves options per directory.
struct Configs<'a> {
    args: &'a Args,
    resolver: Resolver,
    loaded: Mutex<HashMap<PathBuf, Loaded>>,
}

impl<'a> Configs<'a> {
    fn new(args: &'a Args) -> Configs<'a> {
        Configs {
            args,
            resolver: Resolver::default(),
            loaded: Mutex::new(HashMap::new()),
        }
    }

    /// Options for files in `dir`, plus the config file used.
    fn for_dir(&self, dir: &Path) -> Result<(Options, Option<PathBuf>), ConfigIssue> {
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
            })?;
            return Ok((o, None));
        };
        let mut cache = self.loaded.lock().expect("lock");
        let entry = cache.entry(path.clone()).or_insert_with(|| {
            let text = std::fs::read_to_string(&path).map_err(|e| ConfigIssue {
                code: Code::E020,
                path: path.clone(),
                message: format!("Cannot read config: {e}"),
            })?;
            let file = config::parse(&text, &path)?;
            let opts = config::options(&file, self.args, &path)?;
            Ok((opts, file))
        });
        entry.clone().map(|(o, _)| (o, Some(path)))
    }

    /// W101 reports for unknown keys, one per config file.
    fn warnings(&self) -> Vec<FileReport> {
        let cache = self.loaded.lock().expect("lock");
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
            })
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }
}

fn config_error(args: &Args, issue: ConfigIssue) -> i32 {
    let f = FileReport {
        path: issue.path.display().to_string(),
        language: None,
        changed: false,
        status: Status::Error,
        diagnostics: vec![Reported::at_start(issue.code, issue.message)],
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

fn annotate_panic(diags: &mut [Reported]) {
    let loc = LAST_PANIC.with(|p| p.borrow_mut().take());
    if let Some(loc) = loc {
        for d in diags.iter_mut().filter(|d| d.code == Code::X002) {
            d.message = format!("{} (at {loc})", d.message);
        }
    }
}

fn stdin_mode(args: &Args) -> i32 {
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
    let r = format(&input, &hint, &opts);
    let mut report = FileReport {
        path: name.unwrap_or_else(|| "<stdin>".into()),
        language: r.language,
        changed: r.changed,
        status: r.status,
        diagnostics: r.diagnostics,
    };
    annotate_panic(&mut report.diagnostics);
    let success = matches!(report.status, Status::Formatted | Status::Unchanged);
    if success
        && !args.check
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
        print!("{}", report::diff::unified(&report.path, &input, out));
    }
    let mut all = configs.warnings();
    all.push(report);
    print_diagnostics(args, &all);
    all.iter().fold(0, |acc, f| {
        worst(acc, exit_code(f, args.check, args.strict))
    })
}

fn path_mode(args: &Args) -> i32 {
    let roots = if args.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        args.paths.clone()
    };
    let files = crate::walk::files(&roots);
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

    let process = |path: &PathBuf| -> (FileReport, Option<String>) {
        // Paths found by walking `.` are shown without the `./` prefix.
        let display = path.strip_prefix(".").unwrap_or(path).display().to_string();
        let fail = |code: Code, msg: String| FileReport {
            path: display.clone(),
            language: None,
            changed: false,
            status: Status::Error,
            diagnostics: vec![Reported::at_start(code, msg)],
        };
        let input = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return (fail(Code::E030, format!("Cannot read file: {e}")), None),
        };
        let opts = &opts_by_dir[&dir_of(path)];
        let r = format(&input, &LanguageHint::Path(display.clone()), opts);
        let mut diagnostics = r.diagnostics;
        annotate_panic(&mut diagnostics);
        let mut diff = None;
        if let (true, Some(out)) = (r.changed, &r.output) {
            if args.check {
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
        worst(acc, exit_code(f, args.check, args.strict))
    })
}

fn print_diagnostics(args: &Args, files: &[FileReport]) {
    let run = Run {
        files,
        max_diagnostics: args.max_diagnostics,
        check: args.check,
    };
    let mut err = std::io::stderr().lock();
    let _ = match args.diagnostics {
        DiagFormat::Human => report::human::write(&run, &mut err),
        DiagFormat::Json => report::json::write(&run, &mut err),
        DiagFormat::Sarif => report::sarif::write(&run, &mut err),
    };
}
