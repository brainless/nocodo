//! HTTP server over the engine (epic 008, Task 4). Parses requests, calls the
//! engine and returns JSON; it holds no business logic.
//!
//! Slow operations (`prompt`, `continue`, `apply`, `sample-rows` and an answer
//! that resumes a run) return `202` with a run ID and run on a background
//! thread, one at a time per project. The server is generic over how the
//! `Proposer` is built so tests can start it in-process with a scripted one.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use nocodo_engine::{EngineError, Project, Proposer, WriteLock};
use nocodo_types::question::Answer;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Builds the proposer for one operation (production: the Schema agent).
type MakeProposer<P> = Arc<dyn Fn(&Project) -> P + Send + Sync>;

/// How a background run is going.
#[derive(Debug, Clone, Serialize)]
pub struct RunRecord {
    pub id: String,
    pub project: String,
    pub operation: String,
    /// `running`, `finished` or `failed`.
    pub status: &'static str,
    /// Plain-language reason when `failed`; what happened is in the events.
    pub error: Option<String>,
}

#[derive(Default)]
struct Runs {
    counter: u64,
    busy: HashSet<String>,
    records: HashMap<String, RunRecord>,
}

/// Shared server state.
pub struct AppState<P> {
    projects_dir: PathBuf,
    make: MakeProposer<P>,
    runs: Mutex<Runs>,
}

impl<P> AppState<P> {
    /// `projects_dir` holds one folder per project; `make` builds the
    /// proposer for each operation.
    pub fn new(
        projects_dir: PathBuf,
        make: impl Fn(&Project) -> P + Send + Sync + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            projects_dir,
            make: Arc::new(make),
            runs: Mutex::new(Runs::default()),
        })
    }
}

/// An error answered as `{"error": "<plain-language message>"}`.
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }
}

impl From<EngineError> for ApiError {
    fn from(e: EngineError) -> Self {
        use nocodo_engine::OrchestratorError as O;
        let status = match &e {
            EngineError::Busy { .. } => StatusCode::CONFLICT,
            EngineError::NotFound(_) => StatusCode::NOT_FOUND,
            EngineError::Orchestrator(O::NotAProject(_)) => StatusCode::NOT_FOUND,
            EngineError::Orchestrator(O::DirectoryNotEmpty(_) | O::QuestionsWaiting(_)) => {
                StatusCode::CONFLICT
            }
            EngineError::Orchestrator(O::InvalidAnswer(_)) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self::new(status, e.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, axum::Json(json!({ "error": self.message }))).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;
type Shared<P> = State<Arc<AppState<P>>>;

/// The routes. Serve with [`serve`], or with `axum::serve` on any listener.
pub fn router<P: Proposer + 'static>(state: Arc<AppState<P>>) -> Router {
    Router::new()
        .route(
            "/projects",
            get(list_projects::<P>).post(create_project::<P>),
        )
        .route("/projects/{p}/summary", get(summary::<P>))
        .route("/projects/{p}/tables/{t}/rows", get(rows::<P>))
        .route("/projects/{p}/prompt", post(prompt::<P>))
        .route("/projects/{p}/continue", post(continue_run::<P>))
        .route("/projects/{p}/apply", post(apply::<P>))
        .route("/projects/{p}/sample-rows", post(sample_rows::<P>))
        .route("/projects/{p}/questions", get(questions::<P>))
        .route("/projects/{p}/answers", post(answers::<P>))
        .route("/projects/{p}/events", get(events::<P>))
        .route("/projects/{p}/runs/{r}", get(run_status::<P>))
        .fallback(not_found)
        .with_state(state)
}

/// Serve `router(state)` on `listener` until the process stops. Tests bind
/// `127.0.0.1:0` and read the port from `listener.local_addr()`.
pub async fn serve<P: Proposer + 'static>(
    listener: tokio::net::TcpListener,
    state: Arc<AppState<P>>,
) -> std::io::Result<()> {
    axum::serve(listener, router(state)).await
}

async fn not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "There is no such page. See the routes under /projects.",
    )
}

// ---- helpers ----

/// Run blocking engine work off the async threads.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, EngineError> + Send + 'static,
) -> ApiResult<T> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("The request failed unexpectedly: {e}"),
            )
        })?
        .map_err(Into::into)
}

/// Parse a JSON body; an empty body is `{}`.
fn parse_body<T: for<'de> Deserialize<'de>>(body: &Bytes) -> ApiResult<T> {
    let bytes: &[u8] = if body.iter().all(u8::is_ascii_whitespace) {
        b"{}"
    } else {
        body
    };
    serde_json::from_slice(bytes).map_err(|e| {
        ApiError::bad_request(format!("The request body is not what was expected: {e}"))
    })
}

fn number_param(q: &HashMap<String, String>, key: &str, default: u64) -> ApiResult<u64> {
    match q.get(key) {
        None => Ok(default),
        Some(v) => v.parse().map_err(|_| {
            ApiError::bad_request(format!("`{key}` must be a whole number, not `{v}`."))
        }),
    }
}

/// A project folder name is one plain path segment.
fn check_folder_name(name: &str) -> ApiResult<()> {
    let bad = name.is_empty()
        || name.starts_with('.')
        || name.contains(['/', '\\', '\0'])
        || name != name.trim();
    if bad {
        return Err(ApiError::bad_request(format!(
            "`{name}` cannot be a project name: use letters, numbers, dashes and spaces, \
             without slashes, and not starting with a dot."
        )));
    }
    Ok(())
}

/// The folder of an existing project, or 404.
fn project_dir<P>(state: &AppState<P>, name: &str) -> ApiResult<PathBuf> {
    let missing = || {
        ApiError::new(
            StatusCode::NOT_FOUND,
            format!(
                "There is no project called `{name}`. List the projects with GET /projects, or create one with POST /projects."
            ),
        )
    };
    check_folder_name(name).map_err(|_| missing())?;
    let dir = state.projects_dir.join(name);
    if dir.join("project.json").is_file() {
        Ok(dir)
    } else {
        Err(missing())
    }
}

// ---- background runs ----

/// Clears the project's busy flag when the run ends, even by a panic.
struct RunGuard<P> {
    state: Arc<AppState<P>>,
    project: String,
    run_id: String,
}

impl<P> RunGuard<P> {
    fn finish(&self, error: Option<String>) {
        let mut runs = self.state.runs.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(r) = runs.records.get_mut(&self.run_id) {
            r.status = if error.is_some() {
                "failed"
            } else {
                "finished"
            };
            r.error = error;
        }
    }
}

impl<P> Drop for RunGuard<P> {
    fn drop(&mut self) {
        let mut runs = self.state.runs.lock().unwrap_or_else(|e| e.into_inner());
        runs.busy.remove(&self.project);
        if let Some(r) = runs.records.get_mut(&self.run_id)
            && r.status == "running"
        {
            r.status = "failed";
            r.error = Some("The run stopped unexpectedly.".to_string());
        }
    }
}

/// Start `work` on a background thread for `project`. One run per project at a
/// time: a second gets `409`. Returns `202` with the run ID.
fn start_run<P: Proposer + 'static>(
    state: &Arc<AppState<P>>,
    project: &str,
    operation: &'static str,
    work: impl FnOnce(&PathBuf, &MakeProposer<P>) -> Result<(), EngineError> + Send + 'static,
) -> ApiResult<Response> {
    let dir = project_dir(state, project)?;
    let run_id = {
        let mut runs = state.runs.lock().unwrap_or_else(|e| e.into_inner());
        if runs.busy.contains(project) {
            return Err(busy_error(&runs, project));
        }
        runs.counter += 1;
        let id = format!("r{}", runs.counter);
        runs.busy.insert(project.to_string());
        runs.records.insert(
            id.clone(),
            RunRecord {
                id: id.clone(),
                project: project.to_string(),
                operation: operation.to_string(),
                status: "running",
                error: None,
            },
        );
        id
    };
    let guard = RunGuard {
        state: state.clone(),
        project: project.to_string(),
        run_id: run_id.clone(),
    };
    // Another process (the CLI) may hold the project: refuse now, not later.
    if let Err(e) = WriteLock::acquire(&dir, operation) {
        guard.finish(Some(e.to_string()));
        return Err(e.into());
    }
    let make = state.make.clone();
    std::thread::spawn(move || {
        let result = work(&dir, &make);
        guard.finish(result.err().map(|e| e.to_string()));
    });
    Ok((
        StatusCode::ACCEPTED,
        axum::Json(json!({
            "run_id": run_id,
            "operation": operation,
            "status": "running",
            "next": format!("Follow it with GET /projects/{project}/events or GET /projects/{project}/runs/{run_id}."),
        })),
    )
        .into_response())
}

fn busy_error(runs: &Runs, project: &str) -> ApiError {
    let running = runs
        .records
        .values()
        .find(|r| r.project == project && r.status == "running")
        .map(|r| format!("`{}` ({})", r.operation, r.id))
        .unwrap_or_else(|| "another change".to_string());
    ApiError::new(
        StatusCode::CONFLICT,
        format!(
            "This project is busy: {running} is running. Wait for it to finish (watch GET /projects/{project}/events) and try again."
        ),
    )
}

// ---- handlers ----

async fn list_projects<P: Proposer + 'static>(State(s): Shared<P>) -> ApiResult<Response> {
    let root = s.projects_dir.clone();
    let list = blocking(move || nocodo_engine::list_projects(&root)).await?;
    let items: Vec<Value> = list
        .iter()
        .map(|p| {
            let folder = std::path::Path::new(&p.path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            json!({
                "folder": folder,
                "name": p.name,
                "path": p.path,
                "tables": p.tables,
                "last_event": p.last_event.map(|t| t.to_string()),
                "error": p.error,
            })
        })
        .collect();
    Ok(axum::Json(json!({ "projects": items })).into_response())
}

#[derive(Deserialize)]
struct NewProject {
    name: String,
}

async fn create_project<P: Proposer + 'static>(
    State(s): Shared<P>,
    body: Bytes,
) -> ApiResult<Response> {
    let req: NewProject = parse_body(&body)?;
    check_folder_name(&req.name)?;
    let dir = s.projects_dir.join(&req.name);
    let name = req.name.clone();
    let out = blocking(move || nocodo_engine::init(&dir, Some(name), None)).await?;
    Ok((
        StatusCode::CREATED,
        axum::Json(json!({
            "folder": req.name,
            "name": out.name,
            "settings_from": out.source,
            "schema_model": out.schema_model,
        })),
    )
        .into_response())
}

async fn summary<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path(p): Path<String>,
) -> ApiResult<Response> {
    let dir = project_dir(&s, &p)?;
    let out = blocking(move || nocodo_engine::inspect(&dir)).await?;
    Ok(axum::Json(json!({
        "tables": out.tables,
        "applied": out.applied,
        "pending": out.pending,
        "backups": out.backups,
        "summary": out.summary,
    }))
    .into_response())
}

async fn rows<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path((p, t)): Path<(String, String)>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResult<Response> {
    let dir = project_dir(&s, &p)?;
    let offset = number_param(&q, "offset", 0)?;
    let limit = number_param(&q, "limit", 50)?;
    let page = blocking(move || nocodo_engine::read_rows(&dir, &t, offset, limit)).await?;
    Ok(axum::Json(page).into_response())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptBody {
    text: String,
    #[serde(default)]
    confirm_destructive: bool,
}

async fn prompt<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path(p): Path<String>,
    body: Bytes,
) -> ApiResult<Response> {
    let req: PromptBody = parse_body(&body)?;
    if req.text.trim().is_empty() {
        return Err(ApiError::bad_request(
            "The prompt is empty. Say in your own words what you want to keep track of.",
        ));
    }
    start_run(&s, &p, "prompt", move |dir, make| {
        nocodo_engine::prompt_with(dir, &req.text, req.confirm_destructive, |pr| make(pr))
            .map(|_| ())
    })
}

async fn continue_run<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path(p): Path<String>,
) -> ApiResult<Response> {
    start_run(&s, &p, "continue", |dir, make| {
        nocodo_engine::continue_with(dir, |pr| make(pr)).map(|_| ())
    })
}

async fn apply<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path(p): Path<String>,
) -> ApiResult<Response> {
    start_run(&s, &p, "apply", |dir, _| {
        nocodo_engine::apply(dir).map(|_| ())
    })
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct SampleBody {
    per_table: Option<u32>,
}

async fn sample_rows<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path(p): Path<String>,
    body: Bytes,
) -> ApiResult<Response> {
    let req: SampleBody = parse_body(&body)?;
    let per_table = req.per_table.unwrap_or(3);
    start_run(&s, &p, "sample-rows", move |dir, _| {
        nocodo_engine::sample_rows(dir, per_table).map(|_| ())
    })
}

async fn questions<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path(p): Path<String>,
) -> ApiResult<Response> {
    let dir = project_dir(&s, &p)?;
    let list = blocking(move || nocodo_engine::questions(&dir)).await?;
    Ok(axum::Json(json!({ "questions": list })).into_response())
}

/// `{"q1": 2, "q3": 1}` and/or `{"defaults": true}`.
fn parse_answers(body: &Bytes) -> ApiResult<(Vec<Answer>, bool)> {
    let map: BTreeMap<String, Value> = parse_body(body)?;
    let mut defaults = false;
    let mut given = Vec::new();
    for (key, value) in map {
        if key == "defaults" {
            defaults = value
                .as_bool()
                .ok_or_else(|| ApiError::bad_request("`defaults` must be true or false."))?;
            continue;
        }
        let choice = value
            .as_u64()
            .filter(|c| *c >= 1)
            .ok_or_else(|| {
                ApiError::bad_request(format!(
                    "The answer to `{key}` must be a choice number starting from 1, for example {{\"{key}\": 2}}."
                ))
            })?;
        given.push(Answer {
            question_id: key,
            choice: choice as usize,
            by_default: false,
        });
    }
    if given.is_empty() && !defaults {
        return Err(ApiError::bad_request(
            "No answers given. Send {\"q1\": 2} to choose, or {\"defaults\": true} to accept the defaults.",
        ));
    }
    Ok((given, defaults))
}

async fn answers<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path(p): Path<String>,
    body: Bytes,
) -> ApiResult<Response> {
    let (given, defaults) = parse_answers(&body)?;
    start_run(&s, &p, "answer", move |dir, make| {
        nocodo_engine::answer_with(dir, given, defaults, |pr| make(pr)).map(|_| ())
    })
}

async fn events<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path(p): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResult<Response> {
    let dir = project_dir(&s, &p)?;
    let after = number_param(&q, "after", 0)?;
    let list = blocking(move || Ok(Project::open(&dir)?.events()?)).await?;
    let last = list.last().map(|e| e.seq).unwrap_or(0);
    let newer: Vec<_> = list.into_iter().filter(|e| e.seq > after).collect();
    Ok(axum::Json(json!({ "events": newer, "last": last })).into_response())
}

async fn run_status<P: Proposer + 'static>(
    State(s): Shared<P>,
    Path((p, r)): Path<(String, String)>,
) -> ApiResult<Response> {
    project_dir(&s, &p)?;
    let runs = s.runs.lock().unwrap_or_else(|e| e.into_inner());
    match runs.records.get(&r).filter(|rec| rec.project == p) {
        Some(rec) => Ok(axum::Json(rec.clone()).into_response()),
        None => Err(ApiError::new(
            StatusCode::NOT_FOUND,
            format!("There is no run `{r}` in this project since the server started."),
        )),
    }
}
