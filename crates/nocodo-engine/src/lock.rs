//! Per-project write lock. A lock file in the project folder is held by every
//! writing operation. It is an OS file lock, so it is released when the guard
//! drops or the owning process dies; a lock left by a dead process is
//! therefore never in the way. The file's text says who holds it, for the
//! "busy" message.

use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::EngineError;

/// Lock file name inside the project folder.
pub const LOCK_FILE: &str = ".lock";

/// Held while a writing operation runs; released on drop.
#[derive(Debug)]
pub struct WriteLock {
    file: File,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl WriteLock {
    /// Take the lock for `operation` (for example `"prompt"`), or fail with
    /// `EngineError::Busy`.
    pub fn acquire(dir: &Path, operation: &str) -> Result<Self, EngineError> {
        if !dir.is_dir() {
            return Err(
                nocodo_orchestrator::OrchestratorError::NotAProject(dir.to_path_buf()).into(),
            );
        }
        let path = dir.join(LOCK_FILE);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| EngineError::Message(format!("opening {}: {e}", path.display())))?;
        match file.try_lock() {
            Ok(()) => {
                let info = format!("{}\n{}\n{}\n", operation, std::process::id(), now_secs());
                file.set_len(0)
                    .and_then(|()| file.rewind())
                    .and_then(|()| file.write_all(info.as_bytes()))
                    .map_err(|e| {
                        EngineError::Message(format!("writing {}: {e}", path.display()))
                    })?;
                Ok(Self { file })
            }
            Err(TryLockError::WouldBlock) => {
                let mut text = String::new();
                let _ = file.read_to_string(&mut text);
                let mut lines = text.lines();
                let running = lines.next().filter(|s| !s.is_empty()).map(str::to_string);
                let pid = lines.next().and_then(|s| s.parse().ok());
                let since = lines
                    .next()
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|t| now_secs().saturating_sub(t));
                Err(EngineError::Busy {
                    running,
                    pid,
                    since_secs: since,
                })
            }
            Err(TryLockError::Error(e)) => Err(EngineError::Message(format!(
                "locking {}: {e}",
                path.display()
            ))),
        }
    }
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        let _ = self.file.set_len(0);
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nocodo-lock-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn second_writer_is_refused_and_message_says_what_runs() {
        let dir = temp_dir("busy");
        let first = WriteLock::acquire(&dir, "prompt").unwrap();
        let err = WriteLock::acquire(&dir, "apply").unwrap_err();
        assert!(matches!(err, EngineError::Busy { .. }));
        let msg = err.to_string();
        assert!(msg.contains("busy"), "{msg}");
        assert!(msg.contains("`prompt` is running"), "{msg}");
        assert!(msg.contains(&std::process::id().to_string()), "{msg}");
        assert!(msg.contains("try again"), "{msg}");
        drop(first);
        // Released on drop.
        WriteLock::acquire(&dir, "apply").unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_project_folder_is_not_a_project() {
        let dir = std::env::temp_dir().join(format!("nocodo-lock-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let msg = WriteLock::acquire(&dir, "apply").unwrap_err().to_string();
        assert!(msg.contains("not a nocodo project"), "{msg}");
        assert!(!dir.exists());
    }

    #[test]
    fn reads_work_while_locked() {
        let dir = temp_dir("reads");
        let _held = WriteLock::acquire(&dir, "prompt").unwrap();
        // Reads never take the lock: reading the project folder still works.
        assert!(std::fs::read_dir(&dir).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stale_lock_from_a_dead_process_is_taken_over() {
        let dir = temp_dir("stale");
        // A lock file left behind by a process that died: text, no OS lock.
        std::fs::write(dir.join(LOCK_FILE), "prompt\n999999\n1\n").unwrap();
        let lock = WriteLock::acquire(&dir, "apply").unwrap();
        let text = std::fs::read_to_string(dir.join(LOCK_FILE)).unwrap();
        assert!(text.starts_with("apply\n"), "{text}");
        drop(lock);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
