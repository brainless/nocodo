mod common;
use common::*;

#[tokio::test(flavor = "current_thread")]
#[ignore]
async fn live_diesel_model_struct_generates_code() {
    let _guard = LIVE_TEST_LOCK.lock().expect("live test lock poisoned");
    let cfg = LiveTestConfig::from_env();
    recreate_project_from_template(&cfg);

    for model in cfg.models() {
        let agent = agent_for_model(&cfg, model);
        let output = agent
            .diesel_model_struct(
                r#"Write a Diesel SQLite read model struct named ContactRecord for table user_contacts.
Fields: id BigInt primary key, user_id BigInt, contact_type Text, value Text,
country_code nullable Integer, verified_at nullable Timestamp, created_at Timestamp."#,
            )
            .await
            .unwrap_or_else(|e| panic!("model `{model}` diesel_model_struct failed: {e}"));

        let code = output
            .code
            .as_deref()
            .unwrap_or_else(|| panic!("model `{model}` returned no extracted code"));

        assert_not_empty(model, output.raw_response.as_str(), "raw_response");
        assert_not_empty(model, code, "code");
        assert_clean_code(model, code);
        assert_no_imports(model, code);
        assert!(
            code.contains("pub struct ContactRecord"),
            "model `{model}` did not generate requested struct:\n{code}"
        );
    }
}
