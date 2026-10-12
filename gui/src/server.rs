//! Starts a local `nocodo-api` as a child process and learns its address from its first line.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result, anyhow};

pub struct ServerProcess {
    child: Child,
    pub base_url: String,
}

fn server_binary() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("NOCODO_API_BIN") {
        return Ok(p.into());
    }
    let name = format!("nocodo-api{}", std::env::consts::EXE_SUFFIX);
    let path = std::env::current_exe()?
        .parent()
        .ok_or_else(|| anyhow!("no folder for the running program"))?
        .join(name);
    if !path.exists() {
        return Err(anyhow!(
            "{} not found. Run `cargo build --workspace` first, or point the GUI at a running \
             server with --server <url>.",
            path.display()
        ));
    }
    Ok(path)
}

/// The address in the server's startup line, e.g.
/// `nocodo-api listening on http://127.0.0.1:5123 (projects in .projects)`.
pub fn parse_listening(line: &str) -> Option<String> {
    let start = line.find("http://")?;
    let url = line[start..].split_whitespace().next()?;
    Some(url.trim_end_matches('/').to_string())
}

impl ServerProcess {
    pub fn spawn(projects_dir: &Path) -> Result<Self> {
        let bin = server_binary()?;
        let mut child = Command::new(&bin)
            .args(["--addr", "127.0.0.1:0", "--projects-dir"])
            .arg(projects_dir)
            .stdout(Stdio::piped())
            .spawn()
            .with_context(|| format!("starting {}", bin.display()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("no server output"))?;
        let mut line = String::new();
        BufReader::new(stdout).read_line(&mut line)?;
        let Some(base_url) = parse_listening(&line) else {
            let _ = child.kill();
            return Err(anyhow!(
                "The server did not say where it listens (it printed {line:?}). \
                 Run `cargo run --bin nocodo-api` to see its error."
            ));
        };
        Ok(Self { child, base_url })
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_address_from_the_startup_line() {
        assert_eq!(
            parse_listening(
                "nocodo-api listening on http://127.0.0.1:5123 (projects in .projects)\n"
            ),
            Some("http://127.0.0.1:5123".into())
        );
        assert_eq!(parse_listening("Could not listen"), None);
    }
}
