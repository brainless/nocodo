# Bash Tool Implementation Plan for Nocodo Manager

## Executive Summary

This document outlines the plan to implement a Bash tool in the nocodo manager's LLM agent. **We will reuse battle-tested crates from OpenAI's Codex project** to accelerate development and ensure high quality.

**Key Decision**: Reuse `codex-core`, `codex-process-hardening`, and other Codex crates instead of building from scratch.

**Time Savings**: 50-55% reduction (34-48 days → 14-22 days)
**Code Reduction**: 69-81% less code to write (1,700-2,400 lines → 450-750 lines)

📖 **See [CODEX_CRATES_REUSE.md](./CODEX_CRATES_REUSE.md)** for detailed crate analysis, integration examples, and rationale.

---

## Table of Contents

1. [Why Reuse Codex Crates?](#why-reuse-codex-crates)
2. [Current State](#current-state)
3. [Implementation Phases](#implementation-phases)
4. [Architecture Overview](#architecture-overview)
5. [Security Model](#security-model)
6. [Testing Strategy](#testing-strategy)
7. [Success Metrics](#success-metrics)

---

## Why Reuse Codex Crates?

### Nocodo Already Uses Codex

**File**: `manager/Cargo.toml:49`
```toml
codex-apply-patch = { git = "https://github.com/openai/codex", package = "codex-apply-patch" }
```

We have a proven integration path! ✅

### Benefits

| Aspect | From Scratch | With Codex Crates | Benefit |
|--------|-------------|-------------------|---------|
| **Development Time** | 34-48 days | 14-22 days | **50-55% faster** |
| **Code to Write** | 1,700-2,400 lines | 450-750 lines | **69-81% less** |
| **Quality** | Unknown | Production-tested | **Battle-proven** |
| **Security** | Needs audit | Pre-audited | **OpenAI reviewed** |
| **Platform Support** | Gradual | Immediate | **Cross-platform ready** |
| **Complexity** | High | Moderate | **Less to learn** |

### What We Get from Codex

- ✅ **Complete execution engine** with timeouts, signals, streaming
- ✅ **Process hardening** with security checks
- ✅ **OS-level sandboxing** (Landlock, Seatbelt, Restricted Token)
- ✅ **Background process management**
- ✅ **Event streaming architecture**
- ✅ **Edge case handling** (encoding, zombies, signals)

📖 **Detailed analysis**: [CODEX_CRATES_REUSE.md](./CODEX_CRATES_REUSE.md)

---

## Current State

### Existing Nocodo Infrastructure

**File**: `manager/src/llm_agent.rs`

✅ **Already has**:
- Tool system with 5 tools (`list_files`, `read_file`, `write_file`, `grep`, `apply_patch`)
- Tool executor with project-specific sandboxing
- Provider adapter pattern for LLM integration
- WebSocket broadcasting for real-time updates
- Database persistence for tool execution history
- Error handling and recovery

✅ **Already uses**:
- `tokio` async runtime
- `serde` / `serde_json` for serialization
- `anyhow` for errors
- `async-trait` for traits
- `regex` for patterns

### Gaps for Bash Tool

❌ **Missing**:
- Process execution infrastructure
- Timeout management
- OS-level sandboxing
- Background process support
- Output streaming from subprocesses

**Solution**: Add Codex crates to fill these gaps!

---

## Implementation Phases

### Phase 1: Core Execution (Week 1-2) - MVP

**Goal**: Execute commands with timeout and permissions

#### Dependencies to Add

```toml
# Core execution
codex-core = { git = "https://github.com/openai/codex", package = "codex-core" }
codex-process-hardening = { git = "https://github.com/openai/codex", package = "codex-process-hardening" }

# Utilities
async-channel = "2.3"
signal-hook = "0.3"
signal-hook-tokio = { version = "0.3", features = ["futures-v0_3"] }
glob = "0.3"
```

#### Tasks

**1. Create BashExecutor Wrapper** (`manager/src/bash_executor.rs`)
```rust
use codex_core::exec::{execute_command, CommandOptions};
use codex_process_hardening::harden_command;

pub struct BashExecutor {
    project_path: PathBuf,
    config: BashToolConfig,
}

impl BashExecutor {
    pub async fn execute(&self, command: &str, timeout_ms: u64) -> Result<BashResult> {
        // 1. Check permissions
        self.check_permission(command)?;

        // 2. Configure execution
        let options = CommandOptions {
            command: command.to_string(),
            working_dir: self.project_path.clone(),
            timeout_ms: Some(timeout_ms),
            ..Default::default()
        };

        // 3. Execute with codex-core
        let result = execute_command(options).await?;

        // 4. Format for nocodo
        Ok(BashResult::from(result))
    }
}
```

**2. Create Permission Manager** (`manager/src/bash_permissions.rs`)
```rust
use glob::Pattern;

pub struct PermissionManager {
    allowed_patterns: Vec<CommandPermission>,
}

impl PermissionManager {
    pub fn check_permission(&self, command: &str) -> Result<()> {
        for perm in &self.allowed_patterns {
            let pattern = Pattern::new(&perm.pattern)?;
            if pattern.matches(command) {
                return Ok(());
            }
        }

        Err(PermissionError::Denied {
            command: command.to_string(),
            message: "Command not in allowed list".to_string(),
        })
    }
}
```

**3. Integrate with ToolExecutor** (`manager/src/tools.rs`)
```rust
pub enum ToolName {
    // ... existing tools
    Bash,
}

// In execute_tool method:
ToolName::Bash => {
    let command = args["command"].as_str().unwrap();
    let timeout = args.get("timeout")
        .and_then(|v| v.as_u64())
        .unwrap_or(120000);

    let executor = BashExecutor::new(&self.project_path, &self.config);
    let result = executor.execute(command, timeout).await?;

    serde_json::to_value(result)?
}
```

**4. Add Tool Schema** (`manager/src/llm_agent.rs`)
```json
{
  "name": "bash",
  "description": "Execute bash commands with timeout. Use for git, npm, cargo, testing, etc.",
  "input_schema": {
    "type": "object",
    "properties": {
      "command": {
        "type": "string",
        "description": "The bash command to execute"
      },
      "timeout": {
        "type": "number",
        "description": "Optional timeout in milliseconds (default: 120000, max: 600000)"
      },
      "description": {
        "type": "string",
        "description": "Clear description of what this command does (5-10 words)"
      }
    },
    "required": ["command"]
  }
}
```

**5. Add Configuration** (`manager/src/models.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashToolConfig {
    pub enabled: bool,
    pub default_timeout_ms: u64,
    pub max_timeout_ms: u64,
    pub max_output_chars: usize,
    pub allowed_commands: Vec<CommandPermission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandPermission {
    pub pattern: String,  // e.g., "git *", "npm install"
    pub description: String,
    pub requires_approval: bool,
}
```

#### Default Permissions (Conservative)

```rust
vec![
    CommandPermission {
        pattern: "git status".to_string(),
        description: "Check git status".to_string(),
        requires_approval: false,
    },
    CommandPermission {
        pattern: "git log*".to_string(),
        description: "View git history".to_string(),
        requires_approval: false,
    },
    CommandPermission {
        pattern: "git diff*".to_string(),
        description: "View git diffs".to_string(),
        requires_approval: false,
    },
    CommandPermission {
        pattern: "ls*".to_string(),
        description: "List files".to_string(),
        requires_approval: false,
    },
    CommandPermission {
        pattern: "cargo build*".to_string(),
        description: "Build Rust project".to_string(),
        requires_approval: false,
    },
    CommandPermission {
        pattern: "cargo test*".to_string(),
        description: "Run tests".to_string(),
        requires_approval: false,
    },
]
```

#### Deliverables

- ✅ Execute simple commands (`git status`, `ls`, `cargo build`)
- ✅ Timeout after configured duration (default 120s)
- ✅ Permission checking with wildcard patterns
- ✅ Structured output (exit code, stdout, stderr, duration)
- ✅ Integration with existing tool system

**Estimated Effort**: 3-5 days (vs. 10-14 days from scratch)
**Files Created**: 3 new files, ~300-400 lines total

---

### Phase 2: Linux Sandboxing (Week 3-4)

**Goal**: OS-level security with Landlock

#### Dependencies to Add

```toml
codex-linux-sandbox = { git = "https://github.com/openai/codex", package = "codex-linux-sandbox" }

[target.'cfg(target_os = "linux")'.dependencies]
landlock = "0.4"
seccompiler = "0.4"
```

#### Tasks

**1. Create Sandbox Module** (`manager/src/sandbox/mod.rs`)
```rust
pub trait Sandbox {
    fn apply(&self) -> Result<()>;
    fn is_supported() -> bool;
}

pub fn create_sandbox(project_path: &Path, config: &SandboxConfig) -> Result<Box<dyn Sandbox>> {
    #[cfg(target_os = "linux")]
    return Ok(Box::new(LinuxSandbox::new(project_path, config)?));

    #[cfg(target_os = "macos")]
    return Ok(Box::new(MacSandbox::new(project_path, config)?));

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    Ok(Box::new(NoopSandbox))
}
```

**2. Linux Implementation** (`manager/src/sandbox/linux.rs`)
```rust
use codex_linux_sandbox::{LandlockSandbox, SandboxConfig as CodexConfig};

pub struct LinuxSandbox {
    inner: LandlockSandbox,
}

impl LinuxSandbox {
    pub fn new(project_path: &Path, config: &SandboxConfig) -> Result<Self> {
        let codex_config = CodexConfig {
            allowed_read_paths: vec![
                project_path.to_path_buf(),
                PathBuf::from("/usr"),
                PathBuf::from("/lib"),
            ],
            allowed_write_paths: vec![project_path.to_path_buf()],
            deny_network: !config.allow_network,
        };

        Ok(Self {
            inner: LandlockSandbox::new(codex_config)?,
        })
    }
}

impl Sandbox for LinuxSandbox {
    fn apply(&self) -> Result<()> {
        self.inner.apply()
    }

    fn is_supported() -> bool {
        LandlockSandbox::is_supported()
    }
}
```

**3. Update BashExecutor** (add sandbox application)
```rust
impl BashExecutor {
    pub async fn execute(&self, command: &str, timeout_ms: u64) -> Result<BashResult> {
        // ... permission check ...

        // Apply sandbox if enabled
        if self.config.sandbox_enabled {
            let sandbox = create_sandbox(&self.project_path, &self.config)?;
            sandbox.apply()?;
        }

        // ... execute with codex-core ...
    }
}
```

#### Deliverables

- ✅ Linux sandboxing with Landlock
- ✅ Filesystem access restrictions
- ✅ Network deny option
- ✅ Kernel version fallback

**Estimated Effort**: 3-5 days (vs. 7-10 days from scratch)
**Files Created**: 2 new files, ~150-200 lines total

---

### Phase 3: Background Processes (Week 5-6)

**Goal**: Long-running commands in background

#### Dependencies to Add

```toml
codex-exec = { git = "https://github.com/openai/codex", package = "codex-exec" }
```

#### Tasks

**1. Create Process Manager** (`manager/src/process_manager.rs`)
```rust
use codex_exec::{ProcessRegistry, ProcessEvent};

pub struct BashProcessManager {
    registry: ProcessRegistry,
    event_tx: Sender<ProcessEvent>,
    db: Arc<Mutex<Connection>>,
}

impl BashProcessManager {
    pub async fn spawn_background(&self, command: &str) -> Result<String> {
        let process_id = self.registry.spawn(command).await?;

        // Store in database
        self.store_process(&process_id, command).await?;

        // Broadcast event
        self.broadcast_started(&process_id).await?;

        Ok(process_id)
    }

    pub async fn get_output(&self, process_id: &str) -> Result<String> {
        self.registry.get_output(process_id).await
    }

    pub async fn kill_process(&self, process_id: &str) -> Result<()> {
        self.registry.kill(process_id).await?;
        self.broadcast_killed(process_id).await
    }
}
```

**2. Add Background Tool Variants** (`manager/src/tools.rs`)
```rust
pub enum ToolName {
    // ... existing ...
    Bash,
    BashBackground,
    BashOutput,
    BashKill,
}
```

**3. Add Tool Schemas** (`manager/src/llm_agent.rs`)
```json
{
  "name": "bash_background",
  "description": "Start a long-running command in background",
  "input_schema": {
    "type": "object",
    "properties": {
      "command": {"type": "string"},
      "filter": {
        "type": "string",
        "description": "Optional regex to filter output lines"
      }
    },
    "required": ["command"]
  }
}
```

**4. WebSocket Event Streaming**
```rust
// Stream process output in real-time
async fn stream_process_output(process_id: &str, ws: &WebSocket) {
    let mut stream = registry.subscribe_output(process_id);

    while let Some(chunk) = stream.next().await {
        ws.send(json!({
            "type": "bash_output_chunk",
            "process_id": process_id,
            "data": chunk,
        })).await?;
    }
}
```

#### Deliverables

- ✅ Background process execution
- ✅ Real-time output streaming via WebSocket
- ✅ Process management (list, kill)
- ✅ Database persistence of background processes

**Estimated Effort**: 5-7 days (vs. 10-14 days from scratch)
**Files Created**: 1 new file + updates, ~200-300 lines total

---

### Phase 4: macOS & Polish (Week 7-8)

**Goal**: Cross-platform support and UX improvements

#### Dependencies to Add

```toml
codex-sandboxing = { git = "https://github.com/openai/codex", package = "codex-sandboxing" }

[target.'cfg(target_os = "macos")'.dependencies]
codex-macos-sandbox = { git = "https://github.com/openai/codex", package = "codex-macos-sandbox" }
```

#### Tasks

**1. macOS Sandbox** (`manager/src/sandbox/macos.rs`)
```rust
use codex_macos_sandbox::SeatbeltSandbox;

pub struct MacSandbox {
    inner: SeatbeltSandbox,
}

impl Sandbox for MacSandbox {
    fn apply(&self) -> Result<()> {
        self.inner.apply()
    }

    fn is_supported() -> bool {
        true
    }
}
```

**2. Permission Management API**
- `GET /api/projects/{id}/bash_permissions` - List permissions
- `POST /api/projects/{id}/bash_permissions` - Add permission
- `DELETE /api/projects/{id}/bash_permissions/{pattern}` - Remove

**3. Interactive Approval Flow**
```rust
// When command requires approval
pub async fn execute_with_approval(&self, command: &str) -> Result<BashResult> {
    if self.requires_approval(command) {
        // Send WebSocket event
        self.broadcast_approval_required(command).await?;

        // Wait for user decision (60s timeout)
        let approved = self.wait_for_approval(command, 60).await?;

        if !approved {
            return Err(PermissionError::UserDenied);
        }
    }

    self.execute(command).await
}
```

**4. Command History & Suggestions**
- Store successful commands in database
- Suggest based on frequency and recency
- Auto-complete in UI

**5. Output Formatting**
- Syntax highlighting for errors, file paths, URLs
- Collapsible long output
- Download output option

#### Deliverables

- ✅ macOS sandboxing with Seatbelt
- ✅ Permission management UI/API
- ✅ Interactive approval flow
- ✅ Command history and suggestions
- ✅ Polished output formatting

**Estimated Effort**: 4-5 days (vs. 8-10 days from scratch)
**Files Created**: 2-3 new files + updates, ~300-400 lines total

---

## Architecture Overview

### Component Diagram

```
┌─────────────────────────────────────────────────────┐
│                  LlmAgent                           │
│  (nocodo existing - no changes)                     │
│  - Tool call extraction                             │
│  - Database persistence                             │
│  - WebSocket broadcasting                           │
└───────────────────┬─────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────┐
│               ToolExecutor                          │
│  (nocodo existing - add Bash variant)               │
│  - Project path validation                          │
│  - Tool dispatch                                    │
└───────────────────┬─────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────────────┐
│            BashExecutor (NEW)                       │
│  - Permission checking (glob)                       │
│  - Wraps codex-core                                 │
│  - Formats results for nocodo                       │
└───────────────────┬─────────────────────────────────┘
                    │
        ┌───────────┴────────────┐
        ▼                        ▼
┌──────────────────┐   ┌──────────────────────┐
│  codex-core      │   │  PermissionManager   │
│  (EXTERNAL)      │   │  (NEW - simple)      │
│  - Execution     │   │  - Pattern matching  │
│  - Timeout       │   │  - Audit logging     │
│  - Streaming     │   └──────────────────────┘
└────────┬─────────┘
         │
         ▼
┌──────────────────────┐
│ codex-process-       │
│ hardening            │
│ (EXTERNAL)           │
│ - Security checks    │
└──────────────────────┘
         │
         ▼
┌──────────────────────┐
│ Sandbox (NEW)        │
│ - codex-linux-       │
│   sandbox            │
│ - codex-macos-       │
│   sandbox            │
└──────────────────────┘
```

### Code Volume Breakdown

| Component | Lines of Code | Source |
|-----------|---------------|--------|
| **Codex Crates** (reused) | ~1,500 lines | External |
| **BashExecutor** (wrapper) | ~150 lines | New |
| **PermissionManager** | ~200 lines | New |
| **Sandbox module** | ~150 lines | New |
| **ProcessManager** (wrapper) | ~200 lines | New |
| **ToolExecutor integration** | ~50 lines | Modified |
| **Configuration** | ~100 lines | New |
| **Tests** | ~200 lines | New |
| **Total new code** | **~1,050 lines** | - |

**vs. From Scratch**: 1,700-2,400 lines
**Reduction**: 56% less code to write and maintain

---

## Security Model

### Three-Layer Defense

**Layer 1: Permission ACLs** (Phase 1)
- Default-deny with explicit allowlist
- Wildcard pattern matching (`git *`, `cargo build*`)
- Per-command approval flags
- Audit logging of all attempts

**Layer 2: OS Sandboxing** (Phase 2-4)
- Landlock (Linux): Filesystem restrictions
- Seatbelt (macOS): Profile-based isolation
- Restricted Token (Windows): Privilege reduction
- Network deny option

**Layer 3: Process Hardening** (All phases)
- Environment sanitization
- Working directory validation
- Resource limits
- Signal handling

### Permission Examples

**Auto-approved** (read-only):
```
git status
git log*
git diff*
ls*
find*
grep*
```

**Auto-approved** (builds):
```
cargo build*
cargo test*
cargo check*
npm install
npm run build
pytest*
```

**Requires approval**:
```
git push*
git commit*
rm -rf*
npm publish*
```

**Always denied**:
```
rm -rf /
sudo*
curl * | bash
eval*
```

### Sandbox Configuration

```rust
pub struct SandboxConfig {
    pub enabled: bool,
    pub allow_network: bool,
    pub allowed_read_paths: Vec<PathBuf>,
    pub allowed_write_paths: Vec<PathBuf>,
}

// Default configuration
SandboxConfig {
    enabled: true,  // if platform supports
    allow_network: false,
    allowed_read_paths: vec![
        project_path.clone(),
        PathBuf::from("/usr"),
        PathBuf::from("/lib"),
    ],
    allowed_write_paths: vec![
        project_path.clone(),
    ],
}
```

📖 **Security details**: [CODEX_CRATES_REUSE.md - Security Section](./CODEX_CRATES_REUSE.md#security-considerations)

---

## Testing Strategy

### Unit Tests

**PermissionManager** (`manager/src/bash_permissions.rs`):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_wildcard_matching() {
        let mgr = PermissionManager::new(/* ... */);
        assert!(mgr.check_permission("git status").is_ok());
        assert!(mgr.check_permission("git commit -m 'test'").is_ok());
        assert!(mgr.check_permission("rm -rf /").is_err());
    }

    #[test]
    fn test_exact_matching() {
        let mgr = PermissionManager::new(/* ... */);
        assert!(mgr.check_permission("npm install").is_ok());
        assert!(mgr.check_permission("npm install --force").is_err());
    }
}
```

**BashExecutor** (integration with codex-core):
```rust
#[tokio::test]
async fn test_simple_command() {
    let executor = BashExecutor::new(/* ... */);
    let result = executor.execute("echo 'hello'", 5000).await.unwrap();
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout.trim(), "hello");
}

#[tokio::test]
async fn test_timeout() {
    let executor = BashExecutor::new(/* ... */);
    let result = executor.execute("sleep 10", 1000).await.unwrap();
    assert!(result.timed_out);
}
```

### Integration Tests

**End-to-end tool execution**:
```rust
#[tokio::test]
async fn test_bash_tool_via_llm_agent() {
    let agent = create_test_agent().await;
    let session = agent.create_session(/* ... */).await.unwrap();

    let tool_call = json!({
        "name": "bash",
        "input": {
            "command": "git status",
            "description": "Check git status"
        }
    });

    let result = agent.execute_tool(&session, tool_call).await.unwrap();
    assert_eq!(result["exit_code"], 0);
}
```

### Manual Testing Checklist

**Phase 1**:
- [ ] Execute `git status` in a git repo
- [ ] Execute `cargo build` in nocodo project
- [ ] Test timeout with `sleep 300` (5s timeout)
- [ ] Test permission denial with `rm -rf /`
- [ ] Test nonexistent command `asdfxyz123`

**Phase 2**:
- [ ] Execute sandboxed command on Linux
- [ ] Verify cannot read `/etc/passwd`
- [ ] Verify cannot write to `/tmp`
- [ ] Test with old kernel (fallback)

**Phase 3**:
- [ ] Start `cargo build` in background
- [ ] Poll for output updates
- [ ] Kill background process
- [ ] Verify WebSocket streaming

**Phase 4**:
- [ ] Test on macOS with Seatbelt
- [ ] Approve/deny via UI
- [ ] View command history
- [ ] Test output formatting

---

## Success Metrics

### Phase 1 (MVP)
- [ ] Execute 10+ different commands successfully
- [ ] Timeout works (test with `sleep`)
- [ ] Permission denials work
- [ ] Zero crashes or panics
- [ ] Clear error messages

### Overall Success
- **Functionality**: 95%+ commands execute correctly
- **Security**: Zero permission bypasses, zero sandbox escapes
- **Performance**: <100ms overhead vs. direct shell
- **Reliability**: <1% failure rate for valid commands
- **UX**: <5 seconds avg. time from request to result

---

## Implementation Checklist

### Phase 1: Core Execution (Week 1-2)
- [ ] Add `codex-core`, `codex-process-hardening`, utilities to Cargo.toml
- [ ] Create `manager/src/bash_executor.rs`
  - [ ] Wrapper around `codex-core::exec::execute_command`
  - [ ] Permission checking
  - [ ] Result formatting
- [ ] Create `manager/src/bash_permissions.rs`
  - [ ] Pattern matching with `glob`
  - [ ] Audit logging
- [ ] Update `manager/src/tools.rs`
  - [ ] Add `ToolName::Bash` variant
  - [ ] Add execution branch
- [ ] Update `manager/src/llm_agent.rs`
  - [ ] Add bash tool schema
- [ ] Add `BashToolConfig` to `manager/src/models.rs`
- [ ] Write tests
  - [ ] Unit tests for permissions
  - [ ] Integration tests for execution
- [ ] Test manually with 10+ commands

### Phase 2: Linux Sandboxing (Week 3-4)
- [ ] Add `codex-linux-sandbox`, `landlock`, `seccompiler` to Cargo.toml
- [ ] Create `manager/src/sandbox/mod.rs`
  - [ ] `Sandbox` trait
  - [ ] `create_sandbox()` factory
- [ ] Create `manager/src/sandbox/linux.rs`
  - [ ] Wrapper around `codex-linux-sandbox`
  - [ ] Path configuration
- [ ] Update `BashExecutor` to apply sandbox
- [ ] Test on Linux with various commands
- [ ] Test fallback on old kernel

### Phase 3: Background Processes (Week 5-6)
- [ ] Add `codex-exec` to Cargo.toml
- [ ] Create `manager/src/process_manager.rs`
  - [ ] Wrapper around `codex-exec::ProcessRegistry`
  - [ ] Database integration
  - [ ] WebSocket events
- [ ] Add `BashBackground`, `BashOutput`, `BashKill` tool variants
- [ ] Add schemas for background tools
- [ ] Implement real-time output streaming
- [ ] Test with long-running commands

### Phase 4: macOS & Polish (Week 7-8)
- [ ] Add `codex-macos-sandbox`, `codex-sandboxing` to Cargo.toml
- [ ] Create `manager/src/sandbox/macos.rs`
- [ ] Implement permission management API
- [ ] Implement interactive approval flow
- [ ] Add command history feature
- [ ] Polish output formatting
- [ ] Write documentation
- [ ] Final testing across all platforms

---

## Timeline Summary

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| **Phase 1** | 1-2 weeks | Core execution with permissions |
| **Phase 2** | 1-2 weeks | Linux sandboxing |
| **Phase 3** | 1-2 weeks | Background processes |
| **Phase 4** | 1-2 weeks | macOS + UX polish |
| **Total** | **4-8 weeks** | Production-ready Bash tool |

**vs. From Scratch**: 7-12 weeks
**Time Saved**: 3-4 weeks (40-50%)

---

## Next Steps

### Immediate (This Week)

1. **Review this plan**
   - Get team approval
   - Clarify any questions
   - Prioritize phases

2. **Review Codex crates**
   - Read [CODEX_CRATES_REUSE.md](./CODEX_CRATES_REUSE.md)
   - Study key files:
     - `~/Projects/codex/codex-rs/core/src/exec.rs`
     - `~/Projects/codex/codex-rs/core/src/bash.rs`
     - `~/Projects/codex/codex-rs/process-hardening/src/lib.rs`

3. **Prototype** (1-2 days)
   ```bash
   cd /home/brainless/Projects/nocodo/manager

   # Add codex-core to Cargo.toml
   # Write simple wrapper in bash_executor.rs
   # Test with "echo hello"
   ```

4. **Make decision**
   - If prototype works → proceed with Phase 1
   - If issues → assess alternatives

### Short-term (Next 2 Weeks)

- Complete Phase 1 (MVP)
- Internal testing
- Gather feedback

### Medium-term (Next 1-2 Months)

- Complete Phases 2-3
- Beta release
- Security audit

### Long-term (3+ Months)

- Complete Phase 4
- Full GA release
- Monitor and improve

---

## Key Files Reference

### To Create (New)
- `manager/src/bash_executor.rs` (~150 lines)
- `manager/src/bash_permissions.rs` (~200 lines)
- `manager/src/process_manager.rs` (~200 lines)
- `manager/src/sandbox/mod.rs` (~50 lines)
- `manager/src/sandbox/linux.rs` (~100 lines)
- `manager/src/sandbox/macos.rs` (~100 lines)

### To Modify (Existing)
- `manager/Cargo.toml` (add dependencies)
- `manager/src/tools.rs` (add Bash tool variants)
- `manager/src/llm_agent.rs` (add tool schemas)
- `manager/src/models.rs` (add BashToolConfig)

### To Study (Reference)
- `~/Projects/codex/codex-rs/core/src/exec.rs` - Execution engine
- `~/Projects/codex/codex-rs/core/src/bash.rs` - Bash parsing
- `~/Projects/codex/codex-rs/process-hardening/src/lib.rs` - Security
- `~/Projects/codex/codex-rs/linux-sandbox/src/landlock.rs` - Linux sandbox

---

## Resources

- **Detailed Crate Analysis**: [CODEX_CRATES_REUSE.md](./CODEX_CRATES_REUSE.md)
- **Nocodo Architecture**: [AGENT_ARCHITECTURE.md](./AGENT_ARCHITECTURE.md)
- **Codex Repository**: https://github.com/openai/codex
- **Landlock Docs**: https://www.kernel.org/doc/html/latest/userspace-api/landlock.html
- **tokio Process**: https://docs.rs/tokio/latest/tokio/process/

---

## Questions?

Open an issue or discussion in the nocodo repository with:
- Tag: `bash-tool`
- Reference: This document and CODEX_CRATES_REUSE.md
