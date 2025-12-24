pub struct QueryValidator;

impl QueryValidator {
    pub fn validate_query(query: &str) -> anyhow::Result<()> {
        if query.trim().is_empty() {
            return Err(anyhow::anyhow!("Query cannot be empty"));
        }

        let query_upper = query.to_uppercase();

        if query_upper.contains("DROP")
            || query_upper.contains("DELETE")
            || query_upper.contains("UPDATE")
            || query_upper.contains("INSERT")
            || query_upper.contains("CREATE")
            || query_upper.contains("ALTER")
            || query_upper.contains("TRUNCATE")
        {
            return Err(anyhow::anyhow!(
                "Write operations are not allowed. Only SELECT queries and PRAGMA statements are permitted."
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_query() {
        assert!(QueryValidator::validate_query("SELECT * FROM test").is_ok());
        assert!(QueryValidator::validate_query("PRAGMA table_info(test)").is_ok());
        assert!(QueryValidator::validate_query("").is_err());
        assert!(QueryValidator::validate_query("DROP TABLE test").is_err());
        assert!(QueryValidator::validate_query("DELETE FROM test").is_err());
    }
}
