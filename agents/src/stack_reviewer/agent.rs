use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use crate::{
    error::AgentError,
    storage::{StackNoteStorage, StackTag},
    utils::{
        file_ops,
        tools::{ListFilesParams, ReadFileParams},
    },
};

use super::tools::{EmitNoteParams, FinishReviewParams};

const MAX_TURNS: u32 = 20;

pub struct StackReviewerAgent {
    llm_client: Arc<dyn LlmClient>,
    stack_note_storage: Arc<dyn StackNoteStorage>,
    model: String,
    project_id: i64,
    project_path: PathBuf,
}

pub struct StackReviewResult {
    pub emitted_ids: Vec<i64>,
    pub summary: String,
}

impl StackReviewerAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        stack_note_storage: Arc<dyn StackNoteStorage>,
        model: impl Into<String>,
        project_id: i64,
        project_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            llm_client,
            stack_note_storage,
            model: model.into(),
            project_id,
            project_path: project_path.into(),
        }
    }

    pub async fn run(&self) -> Result<StackReviewResult, AgentError> {
        // Load current notes.
        let current_notes = self
            .stack_note_storage
            .list_current_notes(self.project_id)
            .await?;

        let current_notes_text = format_notes(&current_notes);
        let system = super::prompts::system_prompt(&current_notes_text);

        // Build tools.
        let list_files_tool = Tool::from_type::<ListFilesParams>()
            .name("list_files")
            .description(
                "List files and directories at the given path relative to the project root. \
                 Pass an empty string for the project root, or a subdirectory path like 'backend/src'. \
                 Returns a listing of files and directories.",
            )
            .build();

        let read_file_tool = Tool::from_type::<ReadFileParams>()
            .name("read_file")
            .description(
                "Read the contents of a file at the given path relative to the project root. \
                 Use this to examine source files, Cargo.toml, config files, migrations, etc.",
            )
            .build();

        // Aliases the model may attempt from training data — registered so the API
        // doesn't reject the request; handled in the match below.
        let repo_browser_list = Tool::from_type::<ListFilesParams>()
            .name("repo_browser.list_files")
            .description("Alias of list_files.")
            .build();

        let repo_browser_open = Tool::from_type::<ReadFileParams>()
            .name("repo_browser.open_file")
            .description("Alias of read_file.")
            .build();

        let repo_browser_search = Tool::from_type::<ReadFileParams>()
            .name("repo_browser.search")
            .description("Not available. Use list_files and read_file instead.")
            .build();

        let emit_note_tool = Tool::from_type::<EmitNoteParams>()
            .name("emit_note")
            .description(
                "Emit a tech stack note. Provide tag, note text, optional file_path and line_number. \
                 If this note supersedes an existing one, provide the exact existing note text in replaces_note.",
            )
            .build();

        let finish_review_tool = Tool::from_type::<FinishReviewParams>()
            .name("finish_review")
            .description(
                "Call this when you have finished reviewing the codebase and emitting all relevant notes. \
                 Provide a brief summary of what was added or updated.",
            )
            .build();

        let mut messages: Vec<Message> = vec![Message {
            role: Role::User,
            content: vec![llm_sdk::types::ContentBlock::Text {
                text: "Please review the project codebase and update the tech stack notes."
                    .to_string(),
            }],
            tool_call_id: None,
            tool_name: None,
        }];

        let mut emitted_ids: Vec<i64> = Vec::new();
        let mut emitted_texts: HashSet<String> = HashSet::new();
        let mut turns: u32 = 0;
        let mut consecutive_validation_errors: u32 = 0;

        loop {
            if turns >= MAX_TURNS {
                return Err(AgentError::Other(
                    "stack_reviewer exceeded MAX_TURNS without calling finish_review".to_string(),
                ));
            }

            // After 3 consecutive validation errors the model is stuck; force finish_review.
            let tool_choice = if consecutive_validation_errors >= 3 {
                ToolChoice::Specific {
                    name: "finish_review".to_string(),
                }
            } else {
                ToolChoice::Auto
            };

            let request = CompletionRequest {
                messages: messages.clone(),
                max_tokens: 4096,
                model: self.model.clone(),
                system: Some(system.clone()),
                temperature: Some(0.2),
                top_p: None,
                stop_sequences: None,
                tools: Some(vec![
                    list_files_tool.clone(),
                    read_file_tool.clone(),
                    emit_note_tool.clone(),
                    finish_review_tool.clone(),
                    repo_browser_list.clone(),
                    repo_browser_open.clone(),
                    repo_browser_search.clone(),
                ]),
                tool_choice: Some(tool_choice),
                response_format: None,
            };

            let response = match self.llm_client.complete(request).await {
                Ok(r) => {
                    consecutive_validation_errors = 0;
                    r
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("validation failed") || msg.contains("missing properties") {
                        consecutive_validation_errors += 1;
                        log::warn!(
                            "[stack_reviewer] LLM tool validation error (turn {}, consecutive={}): {}. Nudging.",
                            turns, consecutive_validation_errors, msg
                        );
                        messages.push(Message {
                            role: Role::User,
                            content: vec![llm_sdk::types::ContentBlock::Text {
                                text: "A tool call was invalid. You MUST use only: list_files, read_file, emit_note, finish_review. Call finish_review now if you are done.".to_string(),
                            }],
                            tool_call_id: None,
                            tool_name: None,
                        });
                        turns += 1;
                        continue;
                    }
                    return Err(AgentError::Llm(e));
                }
            };

            let assistant_text = response
                .content
                .iter()
                .filter_map(|b| match b {
                    llm_sdk::types::ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");

            if let Some(tool_calls) = response.tool_calls {
                // If there's assistant text alongside tool calls, push it first.
                if !assistant_text.is_empty() {
                    messages.push(Message {
                        role: Role::Assistant,
                        content: vec![llm_sdk::types::ContentBlock::Text {
                            text: assistant_text.clone(),
                        }],
                        tool_call_id: None,
                        tool_name: None,
                    });
                }

                for tool_call in tool_calls {
                    let tool_name = tool_call.name().to_string();
                    let call_id = tool_call.id().to_string();

                    // Push assistant invocation message.
                    messages.push(Message {
                        role: Role::Assistant,
                        content: vec![llm_sdk::types::ContentBlock::Text {
                            text: serde_json::to_string(tool_call.arguments())?,
                        }],
                        tool_call_id: Some(call_id.clone()),
                        tool_name: Some(tool_name.clone()),
                    });

                    // Execute tool and get result.
                    let result = match tool_name.as_str() {
                        "list_files" | "repo_browser.list_files" => {
                            let params: ListFilesParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            file_ops::list_files(&self.project_path, &params.path)
                        }
                        "read_file" | "repo_browser.open_file" | "repo_browser.read_file" => {
                            let params: ReadFileParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            file_ops::read_file(&self.project_path, &params.path)
                        }
                        "repo_browser.search" => {
                            "Search is not available. Use list_files to explore directories and read_file to examine files.".to_string()
                        }
                        "emit_note" => {
                            let params: EmitNoteParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;

                            // Same-run dedup check.
                            if emitted_texts.contains(&params.note) {
                                format!(
                                    "Error: note '{}' was already emitted in this review run.",
                                    params.note
                                )
                            } else {
                                let tag = StackTag::from_str(&params.tag);
                                match self
                                    .stack_note_storage
                                    .add_note(
                                        self.project_id,
                                        tag,
                                        params.note.clone(),
                                        params.file_path,
                                        params.line_number,
                                        params.replaces_note,
                                    )
                                    .await
                                {
                                    Ok(id) => {
                                        emitted_ids.push(id);
                                        emitted_texts.insert(params.note.clone());
                                        format!("Note emitted with id={}.", id)
                                    }
                                    Err(e) => format!("Error emitting note: {}", e),
                                }
                            }
                        }
                        "finish_review" => {
                            let params: FinishReviewParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;

                            // Push the tool result message before returning.
                            messages.push(Message {
                                role: Role::Tool,
                                content: vec![llm_sdk::types::ContentBlock::Text {
                                    text: "Review finished.".to_string(),
                                }],
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some(tool_name.clone()),
                            });

                            return Ok(StackReviewResult {
                                emitted_ids,
                                summary: params.summary,
                            });
                        }
                        unknown => {
                            format!("Error: unknown tool '{}'.", unknown)
                        }
                    };

                    // Push tool result message.
                    messages.push(Message {
                        role: Role::Tool,
                        content: vec![llm_sdk::types::ContentBlock::Text { text: result }],
                        tool_call_id: Some(call_id),
                        tool_name: Some(tool_name),
                    });
                }
            } else {
                // No tool calls — push assistant text and nudge.
                if !assistant_text.is_empty() {
                    messages.push(Message {
                        role: Role::Assistant,
                        content: vec![llm_sdk::types::ContentBlock::Text {
                            text: assistant_text,
                        }],
                        tool_call_id: None,
                        tool_name: None,
                    });
                }

                messages.push(Message {
                    role: Role::User,
                    content: vec![llm_sdk::types::ContentBlock::Text {
                        text: "Continue reviewing. Call finish_review when done.".to_string(),
                    }],
                    tool_call_id: None,
                    tool_name: None,
                });

                turns += 1;
            }
        }
    }
}

fn format_notes(notes: &[crate::storage::StackNoteRow]) -> String {
    if notes.is_empty() {
        return "(no notes yet)".to_string();
    }
    notes
        .iter()
        .map(|n| {
            let location = match (&n.file_path, n.line_number) {
                (Some(fp), Some(ln)) => format!(" ({fp}:{ln})"),
                (Some(fp), None) => format!(" ({fp})"),
                _ => String::new(),
            };
            format!("id={} [{}] {}{}", n.id, n.tag, n.note, location)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
