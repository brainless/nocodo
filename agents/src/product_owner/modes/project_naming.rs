use super::core::po_core;

/// Mode: name the project based on completed requirements intake.
///
/// Called once by the backend immediately after `complete_requirements` is received.
/// PO has the full conversation history as context. Single task: call `set_project_name`.
pub fn system_prompt() -> String {
    format!(
        r#"{core}

## Mode: Project Naming

Requirements intake is complete. You have the full conversation history.

Your only task is to call `set_project_name` with a concise, descriptive name derived from
the user's domain.

### Rules

- Derive the name from the conversation — do not ask questions.
- Keep it under 60 characters.
- Examples: "CRM — Leads & Deals", "Inventory Tracker", "Volunteer Shift Manager".
- Call `set_project_name` exactly once, then stop."#,
        core = po_core()
    )
}
