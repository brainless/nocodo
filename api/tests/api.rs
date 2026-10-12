//! API tests (epic 008, Task 5): the real router on a random local port, with
//! a scripted proposer instead of a model. Requests are plain HTTP/1.1 over a
//! TCP stream, so no HTTP client dependency is needed.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use nocodo_api::{AppState, serve};
use nocodo_engine::{Project, WriteLock};
use nocodo_orchestrator::{Proposal, Proposer, ProposerError, Reply};
use nocodo_types::change::ChangeSet;
use nocodo_types::event::Actor;
use nocodo_types::question::Question;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

// ---- scripted proposer ----

#[derive(Default)]
struct Inner {
    queue: Mutex<VecDeque<Reply>>,
    /// While `true`, `propose` waits (to keep a run in progress).
    held: Mutex<bool>,
    released: Condvar,
}

/// Hands out the queued replies in order; clones share one script.
#[derive(Clone, Default)]
struct Script(Arc<Inner>);

impl Script {
    fn push(&self, reply: Reply) {
        self.0.queue.lock().unwrap().push_back(reply);
    }
    fn hold(&self) {
        *self.0.held.lock().unwrap() = true;
    }
    fn release(&self) {
        *self.0.held.lock().unwrap() = false;
        self.0.released.notify_all();
    }
}

impl Proposer for Script {
    async fn propose(
        &self,
        _current: &nocodo_types::schema::SchemaSpec,
        _prompt: &str,
        _answers: &[nocodo_orchestrator::AnsweredQuestion],
        _feedback: Option<&nocodo_orchestrator::Feedback>,
    ) -> Result<Proposal, ProposerError> {
        // Runs on the engine's own runtime thread, so blocking here is fine.
        let mut held = self.0.held.lock().unwrap();
        while *held {
            held = self.0.released.wait(held).unwrap();
        }
        drop(held);
        let reply = self
            .0
            .queue
            .lock()
            .unwrap()
            .pop_front()
            .ok_or("the script has no more replies")?;
        Ok(Proposal {
            reply,
            actor: Actor::SchemaAgent {
                run_id: "run-test".into(),
                model: "test:model".into(),
            },
        })
    }
}

fn fixture(name: &str) -> Reply {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/change-sets/finance")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap();
    let cs: ChangeSet = serde_json::from_str(&text).unwrap();
    Reply::change_set(cs, vec!["Money is in one currency.".into()])
}

fn ask(text: &str) -> Reply {
    Reply {
        questions: vec![Question {
            id: String::new(),
            asked_by: String::new(),
            text: text.into(),
            choices: vec!["Yes".into(), "No".into()],
            default: None,
            blocking: true,
            why: "test".into(),
        }],
        change_set: None,
        assumptions: vec![],
    }
}

// ---- server and client ----

struct Server {
    addr: String,
    root: PathBuf,
    script: Script,
    _dir: tempfile::TempDir,
}

async fn start() -> Server {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("projects");
    let script = Script::default();
    let s = script.clone();
    let state = AppState::new(root.clone(), move |_: &Project| s.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    tokio::spawn(serve(listener, state));
    Server {
        addr,
        root,
        script,
        _dir: dir,
    }
}

impl Server {
    async fn request(&self, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
        let body = body.map(|b| b.to_string()).unwrap_or_default();
        let req = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\
             Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            self.addr,
            body.len()
        );
        let mut stream = TcpStream::connect(&self.addr).await.unwrap();
        stream.write_all(req.as_bytes()).await.unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await.unwrap();
        let text = String::from_utf8(raw).unwrap();
        let (head, payload) = text.split_once("\r\n\r\n").unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let payload = if head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked")
        {
            dechunk(payload)
        } else {
            payload.to_string()
        };
        let json = if payload.trim().is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&payload).unwrap_or_else(|e| panic!("{e}: {payload}"))
        };
        (status, json)
    }

    async fn get(&self, path: &str) -> (u16, Value) {
        self.request("GET", path, None).await
    }

    async fn post(&self, path: &str, body: Value) -> (u16, Value) {
        self.request("POST", path, Some(body)).await
    }

    /// Start a slow call, expect `202`, wait for its run to end, return the
    /// final run record.
    async fn run(&self, path: &str, body: Value) -> Value {
        let (status, accepted) = self.post(path, body).await;
        assert_eq!(status, 202, "{accepted}");
        self.wait(path, &accepted).await
    }

    async fn wait(&self, path: &str, accepted: &Value) -> Value {
        let project = path.split('/').nth(2).unwrap();
        let id = accepted["run_id"].as_str().unwrap();
        for _ in 0..600 {
            let (status, rec) = self.get(&format!("/projects/{project}/runs/{id}")).await;
            assert_eq!(status, 200, "{rec}");
            if rec["status"] != "running" {
                return rec;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("run {id} did not finish");
    }

    async fn create(&self, name: &str) {
        let (status, body) = self.post("/projects", json!({ "name": name })).await;
        assert_eq!(status, 201, "{body}");
    }
}

fn dechunk(mut s: &str) -> String {
    let mut out = String::new();
    while let Some((size, rest)) = s.split_once("\r\n") {
        let n = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
        if n == 0 {
            break;
        }
        out.push_str(&rest[..n]);
        s = &rest[n + 2..];
    }
    out
}

fn error_text(body: &Value) -> &str {
    body["error"].as_str().unwrap_or_else(|| panic!("{body}"))
}

// ---- tests ----

#[tokio::test]
async fn init_list_prompt_questions_answer_rows() {
    let srv = start().await;

    // No projects yet (the root does not even exist).
    let (status, body) = srv.get("/projects").await;
    assert_eq!(status, 200);
    assert_eq!(body["projects"], json!([]));

    // Init, then list.
    srv.create("fin").await;
    let (status, body) = srv.post("/projects", json!({ "name": "fin" })).await;
    assert_eq!(status, 409, "{body}");
    let (status, _) = srv.post("/projects", json!({ "name": "../x" })).await;
    assert_eq!(status, 400);
    let (_, body) = srv.get("/projects").await;
    let items = body["projects"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["folder"], "fin");
    assert_eq!(items[0]["tables"], 0);
    assert!(items[0]["error"].is_null());

    // A prompt that makes the agent ask first.
    srv.script.push(ask("Do you track several accounts?"));
    srv.script.push(fixture("01-initial.json"));
    let rec = srv
        .run(
            "/projects/fin/prompt",
            json!({ "text": "track my family spending" }),
        )
        .await;
    assert_eq!(rec["status"], "finished", "{rec}");
    let (_, body) = srv.get("/projects/fin/questions").await;
    let open = body["questions"].as_array().unwrap();
    assert_eq!(open.len(), 1, "{body}");
    assert_eq!(open[0]["id"], "q1");
    assert!(
        open[0]["text"]
            .as_str()
            .unwrap()
            .contains("several accounts")
    );

    // Nothing is built while the question is open.
    let (_, body) = srv.get("/projects/fin/summary").await;
    assert_eq!(
        body["tables"].as_object().map_or(0, |t| t.len()),
        0,
        "{body}"
    );

    // Bad answers are refused at once; a wrong choice shows in the run record.
    let (status, _) = srv.post("/projects/fin/answers", json!({})).await;
    assert_eq!(status, 400);
    let (status, _) = srv.post("/projects/fin/answers", json!({ "q1": 0 })).await;
    assert_eq!(status, 400);

    // Answering the last open question resumes the run.
    let rec = srv.run("/projects/fin/answers", json!({ "q1": 1 })).await;
    assert_eq!(rec["status"], "finished", "{rec}");
    let (_, body) = srv.get("/projects/fin/questions").await;
    assert_eq!(body["questions"], json!([]));

    // The tables exist and show in the list and the summary.
    let (_, body) = srv.get("/projects").await;
    assert_eq!(body["projects"][0]["tables"], 4);
    let (_, body) = srv.get("/projects/fin/summary").await;
    assert!(body["tables"]["transactions"].is_number(), "{body}");
    assert!(!body["summary"].as_str().unwrap_or("x").is_empty());

    // Events can be polled.
    let (_, all) = srv.get("/projects/fin/events").await;
    let last = all["last"].as_u64().unwrap();
    assert!(last > 3);
    assert_eq!(all["events"].as_array().unwrap().len() as u64, last);
    let (_, none) = srv.get(&format!("/projects/fin/events?after={last}")).await;
    assert_eq!(none["events"], json!([]));
    assert_eq!(none["last"], last);
    let (_, tail) = srv
        .get(&format!("/projects/fin/events?after={}", last - 1))
        .await;
    assert_eq!(tail["events"].as_array().unwrap().len(), 1);

    // Rows: empty, then sample rows, paged.
    let (status, page) = srv.get("/projects/fin/tables/people/rows").await;
    assert_eq!(status, 200, "{page}");
    assert_eq!(page["total"], 0);
    assert_eq!(page["rows"], json!([]));
    assert!(!page["columns"].as_array().unwrap().is_empty());

    let rec = srv
        .run("/projects/fin/sample-rows", json!({ "per_table": 5 }))
        .await;
    assert_eq!(rec["status"], "finished", "{rec}");
    let (_, page) = srv.get("/projects/fin/tables/people/rows").await;
    assert_eq!(page["total"], 5);
    assert_eq!(page["rows"].as_array().unwrap().len(), 5);
    let (_, page) = srv
        .get("/projects/fin/tables/people/rows?offset=3&limit=10")
        .await;
    assert_eq!(page["rows"].as_array().unwrap().len(), 2);
    assert_eq!(page["offset"], 3);
    let (_, page) = srv
        .get("/projects/fin/tables/people/rows?offset=0&limit=2")
        .await;
    assert_eq!(page["rows"].as_array().unwrap().len(), 2);
    assert_eq!(page["total"], 5);
    let (status, _) = srv.get("/projects/fin/tables/people/rows?limit=lots").await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn destructive_change_waits_for_confirmation() {
    let srv = start().await;
    srv.create("fin").await;
    srv.script.push(fixture("01-initial.json"));
    let rec = srv
        .run("/projects/fin/prompt", json!({ "text": "track spending" }))
        .await;
    assert_eq!(rec["status"], "finished", "{rec}");
    srv.run("/projects/fin/sample-rows", json!({})).await;

    let columns = |page: &Value| -> Vec<String> {
        page["columns"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["name"].as_str().unwrap().to_string())
            .collect()
    };
    let (_, before) = srv.get("/projects/fin/tables/transactions/rows").await;
    assert!(columns(&before).contains(&"notes".to_string()));
    let rows_before = before["total"].clone();

    // The follow-up removes a column: it waits for the user.
    srv.script.push(fixture("04-drop-notes.json"));
    let rec = srv
        .run(
            "/projects/fin/prompt",
            json!({ "text": "forget the notes" }),
        )
        .await;
    assert_eq!(rec["status"], "finished", "{rec}");
    let (_, body) = srv.get("/projects/fin/questions").await;
    let open = body["questions"].as_array().unwrap();
    assert_eq!(open.len(), 1, "{body}");
    let q = &open[0];
    assert_eq!(q["blocking"], true);
    let choices: Vec<String> = q["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_lowercase())
        .collect();
    // Nothing changed yet.
    let (_, still) = srv.get("/projects/fin/tables/transactions/rows").await;
    assert!(columns(&still).contains(&"notes".to_string()));

    // Decline first: the data stays.
    let no = choices
        .iter()
        .position(|c| c.starts_with("no") || c.contains("keep") || c.contains("cancel"))
        .unwrap_or_else(|| panic!("no decline choice in {choices:?}"))
        + 1;
    let id = q["id"].as_str().unwrap().to_string();
    let rec = srv
        .run("/projects/fin/answers", json!({ id.clone(): no }))
        .await;
    assert_eq!(rec["status"], "finished", "{rec}");
    let (_, after_no) = srv.get("/projects/fin/tables/transactions/rows").await;
    assert!(
        columns(&after_no).contains(&"notes".to_string()),
        "{after_no}"
    );
    assert_eq!(after_no["total"], rows_before);
    let (_, body) = srv.get("/projects/fin/questions").await;
    assert_eq!(body["questions"], json!([]));

    // Ask again and confirm.
    srv.script.push(fixture("04-drop-notes.json"));
    srv.run(
        "/projects/fin/prompt",
        json!({ "text": "forget the notes, really" }),
    )
    .await;
    let (_, body) = srv.get("/projects/fin/questions").await;
    let q = &body["questions"][0];
    let id = q["id"].as_str().unwrap().to_string();
    let yes = choices
        .iter()
        .position(|c| c.starts_with("yes") || c.contains("remove") || c.contains("delete"))
        .unwrap_or_else(|| panic!("no confirm choice in {choices:?}"))
        + 1;
    let rec = srv.run("/projects/fin/answers", json!({ id: yes })).await;
    assert_eq!(rec["status"], "finished", "{rec}");
    let (_, after_yes) = srv.get("/projects/fin/tables/transactions/rows").await;
    assert!(
        !columns(&after_yes).contains(&"notes".to_string()),
        "{after_yes}"
    );
    assert_eq!(after_yes["total"], rows_before);
}

#[tokio::test]
async fn confirm_destructive_skips_the_question() {
    let srv = start().await;
    srv.create("fin").await;
    srv.script.push(fixture("01-initial.json"));
    srv.run("/projects/fin/prompt", json!({ "text": "track spending" }))
        .await;
    srv.script.push(fixture("04-drop-notes.json"));
    let rec = srv
        .run(
            "/projects/fin/prompt",
            json!({ "text": "forget the notes", "confirm_destructive": true }),
        )
        .await;
    assert_eq!(rec["status"], "finished", "{rec}");
    let (_, body) = srv.get("/projects/fin/questions").await;
    assert_eq!(body["questions"], json!([]));
    let (_, page) = srv.get("/projects/fin/tables/transactions/rows").await;
    let names = page["columns"].to_string();
    assert!(!names.contains("\"notes\""), "{names}");
    // Unknown body fields are refused rather than ignored.
    let (status, _) = srv
        .post("/projects/fin/prompt", json!({ "text": "x", "nope": 1 }))
        .await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn second_write_is_busy_while_one_runs() {
    let srv = start().await;
    srv.create("fin").await;
    srv.script.push(fixture("01-initial.json"));
    srv.script.hold();

    let (status, accepted) = srv
        .post("/projects/fin/prompt", json!({ "text": "track spending" }))
        .await;
    assert_eq!(status, 202, "{accepted}");

    // Every slow route is refused with 409 and a plain message.
    for (path, body) in [
        ("/projects/fin/prompt", json!({ "text": "again" })),
        ("/projects/fin/continue", json!({})),
        ("/projects/fin/apply", json!({})),
        ("/projects/fin/sample-rows", json!({})),
        ("/projects/fin/answers", json!({ "q1": 1 })),
    ] {
        let (status, body) = srv.post(path, body).await;
        assert_eq!(status, 409, "{path}: {body}");
        assert!(error_text(&body).contains("busy"), "{body}");
    }
    let (status, body) = srv
        .post("/projects/fin/prompt", json!({ "text": "again" }))
        .await;
    assert_eq!(status, 409, "{body}");
    assert!(error_text(&body).contains("busy"));
    let (status, body) = srv.post("/projects/fin/apply", json!({})).await;
    assert_eq!(status, 409, "{body}");

    // Reads still work meanwhile.
    for path in [
        "/projects",
        "/projects/fin/summary",
        "/projects/fin/questions",
        "/projects/fin/events",
    ] {
        let (status, body) = srv.get(path).await;
        assert_eq!(status, 200, "{path}: {body}");
    }
    let id = accepted["run_id"].as_str().unwrap();
    let (_, rec) = srv.get(&format!("/projects/fin/runs/{id}")).await;
    assert_eq!(rec["status"], "running");

    // When it ends, the project accepts writes again.
    srv.script.release();
    let rec = srv.wait("/projects/fin/prompt", &accepted).await;
    assert_eq!(rec["status"], "finished", "{rec}");
    let rec = srv.run("/projects/fin/apply", json!({})).await;
    assert_eq!(rec["status"], "finished", "{rec}");
}

#[tokio::test]
async fn write_held_by_another_process_is_busy() {
    let srv = start().await;
    srv.create("fin").await;
    // The CLI (another holder of the project's write lock) is working.
    let lock = WriteLock::acquire(&srv.root.join("fin"), "prompt").unwrap();
    let (status, body) = srv.post("/projects/fin/apply", json!({})).await;
    assert_eq!(status, 409, "{body}");
    assert!(error_text(&body).contains("busy"), "{body}");
    let (status, _) = srv.get("/projects/fin/summary").await;
    assert_eq!(status, 200);
    drop(lock);
    let rec = srv.run("/projects/fin/apply", json!({})).await;
    assert_eq!(rec["status"], "finished", "{rec}");
}

#[tokio::test]
async fn failed_run_is_reported() {
    let srv = start().await;
    srv.create("fin").await;
    // The script is empty: the "model" fails.
    let rec = srv
        .run("/projects/fin/prompt", json!({ "text": "track spending" }))
        .await;
    assert_eq!(rec["status"], "failed", "{rec}");
    assert!(rec["error"].is_string() || rec["error"].is_null());
    let (_, events) = srv.get("/projects/fin/events").await;
    let kinds = events["events"].to_string();
    assert!(kinds.contains("prompt_failed"), "{kinds}");
    // The run can be retried once the model works.
    srv.script.push(fixture("01-initial.json"));
    let rec = srv.run("/projects/fin/continue", json!({})).await;
    assert_eq!(rec["status"], "finished", "{rec}");
    let (_, body) = srv.get("/projects").await;
    assert_eq!(body["projects"][0]["tables"], 4);
}

#[tokio::test]
async fn unknown_project_table_and_page() {
    let srv = start().await;
    srv.create("fin").await;
    for path in [
        "/projects/nope/summary",
        "/projects/nope/questions",
        "/projects/nope/events",
        "/projects/nope/tables/people/rows",
        "/projects/nope/runs/r1",
        "/projects/.hidden/summary",
        "/projects/fin/runs/r99",
    ] {
        let (status, body) = srv.get(path).await;
        assert_eq!(status, 404, "{path}: {body}");
        assert!(!error_text(&body).is_empty());
    }
    for (path, body) in [
        ("/projects/nope/prompt", json!({ "text": "x" })),
        ("/projects/nope/continue", json!({})),
        ("/projects/nope/apply", json!({})),
        ("/projects/nope/sample-rows", json!({})),
        ("/projects/nope/answers", json!({ "q1": 1 })),
    ] {
        let (status, body) = srv.post(path, body).await;
        assert_eq!(status, 404, "{path}: {body}");
    }
    // A project with no tables has no such list; so does one that has some.
    let (status, body) = srv.get("/projects/fin/tables/people/rows").await;
    assert_eq!(status, 404, "{body}");
    srv.script.push(fixture("01-initial.json"));
    srv.run("/projects/fin/prompt", json!({ "text": "track spending" }))
        .await;
    let (status, body) = srv.get("/projects/fin/tables/spaceships/rows").await;
    assert_eq!(status, 404, "{body}");
    assert!(error_text(&body).contains("people"), "{body}");
    // Unknown page; empty prompt; malformed JSON.
    let (status, _) = srv.get("/nothing-here").await;
    assert_eq!(status, 404);
    let (status, _) = srv
        .post("/projects/fin/prompt", json!({ "text": "  " }))
        .await;
    assert_eq!(status, 400);
    let (status, _) = srv.post("/projects/fin/prompt", json!({ "text": 3 })).await;
    assert_eq!(status, 400);
}
