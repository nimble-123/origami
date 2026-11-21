//! `origami`: layout-only formatter for ABAP and the CDS family.

mod args;
mod config;
mod io;
mod report;
mod run;
mod walk;

use clap::Parser;
use clap::error::ErrorKind;

fn main() {
    // The core catches panics and reports X002. The default hook would print
    // to stderr and corrupt JSON diagnostics there, so this hook prints
    // nothing and only remembers where the panic happened.
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()));
        run::LAST_PANIC.with(|p| *p.borrow_mut() = loc);
    }));

    let args = match args::Args::try_parse() {
        Ok(a) => a,
        Err(e) => {
            let code = match e.kind() {
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => 0,
                _ => 3,
            };
            let _ = e.print();
            std::process::exit(code);
        }
    };
    std::process::exit(run::run(&args));
}
