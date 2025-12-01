# Desktop App: Command Discovery UI Implementation

## Overview

This task implements the command discovery and management UI in the desktop app's Project Detail page. All required manager API endpoints for command discovery and execution are already implemented (see `tasks/project-commands-with-llm-integration.md` Phase 1-3).

## Prerequisites (Already Complete)

The manager has these API endpoints ready:
- `POST /api/projects/{id}/commands/discover` - Discover commands with LLM detection
- `GET /api/projects/{id}/commands` - List saved commands
- `POST /api/projects/{id}/commands` - Create/save commands
- `POST /api/projects/{id}/commands/{cmd_id}/execute` - Execute a command
- `GET /api/projects/{id}/commands/{cmd_id}/executions` - Get execution history

Data structures available from API:
```rust
ProjectCommand {
    id: String,
    project_id: i64,
    name: String,
    description: Option<String>,
    command: String,
    shell: Option<String>,
    working_directory: Option<String>,
    environment: Option<HashMap<String, String>>,
    timeout_seconds: Option<u64>,
    os_filter: Option<Vec<String>>,
    created_at: i64,
    updated_at: i64,
}
```

## Current State Analysis

### Existing Components

**Project Detail Page**: `desktop-app/src/pages/project_detail.rs`
- Commands tab already exists (line 186-189, currently shows placeholder)
- Branch selection already exists (lines 92-155)
- Files tab demonstrates two-column layout pattern (lines 190-221)

**Board Page CTA Pattern**: `desktop-app/src/pages/board.rs` (lines 62-117)
- Pastel green button: `Color32::from_rgb(144, 238, 144)` normal, `Color32::from_rgb(152, 251, 152)` hover
- Dark green text: `Color32::from_rgb(40, 80, 40)`
- Font: `ui_semibold`, size 16.0
- Padding: 12px margin, corner radius 8.0
- Cursor: PointingHand on hover

**Branch State Usage** (from Files tab):
- Selected branch stored in: `state.ui_state.project_detail_selected_branch`
- Branch list in: `state.project_detail_worktree_branches`
- Loading state: `state.loading_project_detail_worktree_branches`
- Branch passed to API calls as: `git_branch: Option<&str>`

### API Client Status

**Current**: `desktop-app/src/api_client.rs`
- NO command discovery methods exist yet
- Need to add methods for all command endpoints

## Implementation Plan

### Phase 1: API Client Integration

**File**: `desktop-app/src/api_client.rs`

Add these methods to `ApiClient`:

```rust
/// Discover commands for a project using LLM detection
pub async fn discover_project_commands(
    &self,
    project_id: i64,
    use_llm: Option<bool>,
) -> Result<serde_json::Value, ApiError> {
    let mut url = format!("{}/api/projects/{}/commands/discover", self.base_url, project_id);

    if let Some(use_llm_val) = use_llm {
        url.push_str(&format!("?use_llm={}", use_llm_val));
    }

    let response = self.client.post(&url).send().await?;
    // Handle response...
}

/// List all saved commands for a project
pub async fn list_project_commands(
    &self,
    project_id: i64,
) -> Result<Vec<serde_json::Value>, ApiError> {
    let url = format!("{}/api/projects/{}/commands", self.base_url, project_id);
    // GET request...
}

/// Save/create project commands (bulk)
pub async fn create_project_commands(
    &self,
    project_id: i64,
    commands: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, ApiError> {
    // POST multiple commands...
}

/// Execute a specific command
pub async fn execute_project_command(
    &self,
    project_id: i64,
    command_id: &str,
    git_branch: Option<&str>,
) -> Result<serde_json::Value, ApiError> {
    let url = format!("{}/api/projects/{}/commands/{}/execute",
        self.base_url, project_id, command_id);

    let body = serde_json::json!({
        "git_branch": git_branch,
    });

    // POST request with body...
}
```

### Phase 2: State Management

**File**: `desktop-app/src/state/ui_state.rs`

Add new UI state fields:

```rust
// Around line 61-63 (near project_detail_selected_branch)
pub project_detail_command_discovery_results: Option<serde_json::Value>,
pub project_detail_command_selected_items: HashSet<String>,  // Set of selected command names/IDs
pub project_detail_show_discovery_form: bool,
```

**File**: `desktop-app/src/state/mod.rs`

Add new app state fields:

```rust
// Around line 70 (near project_detail_worktree_branches)
pub project_detail_saved_commands: Vec<serde_json::Value>,
pub loading_project_detail_commands: bool,
pub project_detail_commands_fetch_attempted: bool,
pub loading_command_discovery: bool,
pub executing_command_id: Option<String>,
```

### Phase 3: Commands Tab UI Layout

**File**: `desktop-app/src/pages/project_detail.rs`

Replace the Commands tab placeholder (lines 186-189) with a two-column layout:

```rust
ProjectDetailTab::Commands => {
    // Two-column layout similar to Files tab
    ui.horizontal(|ui| {
        // LEFT COLUMN (400px wide)
        ui.allocate_ui_with_layout(
            egui::vec2(400.0, ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                // 1. "Discover commands" CTA button (gray color scheme)
                // 2. List of saved commands
            },
        );

        // RIGHT COLUMN (remaining width)
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), ui.available_height()),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                // Show discovery form or command details
            },
        );
    });
}
```

### Phase 4: Left Column - CTA Button

Implement the "Discover commands" CTA in the left column:

**Requirements**:
- Use **gray color scheme** (light/dark gray), NOT the green from Board page
- Same size and styling as Board page CTA, just different colors
- Text: "Discover commands"
- On click: Set `state.ui_state.project_detail_show_discovery_form = true`

**Styling reference** (adapt from Board page lines 62-117):
```rust
// Gray color scheme (example, adjust as needed)
let bg_color_normal = Color32::from_rgb(200, 200, 200);   // Light gray
let bg_color_hover = Color32::from_rgb(220, 220, 220);    // Lighter gray
let text_color = Color32::from_rgb(40, 40, 40);           // Dark gray
let border_color_normal = Color32::from_rgb(180, 180, 180);
let border_color_hover = Color32::from_rgb(160, 160, 160);

// Same sizing as Board page
let font_id = egui::FontId {
    size: 16.0,
    family: egui::FontFamily::Name("ui_semibold".into()),
};
```

### Phase 5: Left Column - Saved Commands List

Below the CTA button, show a list of saved commands:

```rust
// Fetch saved commands on tab load (if not already fetched)
if !state.project_detail_commands_fetch_attempted {
    // Trigger API call: api_client.list_project_commands(project_id)
    // Store in state.project_detail_saved_commands
}

// Display list
egui::ScrollArea::vertical().show(ui, |ui| {
    for command in &state.project_detail_saved_commands {
        // Show command name, description
        // Clickable to show details in right column
    }
});
```

### Phase 6: Right Column - Discovery Form

When `state.ui_state.project_detail_show_discovery_form == true`, show discovery UI:

**Step 1: Trigger Discovery**
```rust
// Add a button to start discovery
if ui.button("Start Discovery").clicked() {
    // Trigger API call: api_client.discover_project_commands(project_id, Some(true))
    // use_llm = true for LLM detection
    // Store results in state.ui_state.project_detail_command_discovery_results
}
```

**Step 2: Display Discovered Commands**
```rust
if let Some(results) = &state.ui_state.project_detail_command_discovery_results {
    // Parse results.commands array
    // For each command, show:
    ui.horizontal(|ui| {
        // 1. Checkbox (selection state in state.ui_state.project_detail_command_selected_items)
        let mut is_selected = state.ui_state.project_detail_command_selected_items
            .contains(&command_name);

        if ui.checkbox(&mut is_selected, "").changed() {
            if is_selected {
                state.ui_state.project_detail_command_selected_items.insert(command_name);
            } else {
                state.ui_state.project_detail_command_selected_items.remove(&command_name);
            }
        }

        // 2. Command name
        ui.label(&command.name);
    });

    // 3. Command description (if available)
    if let Some(desc) = &command.description {
        ui.label(desc);
    }
}
```

**Step 3: Save Selected Commands**
```rust
if ui.button("Save selected commands").clicked() {
    // Filter selected commands from discovery results
    let selected_commands: Vec<_> = /* filter based on selected_items */;

    // Trigger API call: api_client.create_project_commands(project_id, selected_commands)

    // On success:
    // - Clear selection: state.ui_state.project_detail_command_selected_items.clear()
    // - Hide form: state.ui_state.project_detail_show_discovery_form = false
    // - Refresh saved commands list
}
```

### Phase 7: Command Execution (Future Enhancement)

**Note**: This is NOT part of the initial task but planned for future work.

```rust
// When user clicks a saved command, show execution UI
// - Branch selection (use current branch from state.ui_state.project_detail_selected_branch)
// - Execute button
// - Show execution results (stdout, stderr, exit code)

if ui.button("Execute").clicked() {
    let branch = state.ui_state.project_detail_selected_branch.as_deref();
    // api_client.execute_project_command(project_id, command_id, branch)
}
```

## Implementation Checklist

### API Client (`desktop-app/src/api_client.rs`)
- [ ] Add `discover_project_commands` method
- [ ] Add `list_project_commands` method
- [ ] Add `create_project_commands` method (batch save)
- [ ] Add `execute_project_command` method

### State Management
- [ ] Add UI state fields to `ui_state.rs`
  - [ ] `project_detail_command_discovery_results`
  - [ ] `project_detail_command_selected_items`
  - [ ] `project_detail_show_discovery_form`
- [ ] Add app state fields to `state/mod.rs`
  - [ ] `project_detail_saved_commands`
  - [ ] `loading_project_detail_commands`
  - [ ] `project_detail_commands_fetch_attempted`
  - [ ] `loading_command_discovery`
  - [ ] `executing_command_id`

### Commands Tab Layout (`desktop-app/src/pages/project_detail.rs`)
- [ ] Replace placeholder with two-column layout
- [ ] Implement left column (400px wide)
  - [ ] Add "Discover commands" CTA with gray color scheme
  - [ ] Add saved commands list with ScrollArea
  - [ ] Implement command list item click handling
- [ ] Implement right column
  - [ ] Add discovery form UI
  - [ ] Add "Start Discovery" trigger button
  - [ ] Implement discovered commands list display
  - [ ] Add checkbox selection for each command
  - [ ] Add "Save selected commands" button
  - [ ] Handle save operation and state updates

### API Service Integration (`desktop-app/src/services/api.rs`)
- [ ] Add async task for `discover_project_commands`
- [ ] Add async task for `list_project_commands`
- [ ] Add async task for `create_project_commands`
- [ ] Handle loading states appropriately
- [ ] Handle error cases and display to user

### Testing
- [ ] Test command discovery with LLM enabled
- [ ] Test command selection/deselection
- [ ] Test saving selected commands
- [ ] Test saved commands list refresh
- [ ] Test branch context awareness (verify branch state is available for future execution)
- [ ] Test loading states and error handling

## Design Specifications

### Colors

**CTA Button (Gray Scheme)**:
- Background normal: `Color32::from_rgb(200, 200, 200)` (light gray)
- Background hover: `Color32::from_rgb(220, 220, 220)` (lighter gray)
- Border normal: `Color32::from_rgb(180, 180, 180)`
- Border hover: `Color32::from_rgb(160, 160, 160)`
- Text: `Color32::from_rgb(40, 40, 40)` (dark gray)

**Other UI Elements**:
- Use default egui theme colors for lists, checkboxes, etc.

### Typography
- CTA text: `ui_semibold`, size 16.0
- Command names: Default UI font
- Command descriptions: Smaller/lighter font

### Layout Dimensions
- Left column width: 400px
- CTA button padding: 12px
- CTA button corner radius: 8.0
- Right column: Fill remaining width

### Interaction Patterns
- CTA hover: Change background color and border color
- CTA cursor: PointingHand
- Checkboxes: Standard egui checkbox behavior
- Command list items: Clickable to show details (future)

## Branch Context Integration

The Commands tab should respect the current branch selection:
- Branch selector is already implemented (lines 92-155)
- Current branch stored in: `state.ui_state.project_detail_selected_branch`
- For future command execution: Pass this branch to `execute_project_command(project_id, command_id, git_branch)`

## API Response Format Examples

**Discovery Response**:
```json
{
  "commands": [
    {
      "name": "install",
      "description": "Install project dependencies",
      "command": "npm install",
      "shell": "bash",
      "working_directory": null,
      "environment": null,
      "timeout_seconds": null,
      "os_filter": null
    }
  ],
  "project_types": ["Node.js (Npm)"],
  "reasoning": "Detected Node.js project with package.json",
  "discovered_count": 5,
  "stored_count": 0,
  "llm_used": true
}
```

**Saved Commands Response**:
```json
[
  {
    "id": "uuid-here",
    "project_id": 1,
    "name": "install",
    "description": "Install project dependencies",
    "command": "npm install",
    "shell": "bash",
    "working_directory": null,
    "environment": null,
    "timeout_seconds": 120,
    "os_filter": null,
    "created_at": 1234567890,
    "updated_at": 1234567890
  }
]
```

## Error Handling

- Discovery fails: Show error message in right column
- Save fails: Show error toast/notification
- API timeout: Show loading spinner with timeout message
- No commands discovered: Show "No commands found" message with option to retry

## Future Enhancements (Out of Scope)

These are explicitly NOT part of this task but noted for future work:
- Command execution UI
- Real-time output streaming for running commands
- Execution history display
- Edit/delete saved commands
- Command templates
- Custom command creation form
- Command favorites/pinning
- Search/filter saved commands

## References

- Manager API implementation: `tasks/project-commands-with-llm-integration.md`
- Manager API endpoints: `manager/src/main.rs` lines 185-239
- Manager models: `manager/src/models.rs` lines 1135-1179
- Board page CTA pattern: `desktop-app/src/pages/board.rs` lines 62-117
- Files tab layout: `desktop-app/src/pages/project_detail.rs` lines 190-221
- Branch selector: `desktop-app/src/pages/project_detail.rs` lines 92-155
