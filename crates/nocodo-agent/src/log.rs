use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde_json::{Value, json};

use crate::{Error, Result};

/// Appends JSONL records to `<project>/.agent-logs/<run_id>.jsonl`.
///
/// Every line carries `run_id`, `role`, `model`, `at` and `type`, plus the
/// fields given to [`AgentLog::append`]. Clones share the same file and error
/// slot, so a hook can write while the caller keeps a handle.
#[derive(Debug, Clone)]
pub struct AgentLog {
    path: PathBuf,
    run_id: String,
    role: String,
    model: String,
    failure: Arc<Mutex<Option<String>>>,
}

impl AgentLog {
    pub fn new(project_dir: &Path, run_id: &str, role: &str, model: &str) -> Self {
        Self {
            path: project_dir
                .join(".agent-logs")
                .join(format!("{run_id}.jsonl")),
            run_id: run_id.to_string(),
            role: role.to_string(),
            model: model.to_string(),
            failure: Arc::default(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends one line of type `kind`. `fields` must be a JSON object (or
    /// `Null` for none). Write failures are remembered and reported by
    /// [`AgentLog::check`], because hooks cannot return errors.
    pub fn append(&self, kind: &str, fields: Value) {
        if let Err(message) = self.try_append(kind, fields) {
            let mut slot = self.failure.lock().unwrap_or_else(|e| e.into_inner());
            slot.get_or_insert(message);
        }
    }

    /// Returns the first write failure seen so far, if any.
    pub fn check(&self) -> Result<()> {
        let slot = self.failure.lock().unwrap_or_else(|e| e.into_inner());
        match slot.as_ref() {
            Some(message) => Err(Error::Log {
                path: self.path.clone(),
                message: message.clone(),
            }),
            None => Ok(()),
        }
    }

    fn try_append(&self, kind: &str, fields: Value) -> std::result::Result<(), String> {
        let mut line = json!({
            "run_id": self.run_id,
            "role": self.role,
            "model": self.model,
            "at": jiff::Timestamp::now().to_string(),
            "type": kind,
        });
        if let (Some(line), Value::Object(fields)) = (line.as_object_mut(), fields) {
            line.extend(fields);
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| e.to_string())?;
        let mut text = serde_json::to_string(&line).map_err(|e| e.to_string())?;
        text.push('\n');
        file.write_all(text.as_bytes()).map_err(|e| e.to_string())
    }
}
