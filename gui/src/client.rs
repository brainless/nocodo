//! HTTP client for the nocodo API. The blocking functions are plain and testable; `ApiClient`
//! runs them on worker threads and hands the results back over a channel that the render loop
//! drains once per frame, so the window never blocks.

use std::sync::mpsc::{Receiver, Sender, channel};

use std::collections::BTreeMap;

use nocodo_types::event::Event;
use nocodo_types::outcome::RowPage;
use nocodo_types::question::Question;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::json;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ProjectEntry {
    pub folder: Option<String>,
    pub name: String,
    #[serde(default)]
    pub tables: Option<usize>,
    #[serde(default)]
    pub last_event: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

impl ProjectEntry {
    /// The name the API addresses the project by.
    pub fn id(&self) -> &str {
        self.folder.as_deref().unwrap_or(&self.name)
    }

    pub fn is_usable(&self) -> bool {
        self.error.is_none() && self.folder.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct EventsPage {
    pub events: Vec<Event>,
    pub last: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RunStatus {
    pub status: String,
    #[serde(default)]
    pub error: Option<String>,
}

/// What `GET /summary` gives: the rows per list and the plain-language description.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Summary {
    #[serde(default)]
    pub tables: BTreeMap<String, u64>,
    #[serde(default)]
    pub summary: String,
}

#[derive(Deserialize)]
struct QuestionsBody {
    questions: Vec<Question>,
}

#[derive(Deserialize)]
struct ProjectsBody {
    projects: Vec<ProjectEntry>,
}

#[derive(Deserialize)]
struct Accepted {
    run_id: String,
}

#[derive(Deserialize)]
struct Created {
    folder: String,
}

#[derive(Deserialize)]
struct ErrorBody {
    error: String,
}

/// Percent-encodes one path segment.
pub fn encode_segment(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A display-ready message, preferring the server's own `error` text.
fn describe(base: &str, e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => match resp.into_json::<ErrorBody>() {
            Ok(body) => body.error,
            Err(_) => format!("The server answered with an error (status {code})."),
        },
        ureq::Error::Transport(_) => format!(
            "Could not reach the server at {base}. Check that nocodo-api is running, \
             or start the GUI without --server so it starts one itself."
        ),
    }
}

fn read<T: DeserializeOwned>(
    base: &str,
    r: Result<ureq::Response, ureq::Error>,
) -> Result<T, String> {
    r.map_err(|e| describe(base, e))?
        .into_json()
        .map_err(|e| format!("The server's answer could not be read: {e}"))
}

fn get<T: DeserializeOwned>(base: &str, path: &str) -> Result<T, String> {
    read(base, ureq::get(&format!("{base}{path}")).call())
}

fn post<T: DeserializeOwned>(base: &str, path: &str, body: serde_json::Value) -> Result<T, String> {
    read(base, ureq::post(&format!("{base}{path}")).send_json(body))
}

pub fn list_projects(base: &str) -> Result<Vec<ProjectEntry>, String> {
    get::<ProjectsBody>(base, "/projects").map(|b| b.projects)
}

/// Creates a project; gives the folder name to select.
pub fn create_project(base: &str, name: &str) -> Result<String, String> {
    post::<Created>(base, "/projects", json!({ "name": name })).map(|c| c.folder)
}

/// Starts a prompt run; gives the run ID.
pub fn send_prompt(base: &str, project: &str, text: &str) -> Result<String, String> {
    let path = format!("/projects/{}/prompt", encode_segment(project));
    post::<Accepted>(base, &path, json!({ "text": text })).map(|a| a.run_id)
}

pub fn events(base: &str, project: &str, after: u64) -> Result<EventsPage, String> {
    get(
        base,
        &format!("/projects/{}/events?after={after}", encode_segment(project)),
    )
}

pub fn run_status(base: &str, project: &str, run_id: &str) -> Result<RunStatus, String> {
    get(
        base,
        &format!(
            "/projects/{}/runs/{}",
            encode_segment(project),
            encode_segment(run_id)
        ),
    )
}

fn project_path(project: &str, rest: &str) -> String {
    format!("/projects/{}{rest}", encode_segment(project))
}

/// Starts a slow run (`continue`, `answers`, `sample-rows`, ...); gives the run ID.
pub fn start_run(
    base: &str,
    project: &str,
    route: &str,
    body: serde_json::Value,
) -> Result<String, String> {
    post::<Accepted>(base, &project_path(project, route), body).map(|a| a.run_id)
}

pub fn questions(base: &str, project: &str) -> Result<Vec<Question>, String> {
    get::<QuestionsBody>(base, &project_path(project, "/questions")).map(|b| b.questions)
}

pub fn summary(base: &str, project: &str) -> Result<Summary, String> {
    get(base, &project_path(project, "/summary"))
}

pub fn rows(
    base: &str,
    project: &str,
    table: &str,
    offset: u64,
    limit: u64,
) -> Result<RowPage, String> {
    let rest = format!(
        "/tables/{}/rows?offset={offset}&limit={limit}",
        encode_segment(table)
    );
    get(base, &project_path(project, &rest))
}

/// A finished request, with display-ready errors.
pub enum Msg {
    Projects(Result<Vec<ProjectEntry>, String>),
    Created(Result<String, String>),
    PromptSent {
        project: String,
        result: Result<String, String>,
    },
    /// A run started by something other than the prompt box (answers, retry, sample rows).
    Started {
        project: String,
        result: Result<String, String>,
    },
    Events {
        project: String,
        result: Result<EventsPage, String>,
    },
    Questions {
        project: String,
        result: Result<Vec<Question>, String>,
    },
    Summary {
        project: String,
        result: Result<Summary, String>,
    },
    Rows {
        project: String,
        table: String,
        offset: u64,
        result: Result<RowPage, String>,
    },
    Run {
        project: String,
        run_id: String,
        result: Result<RunStatus, String>,
    },
}

pub struct ApiClient {
    base: String,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl ApiClient {
    pub fn new(base: String) -> Self {
        let (tx, rx) = channel();
        Self { base, tx, rx }
    }

    fn spawn(&self, work: impl FnOnce(&str) -> Msg + Send + 'static) {
        let base = self.base.clone();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(work(&base));
        });
    }

    pub fn try_recv(&self) -> Option<Msg> {
        self.rx.try_recv().ok()
    }

    pub fn list_projects(&self) {
        self.spawn(|b| Msg::Projects(list_projects(b)));
    }

    pub fn create_project(&self, name: String) {
        self.spawn(move |b| Msg::Created(create_project(b, &name)));
    }

    pub fn send_prompt(&self, project: String, text: String) {
        self.spawn(move |b| Msg::PromptSent {
            result: send_prompt(b, &project, &text),
            project,
        });
    }

    /// `POST /projects/{p}{route}` for a slow operation.
    pub fn start(&self, project: String, route: &'static str, body: serde_json::Value) {
        self.spawn(move |b| Msg::Started {
            result: start_run(b, &project, route, body),
            project,
        });
    }

    pub fn questions(&self, project: String) {
        self.spawn(move |b| Msg::Questions {
            result: questions(b, &project),
            project,
        });
    }

    pub fn summary(&self, project: String) {
        self.spawn(move |b| Msg::Summary {
            result: summary(b, &project),
            project,
        });
    }

    pub fn rows(&self, project: String, table: String, offset: u64, limit: u64) {
        self.spawn(move |b| Msg::Rows {
            result: rows(b, &project, &table, offset, limit),
            project,
            table,
            offset,
        });
    }

    pub fn events(&self, project: String, after: u64) {
        self.spawn(move |b| Msg::Events {
            result: events(b, &project, after),
            project,
        });
    }

    pub fn run_status(&self, project: String, run_id: String) {
        self.spawn(move |b| Msg::Run {
            result: run_status(b, &project, &run_id),
            project,
            run_id,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Serves the given `(status line, body)` replies one connection at a time and sends back
    /// the request heads it saw.
    fn fake_server(
        replies: Vec<(&'static str, String)>,
    ) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let mut seen = Vec::new();
            for (status, body) in replies {
                let (mut conn, _) = listener.accept().unwrap();
                let mut buf = vec![0u8; 8192];
                let mut got = Vec::new();
                loop {
                    let n = conn.read(&mut buf).unwrap();
                    got.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&got).to_string();
                    if let Some(head_end) = text.find("\r\n\r\n") {
                        let len = text[..head_end]
                            .lines()
                            .find_map(|l| {
                                l.to_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap())
                            })
                            .unwrap_or(0);
                        if got.len() >= head_end + 4 + len {
                            break;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                }
                seen.push(String::from_utf8_lossy(&got).to_string());
                let reply = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                conn.write_all(reply.as_bytes()).unwrap();
            }
            seen
        });
        (base, handle)
    }

    #[test]
    fn lists_projects_including_a_broken_one() {
        let body = r#"{"projects":[
            {"folder":"a","name":"a","path":".projects/a","tables":2,"last_event":"2026-10-01T00:00:00Z","error":null},
            {"folder":"b","name":"b","path":".projects/b","tables":null,"last_event":null,"error":"project.json is broken"}]}"#;
        let (base, h) = fake_server(vec![("200 OK", body.into())]);
        let list = list_projects(&base).unwrap();
        assert_eq!(list.len(), 2);
        assert!(list[0].is_usable());
        assert_eq!(list[0].tables, Some(2));
        assert!(!list[1].is_usable());
        assert!(h.join().unwrap()[0].starts_with("GET /projects "));
    }

    #[test]
    fn prompt_is_posted_as_json_and_gives_the_run_id() {
        let (base, h) = fake_server(vec![(
            "202 Accepted",
            r#"{"run_id":"r1","operation":"prompt","status":"running","next":"x"}"#.into(),
        )]);
        assert_eq!(
            send_prompt(&base, "my project", "Track money").unwrap(),
            "r1"
        );
        let seen = h.join().unwrap();
        assert!(seen[0].starts_with("POST /projects/my%20project/prompt "));
        assert!(seen[0].ends_with(r#"{"text":"Track money"}"#));
    }

    #[test]
    fn server_error_text_is_passed_on_and_busy_is_plain() {
        let msg = "This project is busy: `prompt` is running. Wait for it to finish.";
        let (base, _h) = fake_server(vec![("409 Conflict", format!(r#"{{"error":"{msg}"}}"#))]);
        assert_eq!(send_prompt(&base, "p", "x").unwrap_err(), msg);
    }

    #[test]
    fn events_and_run_status_parse() {
        let events_body = r#"{"events":[{"seq":3,"at":"2026-10-01T00:00:00Z","actor":{"type":"user"},"kind":{"type":"user_prompt","text":"hi"}}],"last":3}"#;
        let (base, h) = fake_server(vec![
            ("200 OK", events_body.into()),
            ("200 OK", r#"{"id":"r1","project":"p","operation":"prompt","status":"failed","error":"nope"}"#.into()),
        ]);
        let page = events(&base, "p", 2).unwrap();
        assert_eq!(page.last, 3);
        assert_eq!(page.events.len(), 1);
        let run = run_status(&base, "p", "r1").unwrap();
        assert_eq!(run.status, "failed");
        assert_eq!(run.error.as_deref(), Some("nope"));
        let seen = h.join().unwrap();
        assert!(seen[0].starts_with("GET /projects/p/events?after=2 "));
        assert!(seen[1].starts_with("GET /projects/p/runs/r1 "));
    }

    #[test]
    fn questions_summary_rows_and_answers_use_the_documented_routes() {
        let q = r#"{"questions":[{"id":"q1","asked_by":"schema","text":"Weekly or monthly?","choices":["Weekly","Monthly"],"default":2,"blocking":false,"why":""}]}"#;
        let s = r#"{"tables":{"expenses":3},"applied":[],"pending":[],"backups":0,"summary":"You keep expenses."}"#;
        let r = r#"{"table":"expenses","columns":[{"name":"id","label":"id","column_type":"integer"}],"rows":[[1],[2]],"total":2,"offset":0,"limit":50}"#;
        let a = r#"{"run_id":"r9","operation":"answer","status":"running","next":"x"}"#;
        let (base, h) = fake_server(vec![
            ("200 OK", q.into()),
            ("200 OK", s.into()),
            ("200 OK", r.into()),
            ("202 Accepted", a.into()),
        ]);
        let qs = questions(&base, "p").unwrap();
        assert_eq!(qs[0].id, "q1");
        assert_eq!(qs[0].default, Some(2));
        let sum = summary(&base, "p").unwrap();
        assert_eq!(sum.tables.get("expenses"), Some(&3));
        let page = rows(&base, "p", "expenses", 0, 50).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(
            start_run(&base, "p", "/answers", json!({"q1": 2})).unwrap(),
            "r9"
        );
        let seen = h.join().unwrap();
        assert!(seen[0].starts_with("GET /projects/p/questions "));
        assert!(seen[1].starts_with("GET /projects/p/summary "));
        assert!(seen[2].starts_with("GET /projects/p/tables/expenses/rows?offset=0&limit=50 "));
        assert!(seen[3].starts_with("POST /projects/p/answers "));
        assert!(seen[3].ends_with(r#"{"q1":2}"#));
    }

    #[test]
    fn unreachable_server_message_says_what_to_do() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let err = list_projects(&base).unwrap_err();
        assert!(err.contains("Could not reach the server"));
        assert!(err.contains(&base));
    }

    #[test]
    fn segments_are_encoded() {
        assert_eq!(encode_segment("a b/c"), "a%20b%2Fc");
        assert_eq!(encode_segment("g-finance_1.x"), "g-finance_1.x");
    }
}
