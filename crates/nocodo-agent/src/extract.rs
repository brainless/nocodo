use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
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
    completion::StructuredOutputError,
    extractor::ExtractorBuilder,
};
use rig_core::{
    DynModel,
    message::{AssistantContent, Message, ToolResultContent},
    operation::Completion,
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
    /// Extra model calls `run_extractor` may make when output is invalid.
    /// The first call is not counted, so a run makes at most `retries + 1`.
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
    /// The assistant content of the latest model turn, kept so a rejected
    /// `submit` call can be replayed to the model in the follow-up.
    last_content: Arc<Mutex<Vec<AssistantContent>>>,
    /// Usage of the latest model turn, reported when an output is repaired.
    last_usage: Arc<Mutex<Value>>,
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
        *self.last_content.lock().unwrap_or_else(|e| e.into_inner()) = event.content.clone();
        *self.last_usage.lock().unwrap_or_else(|e| e.into_inner()) = to_json(&event.usage);
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
/// each rejected output (and for tool calls Rig could not dispatch),
/// `repaired_output` when arguments sent as strings holding JSON were
/// repaired and accepted (the usage is then that of the last model call), and
/// `run_finished` with the typed output or the error, `calls`,
/// `invalid_outputs` and `repaired_outputs`. Contains no role-specific logic.
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
    let built = build_model(&ctx.model)?;
    extract_with_model(built.model, built.strict, ctx, preamble, input).await
}

/// The extraction loop over an already built model. Rig's own retry is off
/// (`retries(0)`): it would start over without telling the model why. Instead
/// each rejected output is logged and the conversation continues with a
/// follow-up that quotes the error (see [`follow_up`]).
async fn extract_with_model<T>(
    model: impl Into<DynModel<Completion>>,
    strict: bool,
    ctx: &AgentContext,
    preamble: &str,
    input: &str,
) -> Result<(T, String)>
where
    T: JsonSchema + DeserializeOwned + Serialize + Send + Sync + 'static,
{
    let run_id = uuid::Uuid::now_v7().to_string();
    let log = AgentLog::new(&ctx.project_dir, &run_id, &ctx.role, &ctx.model.model);
    log.append(
        "run_started",
        json!({
            "preamble": preamble,
            "input": input,
            "retries": ctx.retries,
            "strict": strict,
        }),
    );

    let last_content = Arc::new(Mutex::new(Vec::new()));
    let last_usage = Arc::new(Mutex::new(Value::Null));
    let hook = LogHook {
        log: log.clone(),
        attempt: AtomicU32::new(0),
        started: Mutex::new(None),
        last_content: last_content.clone(),
        last_usage: last_usage.clone(),
    };
    let extractor = ExtractorBuilder::<T>::new(model)
        .append_preamble(preamble)
        .max_tokens(ctx.max_tokens)
        .retries(0)
        .add_hook(hook)
        .build();

    let started = Instant::now();
    let mut history: Vec<Message> = Vec::new();
    let mut prompt = Message::user(input);
    let mut calls: u64 = 0;
    let mut invalid_outputs: u64 = 0;
    let mut repaired_outputs: u64 = 0;
    let outcome = loop {
        calls += 1;
        let mut run = extractor.extract(prompt.clone());
        if !history.is_empty() {
            run = run.history(history.clone());
        }
        let mut original_error: Option<String> = None;
        let (raw_output, reason) = match run.await {
            Ok(response) => break Ok((response.output, to_json(&response.usage))),
            Err(StructuredOutputError::DeserializationError(error)) => {
                let mut reason = error.to_string();
                match repair_output::<T>(&last_content) {
                    Some(Ok((output, original, repaired))) => {
                        log.append(
                            "repaired_output",
                            json!({
                                "call": calls,
                                "original": original,
                                "repaired": repaired,
                                "error": reason,
                            }),
                        );
                        repaired_outputs += 1;
                        let usage = last_usage.lock().unwrap_or_else(|e| e.into_inner()).clone();
                        break Ok((output, usage));
                    }
                    // Strings were unwrapped but the value still does not fit:
                    // tell the model the real problem (e.g. a missing field),
                    // not the original string-vs-object error.
                    Some(Err(repaired_error)) => {
                        original_error = Some(std::mem::replace(&mut reason, repaired_error));
                    }
                    None => {}
                }
                (Some(rejected_output(&last_content)), reason)
            }
            Err(StructuredOutputError::EmptyResponse) => {
                (None, "the model did not call the `submit` tool".to_string())
            }
            Err(other) => break Err(other.to_string()),
        };
        invalid_outputs += 1;
        log.append(
            "invalid_output",
            json!({
                "call": calls,
                "output": raw_output,
                "error": reason,
                "original_error": original_error,
            }),
        );
        if calls > ctx.retries {
            break Err(match raw_output {
                Some(_) => format!("structured output does not match the requested type: {reason}"),
                None => "the model returned no structured output".to_string(),
            });
        }
        let rejected = std::mem::take(&mut *last_content.lock().unwrap_or_else(|e| e.into_inner()));
        match follow_up(&rejected, &reason) {
            FollowUp::ToolResult {
                assistant,
                prompt: next,
            } => {
                history.push(std::mem::replace(&mut prompt, next));
                history.push(assistant);
            }
            FollowUp::UserMessage {
                assistant,
                prompt: next,
            } => {
                if let Some(assistant) = assistant {
                    history.push(std::mem::replace(&mut prompt, next));
                    history.push(assistant);
                } else {
                    // Nothing to replay: restate the original input with the error.
                    prompt = Message::user(format!("{input}\n\n{}", user_text(&reason)));
                }
            }
        }
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    let result = match outcome {
        Ok((output, usage)) => {
            log.append(
                "run_finished",
                json!({
                    "ok": true,
                    "output": to_json(&output),
                    "usage": usage,
                    "calls": calls,
                    "invalid_outputs": invalid_outputs,
                    "repaired_outputs": repaired_outputs,
                    "duration_ms": duration_ms,
                }),
            );
            Ok((output, run_id))
        }
        Err(message) => {
            log.append(
                "run_finished",
                json!({
                    "ok": false,
                    "error": message,
                    "calls": calls,
                    "invalid_outputs": invalid_outputs,
                    "repaired_outputs": repaired_outputs,
                    "duration_ms": duration_ms,
                }),
            );
            Err(Error::Extraction(message))
        }
    };
    log.check()?;
    result
}

/// Some models send an object or list argument as a string holding JSON
/// (`"change_set": "{\"changes\": ...}"`). Walks `value` and turns every
/// string that parses as a JSON object or array into that value, repeatedly,
/// so doubly encoded values are unwrapped too. Returns `None` when nothing
/// changed. Scalars and strings that are not objects or arrays stay as they are.
fn unstringify(value: &Value) -> Option<Value> {
    match value {
        Value::String(text) => {
            let trimmed = text.trim_start();
            if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
                return None;
            }
            let parsed = serde_json::from_str::<Value>(text).ok()?;
            Some(unstringify(&parsed).unwrap_or(parsed))
        }
        Value::Array(items) => {
            let new: Vec<Option<Value>> = items.iter().map(unstringify).collect();
            new.iter().any(Option::is_some).then(|| {
                Value::Array(
                    new.into_iter()
                        .zip(items)
                        .map(|(n, old)| n.unwrap_or_else(|| old.clone()))
                        .collect(),
                )
            })
        }
        Value::Object(map) => {
            let new: Vec<Option<Value>> = map.values().map(unstringify).collect();
            new.iter().any(Option::is_some).then(|| {
                Value::Object(
                    new.into_iter()
                        .zip(map)
                        .map(|(n, (k, old))| (k.clone(), n.unwrap_or_else(|| old.clone())))
                        .collect(),
                )
            })
        }
        _ => None,
    }
}

/// Tries to rescue a rejected `submit` call by un-stringifying its arguments
/// (see [`unstringify`]). Returns the typed value with the original and the
/// repaired arguments, or `None` if there is no `submit` call, nothing to
/// repair. `Some(Err(message))` means strings were unwrapped but the repaired
/// arguments still do not match `T`; `message` is that deserialization error.
fn repair_output<T: DeserializeOwned>(
    last_content: &Mutex<Vec<AssistantContent>>,
) -> Option<std::result::Result<(T, Value, Value), String>> {
    let original = last_content
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .find_map(|c| match c {
            AssistantContent::ToolCall(call) => Some(call.function.arguments.clone()),
            _ => None,
        })?;
    let outcome = try_repair::<T>(&original)?;
    Some(outcome.map(|(output, repaired)| (output, original, repaired)))
}

fn try_repair<T: DeserializeOwned>(
    arguments: &Value,
) -> Option<std::result::Result<(T, Value), String>> {
    let repaired = unstringify(arguments)?;
    Some(
        serde_json::from_value::<T>(repaired.clone())
            .map(|output| (output, repaired))
            .map_err(|e| e.to_string()),
    )
}

#[cfg(test)]
fn repair_arguments<T: DeserializeOwned>(arguments: &Value) -> Option<(T, Value)> {
    try_repair(arguments)?.ok()
}

/// The next step of a conversation after a rejected output.
#[derive(Debug, Clone, PartialEq)]
enum FollowUp {
    /// The model made a `submit` call: replay it, then answer it with an
    /// error tool result. `assistant` goes to history, `prompt` is sent.
    ToolResult { assistant: Message, prompt: Message },
    /// The model made no tool call: replay what it said (if anything), then
    /// send a plain user message.
    UserMessage {
        assistant: Option<Message>,
        prompt: Message,
    },
}

/// The raw output of the last model turn as the log should show it: the
/// arguments of its `submit` call, or its text when it made no call.
fn rejected_output(last_content: &Mutex<Vec<AssistantContent>>) -> String {
    let content = last_content.lock().unwrap_or_else(|e| e.into_inner());
    content
        .iter()
        .find_map(|c| match c {
            AssistantContent::ToolCall(call) => Some(call.function.arguments.to_string()),
            _ => None,
        })
        .or_else(|| {
            content.iter().find_map(|c| match c {
                AssistantContent::Text(t) => Some(t.text.clone()),
                _ => None,
            })
        })
        .unwrap_or_default()
}

fn user_text(error: &str) -> String {
    format!(
        "Your answer was rejected and nothing was saved: {error}\n\
         Answer again by calling `submit` exactly once. Its arguments must match the schema \
         exactly: every list and object must be real JSON, not text containing JSON."
    )
}

/// Builds the follow-up for a rejected model turn. Pure, so it can be
/// snapshot-tested. Rig can carry a tool result as the next prompt
/// (`Message::tool_results` with `ToolCall::result`), so a rejected `submit`
/// call is answered that way, with the error quoted verbatim. Rig 0.43's
/// `ToolResult` has no `is_error` flag, so the text says the call failed.
fn follow_up(rejected: &[AssistantContent], error: &str) -> FollowUp {
    let calls: Vec<_> = rejected
        .iter()
        .filter_map(|c| match c {
            AssistantContent::ToolCall(call) => Some(call),
            _ => None,
        })
        .collect();
    let assistant = (!rejected.is_empty()).then(|| Message::Assistant {
        id: None,
        content: rejected.to_vec(),
    });
    if calls.is_empty() {
        return FollowUp::UserMessage {
            assistant,
            prompt: Message::user(user_text(error)),
        };
    }
    let results = calls
        .iter()
        .enumerate()
        .map(|(i, call)| {
            let text = if i == 0 {
                user_text(error)
            } else {
                "Not run: call `submit` only once.".to_string()
            };
            call.result(vec![ToolResultContent::text(text)])
        })
        .collect();
    FollowUp::ToolResult {
        assistant: assistant.expect("calls imply content"),
        prompt: Message::tool_results(results),
    }
}

#[cfg(test)]
mod tests;
