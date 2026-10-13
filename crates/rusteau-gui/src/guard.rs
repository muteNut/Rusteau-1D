//! Protection against internal errors: panics of the calculation threads and of the drawing of pages and figures are
//! turned into error messages (the program keeps running), every panic is written to a crash log, and an unsaved
//! project is copied to a recovery file now and then, which the next start offers to open after a crash.

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Folder of the crash log and the recovery copy (per user; created when needed).
pub fn data_dir() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
    };
    base.unwrap_or_else(std::env::temp_dir).join("Rusteau-1D")
}

pub fn crash_log() -> PathBuf {
    data_dir().join("crash.log")
}

pub fn recovery_file() -> PathBuf {
    data_dir().join("recovery.h1dr")
}

/// The text of a panic payload.
fn payload_text(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| e.downcast_ref::<String>().cloned()).unwrap_or_else(|| "unknown error".into())
}

/// Runs `f`; a panic becomes the error "`what`: internal error (…)".
pub fn catch<T>(what: &str, f: impl FnOnce() -> T) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|e| format!("{what}: internal error ({}); details in {}", payload_text(e.as_ref()), crash_log().display()))
}

/// Writes every panic with its place and a backtrace to the crash log (and to the standard error as before).
pub fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // the same error again soon (e.g. a page drawn every frame) is written once
        static LAST: std::sync::Mutex<Option<(String, Instant)>> = std::sync::Mutex::new(None);
        let key = info.to_string();
        if let Ok(mut last) = LAST.lock() {
            if last.as_ref().is_some_and(|(k, t)| *k == key && t.elapsed() < Duration::from_secs(30)) {
                return;
            }
            *last = Some((key, Instant::now()));
        }
        let bt = std::backtrace::Backtrace::force_capture();
        let when = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let text = format!(
            "Rusteau-1D {} – internal error (unix time {when})\n{info}\nthread: {}\n\nbacktrace:\n{bt}\n\n",
            env!("CARGO_PKG_VERSION"),
            std::thread::current().name().unwrap_or("?")
        );
        let _ = std::fs::create_dir_all(data_dir());
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(crash_log()) {
            use std::io::Write;
            let _ = f.write_all(text.as_bytes());
        }
        default(info);
    }));
}

/// Periodic copy of an unsaved project.
pub struct Recovery {
    last: Instant,
    /// a recovery file of this session exists
    written: bool,
}

impl Default for Recovery {
    fn default() -> Self {
        Recovery { last: Instant::now(), written: false }
    }
}

impl Recovery {
    const EVERY: Duration = Duration::from_secs(60);

    /// Writes the project when it has unsaved changes (at most once a minute); removes the copy when there are none.
    pub fn tick(&mut self, prj: &rusteau_core::Project, dirty: bool) {
        if !dirty {
            if self.written {
                self.discard();
            }
            return;
        }
        if self.last.elapsed() < Self::EVERY {
            return;
        }
        self.last = Instant::now();
        let _ = std::fs::create_dir_all(data_dir());
        // written next to it first, so a crash while writing keeps the previous copy
        let tmp = recovery_file().with_extension("tmp");
        if rusteau_io::save_project(prj, &tmp).is_ok() && std::fs::rename(&tmp, recovery_file()).is_ok() {
            self.written = true;
        }
    }

    /// Removes the recovery copy (the project was saved or the program closes normally).
    pub fn discard(&mut self) {
        let _ = std::fs::remove_file(recovery_file());
        self.written = false;
    }
}

/// A recovery copy left by a session that did not end normally: its path and age.
pub fn pending_recovery() -> Option<(PathBuf, Duration)> {
    let p = recovery_file();
    let age = std::fs::metadata(&p).ok()?.modified().ok()?.elapsed().unwrap_or_default();
    Some((p, age))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panics_become_errors() {
        assert_eq!(catch("x", || 2 + 2), Ok(4));
        let e = catch("Drawing", || -> i32 { panic!("index out of bounds") }).unwrap_err();
        assert!(e.starts_with("Drawing: internal error (index out of bounds)"), "{e}");
        let e = catch("Drawing", || -> i32 { panic!("{}", String::from("owned")) }).unwrap_err();
        assert!(e.contains("(owned)"));
    }
}
