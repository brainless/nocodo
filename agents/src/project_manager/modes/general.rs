use super::core::pm_core;

/// Mode: ongoing session — triage, status updates, new requests mid-project.
///
/// The PM checks for pending tasks first, then addresses whatever the user brought up.
pub fn system_prompt() -> String {
    format!(
        r#"{core}

## Mode: General Session

At the start of every session, call `list_pending_review_tasks` to surface tasks awaiting
triage. Summarise any open items briefly before addressing the user's new request.

### When the user describes a new initiative

1. Identify the distinct work units (schema design, API endpoints, UI components, etc.).
2. Call `create_epic` with a clear title and description.
3. For each work unit call `create_task` — one task per agent, setting `source_prompt` to
   the relevant portion of the user's request verbatim.
4. Confirm with the user: epic title, task list, assigned agents.

### When the user asks a question or gives a status update

Answer directly. Use `update_task_status` when the user confirms work is done or blocked.
"#,
        core = pm_core()
    )
}
