# Desktop App UI Conventions

**Last Updated**: November 29, 2025
**Status**: Production Guidelines

This document defines the UI conventions and design patterns used in the nocodo desktop application, particularly for rendering AI tool interactions and work details.

---

## Table of Contents

1. [Tool Widget Rendering Strategy](#tool-widget-rendering-strategy)
2. [Widget Styling Patterns](#widget-styling-patterns)
3. [Interaction Patterns](#interaction-patterns)
4. [State Management](#state-management)
5. [Message Display Patterns](#message-display-patterns)
6. [Form and Modal Styling](#form-and-modal-styling)

---

## Tool Widget Rendering Strategy

### Design Philosophy

**Established**: November 22, 2025 (commits `912a909`, `b3095ff`, `b4ec398`)

**Core Principle**: Minimize UI clutter while maintaining access to technical details.

### Key Decisions

#### 1. Don't Show Tool Requests Separately

**Rationale**:
- Text explanations from the AI model are already shown via extracted "text" field from assistant messages
- Separate tool request widgets create redundancy
- Reduces UI clutter

**Implementation**:
```rust
// Don't display list_files tool requests - only show responses
// Don't display read_file tool requests - only show responses
// Don't display bash tool requests separately

// ❌ OLD: Separate request widget + separate response widget
// ✅ NEW: Single combined widget showing response with request context
```

**Example**:
- AI says: "Let me check the files in your project directory"
- Tool request: `list_files({ path: "/home/user/project" })`
- **Don't show**: Separate widget saying "🤖 📁 List files: /home/user/project"
- **Do show**: Only the response widget: "▶ Listed 20 files in /home/user/project"

#### 2. Combine Request + Response in Single Widget

**Pattern**: One collapsible widget per tool call that combines information from both request and response.

**Collapsed Summary Format**:
```
▶ {action_description}
```

**Examples**:
- `▶ Listed 20 files in /home/user/project`
- `▶ Read config.json (1.2 KB)`
- `▶ Wrote server.rs (350 lines)`
- `▶ npm install`

**Expanded Format**:
Shows full details including:
- Request parameters (if relevant)
- Full response data
- Execution metadata (status, timing, etc.)

**Code Pattern**:
```rust
let is_expanded = state.ui_state.expanded_tool_calls.contains(&tool_call_id);

let summary = format!(
    "{} {} in {}",
    if is_expanded { "▼" } else { "▶" },
    response.action_description(),
    request.path
);

if is_expanded {
    // Show full request JSON
    // Show full response JSON
    // Show execution details
}
```

#### 3. No Emojis in Tool Widgets

**Decision**: Remove emoji icons for cleaner, more professional UI.

**Before**:
```
🤖 📁 Listed 20 files in /home/user/project
```

**After**:
```
▶ Listed 20 files in /home/user/project
```

**Exception**: Emojis may still be used in AI assistant text messages if the model generates them.

#### 4. Hide/Show Tools Toggle

**Default State**: Tools hidden (`show_tool_widgets: false`)

**Toggle Location**: Work details header, far right of metadata line

**Button Text**:
- When hidden: "Show tools"
- When visible: "Hide tools"

**Purpose**:
- Provides cleaner initial view
- Hides technical execution details by default
- Users can toggle on when debugging or investigating

**Code Pattern**:
```rust
// Only render tool widgets when show_tool_widgets is true
if state.ui_state.show_tool_widgets {
    // Render bash requests
    // Render list_files responses
    // Render read_file responses
    // Render write_file responses
    // etc.
}
```

#### 5. Collapsed by Default

**State Management**: Use `state.ui_state.expanded_tool_calls: HashSet<i64>` to track which tool calls are expanded.

**Default Behavior**:
- Tool widgets render in collapsed state
- Click anywhere on widget to toggle expansion
- Expansion state persists during session

**Visual Indicators**:
- Arrow: `▶` (collapsed) / `▼` (expanded)
- Cursor changes to pointer on hover
- Entire widget is clickable (not just the arrow)

---

## Widget Styling Patterns

### Tool Widget Style

**Reference Implementation**: `desktop-app/src/pages/board.rs:502-566` (bash tool widget)

```rust
let bg_color = ui.style().visuals.widgets.inactive.bg_fill;

let response = egui::Frame::NONE
    .fill(bg_color)
    .corner_radius(0.0)  // No rounded corners
    .inner_margin(egui::Margin::symmetric(12, 6))  // 12px horizontal, 6px vertical
    .show(ui, |ui| {
        ui.set_width(ui.available_width());  // Full width
        ui.vertical(|ui| {
            // Header row - clickable
            let header_response = ui.horizontal(|ui| {
                let arrow = if is_expanded { "▼" } else { "▶" };
                ui.label(egui::RichText::new(arrow).size(12.0));
                ui.label(egui::RichText::new(summary).size(12.0).strong());
            }).response;

            // Expanded content
            if is_expanded {
                // Show details
            }

            header_response
        }).inner
    })
    .response;

// Handle click to toggle expansion
if response.interact(egui::Sense::click()).clicked() {
    if is_expanded {
        state.ui_state.expanded_tool_calls.remove(&tool_call_id);
    } else {
        state.ui_state.expanded_tool_calls.insert(tool_call_id);
    }
}

// Change cursor to pointer on hover
if response.hovered() {
    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
}

ui.add_space(4.0);  // Spacing after widget
```

**Key Properties**:
- **Background**: `ui.style().visuals.widgets.inactive.bg_fill`
- **Corners**: `corner_radius(0.0)` - sharp corners, not rounded
- **Margin**: `egui::Margin::symmetric(12, 6)` - 12px left/right, 6px top/bottom
- **Width**: Full available width
- **Spacing**: 4.0px after each widget

### Message Widget Styles

#### User Message
```rust
let bg_color = ui.style().visuals.widgets.inactive.bg_fill;

egui::Frame::NONE
    .fill(bg_color)
    .corner_radius(0.0)
    .inner_margin(egui::Margin::symmetric(12, 6))
    .show(ui, |ui| {
        // User message content
    });
```

#### AI Assistant Message
```rust
let bg_color = ui.style().visuals.widgets.noninteractive.bg_fill;

egui::Frame::NONE
    .fill(bg_color)
    .corner_radius(0.0)
    .inner_margin(egui::Margin::symmetric(12, 6))
    .show(ui, |ui| {
        // AI assistant content
    });
```

**Difference**: User messages use `inactive.bg_fill`, AI messages use `noninteractive.bg_fill`.

---

## Interaction Patterns

### Collapsible Widgets

**Standard Pattern**:

1. **Clickable Area**: Entire widget is clickable (not just the arrow)
2. **Visual Feedback**:
   - Arrow changes: `▶` → `▼`
   - Cursor changes to pointer on hover
3. **State Persistence**: Expansion state stored in `HashSet<i64>` for session
4. **No Animation**: Instant expand/collapse (no transitions)

**Code Template**:
```rust
let is_expanded = state.ui_state.expanded_tool_calls.contains(&id);

let response = egui::Frame::NONE
    .fill(bg_color)
    .corner_radius(0.0)
    .inner_margin(egui::Margin::symmetric(12, 6))
    .show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            let header_response = ui.horizontal(|ui| {
                let arrow = if is_expanded { "▼" } else { "▶" };
                ui.label(egui::RichText::new(arrow).size(12.0));
                ui.label(egui::RichText::new(title).size(12.0).strong());
            }).response;

            if is_expanded {
                // Expanded content
            }

            header_response
        }).inner
    })
    .response;

if response.interact(egui::Sense::click()).clicked() {
    if is_expanded {
        state.ui_state.expanded_tool_calls.remove(&id);
    } else {
        state.ui_state.expanded_tool_calls.insert(id);
    }
}

if response.hovered() {
    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
}
```

### Toggle Buttons

**Example**: Show/Hide tools button

**Pattern**:
- Text changes based on state: "Show X" / "Hide X"
- Action toggles boolean flag
- UI re-renders based on flag

```rust
let button_text = if state.ui_state.show_tool_widgets {
    "Hide tools"
} else {
    "Show tools"
};

if ui.button(button_text).clicked() {
    state.ui_state.show_tool_widgets = !state.ui_state.show_tool_widgets;
}
```

---

## State Management

### UI State Fields

**Location**: `desktop-app/src/state/ui_state.rs`

#### Tool Display State

```rust
pub struct UiState {
    /// Whether to show tool widgets (default: false)
    pub show_tool_widgets: bool,

    /// Set of tool call IDs that are currently expanded
    pub expanded_tool_calls: HashSet<i64>,

    // ... other fields
}
```

#### Scroll State

```rust
pub struct UiState {
    /// Whether to reset scroll position in work details (default: false)
    pub reset_work_details_scroll: bool,

    // ... other fields
}
```

**Pattern**: Set to `true` when work item changes, scroll area consumes it and sets back to `false`.

```rust
// In work selection logic
if selected_work_changed {
    state.ui_state.reset_work_details_scroll = true;
}

// In scroll area
let mut scroll_area = egui::ScrollArea::vertical();
if state.ui_state.reset_work_details_scroll {
    scroll_area = scroll_area.vertical_scroll_offset(0.0);
    state.ui_state.reset_work_details_scroll = false;
}
```

---

## Message Display Patterns

### Message Timeline

**Pattern**: Combine all messages (user + AI) and sort by timestamp.

```rust
#[derive(Clone)]
enum DisplayMessage {
    WorkMessage(manager_models::WorkMessage),
    AiOutput(manager_models::AiSessionOutput),
    // Future: ToolCall(manager_models::LlmAgentToolCall),
}

let mut all_messages: Vec<(i64, DisplayMessage)> = Vec::new();

// Add work messages (user input)
for msg in &state.work_messages {
    all_messages.push((msg.created_at, DisplayMessage::WorkMessage(msg.clone())));
}

// Add AI session outputs (AI responses)
for output in &state.ai_session_outputs {
    all_messages.push((output.created_at, DisplayMessage::AiOutput(output.clone())));
}

// Future: Add tool calls
// for tool_call in &state.ai_tool_calls {
//     all_messages.push((tool_call.created_at, DisplayMessage::ToolCall(tool_call.clone())));
// }

// Sort by timestamp
all_messages.sort_by_key(|(timestamp, _)| *timestamp);

// Render in chronological order
for (_timestamp, message) in &all_messages {
    match message {
        DisplayMessage::WorkMessage(msg) => { /* render user message */ }
        DisplayMessage::AiOutput(output) => { /* render AI message + embedded tools */ }
        // DisplayMessage::ToolCall(tool_call) => { /* render tool call widget */ }
    }
}
```

### Extracting Assistant Text

**Pattern**: AI messages may contain both text and tool calls in JSON format.

```rust
let assistant_text = if output.role.as_deref() == Some("assistant") {
    if let Ok(assistant_data) = serde_json::from_str::<serde_json::Value>(&output.content) {
        assistant_data.get("text").and_then(|t| t.as_str()).map(|s| s.to_string())
    } else {
        None
    }
} else {
    None
};

if let Some(ref text) = assistant_text {
    if !text.trim().is_empty() {
        // Show assistant text message
        AiMessageRenderer::render_text(ui, text, output.model.as_deref(), output.created_at);
    }
}
```

### Parsing Tool Calls from Messages

**Pattern**: Tool calls are embedded in assistant messages in `tool_calls` array.

```rust
if let Ok(assistant_data) = serde_json::from_str::<serde_json::Value>(&output.content) {
    if let Some(tool_calls) = assistant_data.get("tool_calls").and_then(|tc| tc.as_array()) {
        for tool_call in tool_calls {
            if let Some(function) = tool_call.get("function") {
                if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                    if name == "bash" {
                        if let Some(args) = function.get("arguments").and_then(|a| a.as_str()) {
                            match serde_json::from_str::<BashRequest>(args) {
                                Ok(bash_req) => {
                                    // Process bash request
                                }
                                Err(e) => {
                                    tracing::warn!(error = %e, "Failed to parse BashRequest");
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
```

---

## Form and Modal Styling

**Reference**: See `specs/DESKTOP_APP_STYLING.md` for comprehensive modal/form styling specifications.

### Quick Reference

**Modal Window**:
- Fixed width: `320.0` pixels
- Auto height: `0.0`
- Not collapsible, not resizable

**Form Fields**:
- Vertical spacing: `4.0` pixels between fields
- Full width inputs: `desired_width(f32::INFINITY)`
- Inner margin: `egui::Margin::same(4)`

**CTA Buttons**:
- Separator before button row
- Spacing after separator: `4.0` pixels
- Button padding: `egui::vec2(6.0, 4.0)`

---

## Implementation Checklist for New Tool Widgets

When implementing rendering for `state.ai_tool_calls` or any new tool type:

- [ ] **Don't render separate request widgets** - only responses
- [ ] **Combine request + response info** in one widget
- [ ] **Use collapsed summary format**: `▶ {action}: {brief_summary}`
- [ ] **Implement expand/collapse** with `expanded_tool_calls` HashSet
- [ ] **Respect `show_tool_widgets` flag** - only render when true
- [ ] **Use standard widget styling** - `inactive.bg_fill`, `corner_radius(0.0)`, etc.
- [ ] **Make entire widget clickable** for expand/collapse
- [ ] **Change cursor to pointer** on hover
- [ ] **Show arrow**: `▶` collapsed, `▼` expanded
- [ ] **Use 12.0pt font size** for text
- [ ] **Add 4.0px spacing** after widget
- [ ] **No emojis** in tool widgets
- [ ] **Full width** - `ui.set_width(ui.available_width())`

---

## Future Considerations

### Planned: Render `state.ai_tool_calls`

**Context**: Currently tool calls are only shown when embedded in `ai_session_outputs`. The API returns all tool calls separately via `/api/work/{id}/tool-calls` but desktop app doesn't render them.

**Task**: Add rendering for `state.ai_tool_calls` following these conventions.

**Approach**:
1. Add `ToolCall` variant to `DisplayMessage` enum
2. Include `state.ai_tool_calls` in combined message timeline
3. Sort chronologically by `created_at`
4. Render each tool call using standard widget pattern
5. Generate summary from `tool_name`, `request`, and `response` fields
6. Handle all tool types: `list_files`, `read_file`, `write_file`, `grep`, `apply_patch`, `bash`

**Example Summary Formats**:
- `list_files`: `▶ Listed {count} files in {path}`
- `read_file`: `▶ Read {filename} ({size})`
- `write_file`: `▶ Wrote {filename} ({lines} lines)`
- `grep`: `▶ Found {matches} matches for "{pattern}" in {path}`
- `apply_patch`: `▶ Applied {changes} changes`
- `bash`: `▶ {command}` (first 100 chars)

**Data Structure** (`manager-models/src/lib.rs:808-821`):
```rust
pub struct LlmAgentToolCall {
    pub id: i64,
    pub session_id: i64,
    pub message_id: Option<i64>,
    pub tool_name: String,                    // "list_files", "read_file", etc.
    pub request: serde_json::Value,           // Tool request parameters
    pub response: Option<serde_json::Value>,  // Tool response data
    pub status: String,                       // "pending", "executing", "completed", "failed"
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub execution_time_ms: Option<i64>,
    pub progress_updates: Option<String>,
    pub error_details: Option<String>,
}
```

---

## Related Documentation

- **`specs/DESKTOP_APP_STYLING.md`** - Modal/form styling specifications
- **`specs/AGENT_ARCHITECTURE.md`** - LLM agent tool system architecture
- **`tasks/desktop-app-display-all-tool-calls.md`** - Task for rendering `ai_tool_calls`

---

## Version History

- **November 29, 2025**: Initial documentation based on established patterns
- **November 22, 2025**: Implementation of core tool rendering strategy (commits `912a909`, `b3095ff`, `b4ec398`)

---

## Summary

The desktop app UI follows a **minimal, clean approach** to tool rendering:

1. **Don't show requests separately** - AI explanations are enough
2. **Combine request + response** in single collapsible widget
3. **Collapsed by default** with brief summary
4. **No emojis** in technical widgets
5. **Hidden by default** with toggle to show/hide all tools
6. **Consistent styling** across all widget types
7. **Click anywhere to expand** with pointer cursor feedback

This creates a clean, professional UI that doesn't overwhelm users with technical details while keeping them accessible when needed.
