//! A deliberately crude append-only log the human can read on the gaming PC.
//!
//! **Scope note.** This is not the diagnostics surface.
//! [Decide how to debug on a Windows box with no dev environment](https://github.com/Furizaa/poe-graft/issues/11)
//! owns log levels, volume, rotation, capture/replay and self-test. What is here is the minimum
//! the bootstrap needs: somewhere the updater's story lands so a failed round trip is
//! distinguishable from a build that never happened. Keep it dumb until that ticket says
//! otherwise.
//!
//! Failures are swallowed on purpose. A log that cannot be written must never take the app down
//! with it.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// How many lines the UI is willing to show. The file itself is never truncated — at this
/// volume it will not matter, and rotation is #11's call.
const TAIL_LIMIT: usize = 300;

/// An append-only text log at a fixed path.
pub struct Journal {
    path: PathBuf,
    /// Serialises appends so two threads cannot interleave halves of a line.
    write_lock: Mutex<()>,
}

impl Journal {
    /// Point a journal at `path`, creating its directory if need be.
    pub fn new(path: PathBuf) -> Self {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        Self {
            path,
            write_lock: Mutex::new(()),
        }
    }

    /// Where the file lives, for the UI to show and for "open containing folder".
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append one timestamped line. Best effort.
    pub fn append(&self, line: &str) {
        let stamped = format!(
            "{} {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
            line
        );

        // A poisoned lock means another thread panicked mid-append. The log is not worth
        // propagating that, so take the guard either way.
        let _guard = match self.write_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = file.write_all(stamped.as_bytes());
        }
    }

    /// The last [`TAIL_LIMIT`] lines, oldest first. An unreadable or absent file reads as empty.
    pub fn tail(&self) -> Vec<String> {
        let Ok(file) = File::open(&self.path) else {
            return Vec::new();
        };

        let mut lines: Vec<String> = Vec::new();
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            lines.push(line);
            if lines.len() > TAIL_LIMIT {
                lines.remove(0);
            }
        }
        lines
    }
}
