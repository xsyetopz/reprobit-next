//! Human-facing progress on stderr. Results go to stdout through `main`, so
//! piping `--json` output never mixes in progress lines.
#![expect(
    clippy::print_stderr,
    reason = "this module is the one place progress is written"
)]

use std::sync::atomic::{AtomicBool, Ordering};

static QUIET: AtomicBool = AtomicBool::new(false);

/// Silences [`note`] for the rest of the process.
pub(crate) fn set_quiet(quiet: bool) {
    QUIET.store(quiet, Ordering::Relaxed);
}

/// Reports one step of long-running work.
pub(crate) fn note(message: &str) {
    if !QUIET.load(Ordering::Relaxed) {
        eprintln!("{message}");
    }
}

/// Reports an error, whatever `--quiet` says.
pub(crate) fn error(message: &str) {
    eprintln!("error: {message}");
}

/// Where a child process's chatter goes: stderr, so stdout stays reserved
/// for results and the MCP stream.
pub(crate) fn child_output() -> std::process::Stdio {
    std::io::stderr().into()
}
