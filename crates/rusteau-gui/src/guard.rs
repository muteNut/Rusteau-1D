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

/// Periodic copy of an unsaved project: one file with the latest state, written only when the project changed
/// since the last copy, removed when the project is saved (or has no unsaved changes) and when the program closes.
pub struct Recovery {
    last: Instant,
    /// hash of the project in the recovery file of this session (`None`: no file of this session)
    written: Option<u64>,
    /// folder of the recovery file (the user data; another folder in tests)
    dir: PathBuf,
}

impl Default for Recovery {
    fn default() -> Self {
        Recovery { last: Instant::now(), written: None, dir: data_dir() }
    }
}

impl Recovery {
    const EVERY: Duration = Duration::from_secs(60);

    pub fn file(&self) -> PathBuf {
        self.dir.join("recovery.h1dr")
    }

    /// Called every frame: writes the project when it has unsaved changes that are not in the copy yet (at most once
    /// a minute); removes the copy when there are no unsaved changes.
    pub fn tick(&mut self, prj: &rusteau_core::Project, dirty: bool) {
        if !dirty {
            if self.written.is_some() {
                self.discard();
            }
            return;
        }
        if self.last.elapsed() < Self::EVERY {
            return;
        }
        self.last = Instant::now();
        self.write(prj);
    }

    /// Writes the project unless the copy already holds this state.
    pub fn write(&mut self, prj: &rusteau_core::Project) {
        let Ok(text) = serde_json::to_string_pretty(prj) else { return };
        let hash = {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            text.hash(&mut h);
            h.finish()
        };
        if self.written == Some(hash) && self.file().exists() {
            return;
        }
        let _ = std::fs::create_dir_all(&self.dir);
        // written next to it first and renamed: the file always holds one complete, latest copy
        let tmp = self.file().with_extension("tmp");
        if std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, self.file()).is_ok() {
            self.written = Some(hash);
        }
    }

    /// Removes the recovery copy (the project was saved, or the program closes normally).
    pub fn discard(&mut self) {
        let _ = std::fs::remove_file(self.file());
        self.written = None;
    }
}

/// A recovery copy left by a session that did not end normally: its path and age.
pub fn pending_recovery() -> Option<(PathBuf, Duration)> {
    let p = Recovery::default().file();
    let age = std::fs::metadata(&p).ok()?.modified().ok()?.elapsed().unwrap_or_default();
    Some((p, age))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_keeps_the_latest_changed_state_and_is_cleared_by_saving() {
        let dir = std::env::temp_dir().join(format!("rusteau-recovery-{}", std::process::id()));
        let mut r = Recovery { last: Instant::now(), written: None, dir: dir.clone() };
        let mut p = rusteau_core::examples::infiltration_evaporation();
        // within a minute of the last copy nothing is written
        r.tick(&p, true);
        assert!(!r.file().exists());
        r.write(&p);
        let t0 = std::fs::metadata(r.file()).unwrap().modified().unwrap();
        // the same state again: the file is not rewritten
        std::thread::sleep(Duration::from_millis(20));
        r.write(&p);
        assert_eq!(std::fs::metadata(r.file()).unwrap().modified().unwrap(), t0);
        // a change replaces the copy (one file with the latest state)
        p.title = "changed".into();
        r.write(&p);
        assert_eq!(rusteau_io::load_project(&r.file()).unwrap().title, "changed");
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        // saved (no unsaved changes): the copy is removed
        r.tick(&p, false);
        assert!(!r.file().exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn panics_become_errors() {
        assert_eq!(catch("x", || 2 + 2), Ok(4));
        let e = catch("Drawing", || -> i32 { panic!("index out of bounds") }).unwrap_err();
        assert!(e.starts_with("Drawing: internal error (index out of bounds)"), "{e}");
        let e = catch("Drawing", || -> i32 { panic!("{}", String::from("owned")) }).unwrap_err();
        assert!(e.contains("(owned)"));
    }
}
