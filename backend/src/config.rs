use std::{fs, path::Path, path::PathBuf};

fn read_conf_file(path: &Path, key: &str) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, value)) = line.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|p| p.to_path_buf())
}

pub fn read_project_conf(key: &str) -> Option<String> {
    let mut candidates = vec![
        PathBuf::from("project.conf"),
        PathBuf::from("../project.conf"),
    ];
    if let Some(dir) = exe_dir() {
        candidates.push(dir.join("server.env"));
    }
    candidates.iter().find_map(|p| read_conf_file(p, key))
}
