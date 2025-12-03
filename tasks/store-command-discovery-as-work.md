# Store Command Discovery as Work Item

## Objective
Store the entire command discovery process (LLM interactions, tool calls, responses) as a proper Work item instead of using `project_id` as a placeholder `work_id`. This enables full history tracking, replay capability, and consistency with regular AI work sessions.

## Implementation Plan

### 1. Modify `enhance_discovery_with_llm()` in `manager/src/handlers/project_commands.rs:527-586`

**Changes:**
- Remove `_` from `_db` parameter (line 529) - we'll need it
- Create a Work item before creating LlmAgentSession
- Use the real `work_id` instead of `project_id` placeholder

**Code:**
```rust
// After line 535, replace the placeholder work_id with:
let work = Work {
    id: 0,
    title: format!("Command Discovery for Project {}", project_id),
    project_id: Some(project_id),
    model: Some(model.clone()),
    status: "active".to_string(),
    created_at: chrono::Utc::now().timestamp(),
    updated_at: chrono::Utc::now().timestamp(),
    git_branch: None,
    working_directory: Some(project_path.to_string_lossy().to_string()),
};

let work_id = db.create_work(&work)
    .map_err(|e| AppError::Internal(format!("Failed to create work item: {}", e)))?;

info!("Created work item {} for command discovery", work_id);
```

### 2. Optional: Add initial user message to Work
```rust
// After creating work item
db.create_work_message(&WorkMessage {
    id: 0,
    work_id,
    content: "Discovering commands for this project...".to_string(),
    content_type: MessageContentType::Text,
    author_type: MessageAuthorType::User,
    author_id: Some("system".to_string()),
    sequence_order: 0,
    created_at: chrono::Utc::now().timestamp(),
})?;
```

### 3. Update function signature
Change line 529 from:
```rust
_db: &Arc<Database>,
```
to:
```rust
db: &Arc<Database>,
```

## Result
- LlmAgentSession properly linked to Work via real work_id
- All messages already stored in llm_agent_messages (no change needed)
- All tool calls already stored in llm_agent_tool_calls (no change needed)
- Full command discovery history available through Work item
