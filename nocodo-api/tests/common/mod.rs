use actix_web::{test, web, App};
use nocodo_agents::database::Database;
use nocodo_api::handlers::agent_execution::execute_sqlite_agent;
use nocodo_api::handlers::sessions::get_session;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::error::LlmError;
use nocodo_llm_sdk::tools::ToolCall;
use nocodo_llm_sdk::types::{CompletionRequest, CompletionResponse, ContentBlock, Role, Usage};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tempfile::NamedTempFile;

pub type DbConnection = Arc<Mutex<Connection>>;

pub struct TestApp<S> {
    pub db_conn: DbConnection,
    pub db: Arc<Database>,
    pub mock_llm_client: Arc<MockLlmClient>,
    pub app: S,
}

pub struct MockLlmClient {
    pub responses: Arc<Mutex<Vec<CompletionResponse>>>,
    pub call_count: Arc<Mutex<usize>>,
}

impl MockLlmClient {
    pub fn new() -> Self {
        MockLlmClient {
            responses: Arc::new(Mutex::new(Vec::new())),
            call_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn with_responses(responses: Vec<CompletionResponse>) -> Self {
        let client = MockLlmClient {
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(Mutex::new(0)),
        };
        client
    }

    pub fn push_response(&self, response: CompletionResponse) {
        let mut responses = self.responses.lock().unwrap();
        responses.push(response);
    }

    pub fn get_call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

impl Default for MockLlmClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let mut call_count = self.call_count.lock().unwrap();
        *call_count += 1;
        drop(call_count);

        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            Ok(CompletionResponse {
                content: vec![ContentBlock::Text {
                    text: "There are 5 users in database.".to_string(),
                }],
                role: Role::Assistant,
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 20,
                },
                stop_reason: Some("end_turn".to_string()),
                tool_calls: None,
            })
        } else {
            Ok(responses.remove(0))
        }
    }

    fn provider_name(&self) -> &str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }
}

pub fn setup_test_db() -> anyhow::Result<Database> {
    Database::new(&PathBuf::from(":memory:"))
}

pub fn setup_test_sqlite_db_with_data() -> anyhow::Result<(NamedTempFile, String)> {
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file
        .path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert path to string"))?
        .to_string();

    let conn = Connection::open(&db_path)?;

    conn.execute(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    let users = vec![
        ("Alice Johnson", "alice@example.com", "2024-01-15 10:30:00"),
        ("Bob Smith", "bob@example.com", "2024-02-20 14:15:00"),
        (
            "Charlie Brown",
            "charlie@example.com",
            "2024-03-10 09:00:00",
        ),
        ("Diana Prince", "diana@example.com", "2024-04-05 16:45:00"),
        ("Eve Anderson", "eve@example.com", "2024-05-12 11:20:00"),
    ];

    for (name, email, created_at) in users {
        conn.execute(
            "INSERT INTO users (name, email, created_at) VALUES (?, ?, ?)",
            [name, email, created_at],
        )?;
    }

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    assert_eq!(count, 5, "Expected 5 users to be inserted");

    Ok((temp_file, db_path))
}

pub fn create_mock_llm_client() -> Arc<MockLlmClient> {
    Arc::new(MockLlmClient::new())
}

pub async fn setup_test_app() -> anyhow::Result<TestApp<impl actix_web::dev::Service<
    actix_http::Request,
    Response = actix_web::dev::ServiceResponse,
    Error = actix_web::Error,
>>> {
    let db = Arc::new(setup_test_db()?);
    let db_conn = db.connection.clone();
    let mock_llm_client = create_mock_llm_client();
    let llm_client = mock_llm_client.clone() as Arc<dyn LlmClient>;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(llm_client))
            .app_data(web::Data::new(db_conn.clone()))
            .app_data(web::Data::new(db.clone()))
            .service(execute_sqlite_agent)
            .service(get_session),
    )
    .await;

    Ok(TestApp {
        db_conn,
        db,
        mock_llm_client,
        app,
    })
}

pub fn create_completion_response_with_tool_call(
    tool_name: &str,
    tool_args: serde_json::Value,
) -> CompletionResponse {
    CompletionResponse {
        content: vec![],
        role: Role::Assistant,
        usage: Usage {
            input_tokens: 10,
            output_tokens: 20,
        },
        stop_reason: Some("tool_use".to_string()),
        tool_calls: Some(vec![ToolCall::new(
            "call_123".to_string(),
            tool_name.to_string(),
            tool_args,
        )]),
    }
}

pub fn create_completion_response_with_text(text: &str) -> CompletionResponse {
    CompletionResponse {
        content: vec![ContentBlock::Text {
            text: text.to_string(),
        }],
        role: Role::Assistant,
        usage: Usage {
            input_tokens: 10,
            output_tokens: 20,
        },
        stop_reason: Some("end_turn".to_string()),
        tool_calls: None,
    }
}
