use std::{
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU32, Ordering},
    },
    time::Instant,
};

use nocodo_types::project::ModelConfig;
use rig_agent::{
    agent::{
        AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, InvalidToolCallAction,
        InvalidToolCallContext, ModelTurnAction, ModelTurnFinished,
    },
    extractor::ExtractorBuilder,
};
use schemars::JsonSchema;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::{AgentLog, Error, Result, build_model};

/// Everything a role needs to run one LLM call: where the project is, which
/// role and model, and the retry budget for invalid output.
#[derive(Debug, Clone)]
pub struct AgentContext {
    pub project_dir: PathBuf,
    pub role: String,
    pub model: ModelConfig,
    /// Extra attempts the extractor may make when output is invalid.
    pub retries: u64,
    pub max_tokens: u64,
}

impl AgentContext {
    pub fn new(project_dir: impl Into<PathBuf>, role: &str, model: ModelConfig) -> Self {
        Self {
            project_dir: project_dir.into(),
            role: role.to_string(),
            model,
            retries: 2,
            max_tokens: 16_000,
        }
    }
}

/// Logs the real exchange for each completion call of one run.
struct LogHook {
    log: AgentLog,
    attempt: AtomicU32,
    started: Mutex<Option<Instant>>,
}

impl AgentHook for LogHook {
    async fn on_completion_call(
        &self,
        _ctx: &HookContext,
        event: CompletionCallEvent<'_>,
    ) -> CompletionCallAction {
        self.attempt.fetch_add(1, Ordering::SeqCst);
        *self.started.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
        self.log.append(
            "completion_request",
            json!({
                "attempt": self.attempt.load(Ordering::SeqCst),
                "turn": event.turn,
                "prompt": to_json(event.prompt),
                "history": to_json(&event.history),
            }),
        );
        CompletionCallAction::Continue
    }

    async fn on_model_turn_finished(
        &self,
        _ctx: &HookContext,
        event: ModelTurnFinished<'_>,
    ) -> ModelTurnAction {
        let duration_ms = self
            .started
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .map(|s| s.elapsed().as_millis() as u64);
        self.log.append(
            "completion",
            json!({
                "attempt": self.attempt.load(Ordering::SeqCst),
                "turn": event.turn,
                "content": to_json(event.content),
                "raw_response": event.raw,
                "usage": to_json(&event.usage),
                "finish_reason": event.finish_reason.map(|r| format!("{r:?}")),
                "duration_ms": duration_ms,
            }),
        );
        ModelTurnAction::Continue
    }

    async fn on_invalid_tool_call(
        &self,
        _ctx: &HookContext,
        event: &InvalidToolCallContext,
    ) -> Option<InvalidToolCallAction> {
        self.log.append(
            "invalid_output",
            json!({
                "attempt": self.attempt.load(Ordering::SeqCst),
                "tool_name": event.tool_name,
                "args": event.args,
                "reason": format!("{:?}", event.reason),
            }),
        );
        None
    }
}

fn to_json<T: Serialize + ?Sized>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or_else(|e| json!({ "unserializable": e.to_string() }))
}

/// Runs one typed extraction: builds the model, hooks Rig's lifecycle into the
/// run's JSONL log, and returns the parsed `T`.
///
/// Log lines written: `run_started` (preamble and input in full), one
/// `completion_request`/`completion` pair per model call, `invalid_output` for
/// tool calls Rig could not dispatch, and `run_finished` with either the typed
/// output or the error. Contains no role-specific logic.
pub async fn run_extractor<T>(ctx: &AgentContext, preamble: &str, input: &str) -> Result<T>
where
    T: JsonSchema + DeserializeOwned + Serialize + Send + Sync + 'static,
{
    run_extractor_with_id(ctx, preamble, input)
        .await
        .map(|(output, _)| output)
}

/// Like [`run_extractor`], but also returns the run ID (the name of the
/// `.agent-logs/<run_id>.jsonl` file), so callers can record it on events.
pub async fn run_extractor_with_id<T>(
    ctx: &AgentContext,
    preamble: &str,
    input: &str,
) -> Result<(T, String)>
where
    T: JsonSchema + DeserializeOwned + Serialize + Send + Sync + 'static,
{
    let model = build_model(&ctx.model)?;
    let run_id = uuid::Uuid::now_v7().to_string();
    let log = AgentLog::new(&ctx.project_dir, &run_id, &ctx.role, &ctx.model.model);
    log.append(
        "run_started",
        json!({ "preamble": preamble, "input": input, "retries": ctx.retries }),
    );

    let hook = LogHook {
        log: log.clone(),
        attempt: AtomicU32::new(0),
        started: Mutex::new(None),
    };
    let extractor = ExtractorBuilder::<T>::new(model)
        .append_preamble(preamble)
        .max_tokens(ctx.max_tokens)
        .retries(ctx.retries)
        .add_hook(hook)
        .build();

    let started = Instant::now();
    let result = extractor.extract(input).await;
    let duration_ms = started.elapsed().as_millis() as u64;
    let result = match result {
        Ok(response) => {
            log.append(
                "run_finished",
                json!({
                    "ok": true,
                    "output": to_json(&response.output),
                    "usage": to_json(&response.usage),
                    "duration_ms": duration_ms,
                }),
            );
            Ok((response.output, run_id))
        }
        Err(err) => {
            let message = err.to_string();
            log.append(
                "run_finished",
                json!({ "ok": false, "error": message, "duration_ms": duration_ms }),
            );
            Err(Error::Extraction(message))
        }
    };
    log.check()?;
    result
}
