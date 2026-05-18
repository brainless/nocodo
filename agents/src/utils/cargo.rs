use std::path::Path;

pub fn collect_cargo_dependencies(project_root: &Path, cargo_toml_relative_path: &str) -> String {
    let cargo_toml_path = project_root.join(cargo_toml_relative_path);
    let raw = match std::fs::read_to_string(&cargo_toml_path) {
        Ok(content) => content,
        Err(e) => {
            return format!(
                "Unable to read {} for deterministic dependency context: {}",
                cargo_toml_relative_path, e
            )
        }
    };

    let parsed: toml::Value = match toml::from_str(&raw) {
        Ok(value) => value,
        Err(e) => {
            return format!(
                "Unable to parse {} for deterministic dependency context: {}",
                cargo_toml_relative_path, e
            )
        }
    };

    let mut out = String::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(table) = parsed.get(section).and_then(|v| v.as_table()) else {
            continue;
        };
        let mut entries: Vec<_> = table.iter().collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        out.push_str(&format!("[{}]\n", section));
        for (name, value) in entries {
            out.push_str(&format!(
                "- {} = {}\n",
                name,
                format_dependency_value(value)
            ));
        }
        out.push('\n');
    }

    if out.trim().is_empty() {
        format!(
            "No dependency sections found in {}.",
            cargo_toml_relative_path
        )
    } else {
        out.trim_end().to_string()
    }
}

fn format_dependency_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => format!("\"{}\"", s),
        toml::Value::Table(t) => {
            let mut fields = Vec::new();
            if let Some(v) = t.get("version").and_then(|v| v.as_str()) {
                fields.push(format!("version=\"{}\"", v));
            }
            if let Some(v) = t.get("path").and_then(|v| v.as_str()) {
                fields.push(format!("path=\"{}\"", v));
            }
            if let Some(v) = t.get("git").and_then(|v| v.as_str()) {
                fields.push(format!("git=\"{}\"", v));
            }
            if let Some(v) = t.get("branch").and_then(|v| v.as_str()) {
                fields.push(format!("branch=\"{}\"", v));
            }
            if let Some(v) = t.get("tag").and_then(|v| v.as_str()) {
                fields.push(format!("tag=\"{}\"", v));
            }
            if let Some(v) = t.get("rev").and_then(|v| v.as_str()) {
                fields.push(format!("rev=\"{}\"", v));
            }
            if let Some(v) = t.get("workspace").and_then(|v| v.as_bool()) {
                fields.push(format!("workspace={}", v));
            }
            if let Some(v) = t.get("default-features").and_then(|v| v.as_bool()) {
                fields.push(format!("default-features={}", v));
            }
            if let Some(features) = t.get("features").and_then(|v| v.as_array()) {
                let vals: Vec<String> = features
                    .iter()
                    .filter_map(|f| f.as_str().map(|s| format!("\"{}\"", s)))
                    .collect();
                fields.push(format!("features=[{}]", vals.join(", ")));
            }
            if fields.is_empty() {
                "{...}".to_string()
            } else {
                format!("{{ {} }}", fields.join(", "))
            }
        }
        _ => value.to_string(),
    }
}
