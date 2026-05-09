pub fn normalize_backend_context_json(raw: &str) -> String {
    let mut parsed: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => return raw.to_string(),
    };
    if let serde_json::Value::Object(map) = &mut parsed {
        map.remove("dependencies");
        map.remove("database");
    }
    serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| raw.to_string())
}
