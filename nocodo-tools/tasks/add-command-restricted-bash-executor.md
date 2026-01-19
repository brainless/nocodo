# Add Command-Restricted Bash Executor to manager-tools

**Status**: ✅ Complete
**Priority**: Medium
**Created**: 2026-01-09

## Summary

Extend the bash tool in manager-tools to support agent-specific command restrictions. This enables creating specialized agents (like TesseractAgent) that can only execute specific whitelisted commands (e.g., `tesseract`) while maintaining full security and sandboxing capabilities.

## Problem Statement

Currently, the bash tool uses a global `BashPermissions` configuration that applies the same rules to all agents. This creates several limitations:

- **All-or-nothing access**: Agents either get access to all allowed commands or no bash access
- **Security concerns**: Cannot restrict specialized agents to only their required commands
- **No per-agent isolation**: A codebase agent and OCR agent would share the same bash permissions
- **Overprivileged agents**: Agents get more bash capabilities than they need

This prevents us from creating:
- Specialized agents that only need specific commands (e.g., `tesseract` for OCR)
- Secure sandboxed agents with minimal command access
- Per-agent bash permission policies

## Goals

1. **Flexible permission configuration**: Allow custom `BashPermissions` per agent/use-case
2. **Maintain security**: Preserve existing sandbox and permission validation
3. **Backward compatibility**: Don't break existing agents using default permissions
4. **Easy customization**: Simple API for creating command-restricted executors
5. **Clear documentation**: Document how to create restricted executors for new agents

## Architecture Overview

### Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Permission injection** | Pass `BashPermissions` to `BashExecutor::new()` | Allows per-executor customization |
| **Default behavior** | Keep existing default permissions | Maintains backward compatibility |
| **Builder pattern** | Optional builder for `ToolExecutor` | Clean API for custom configuration |
| **Permission rules** | Reuse existing `BashPermissions` struct | No need to duplicate logic |
| **Validation** | Existing permission checking unchanged | Security model remains intact |
| **Scope** | Per-executor, not per-request | Simpler implementation, clearer semantics |

### Current Architecture

```
ToolExecutor
  ├─ base_path: PathBuf
  └─ bash_executor: Option<BashExecutor>
       ├─ permissions: BashPermissions::default()  ← Currently hardcoded
       └─ timeout_secs: u64
```

### Proposed Architecture

```
ToolExecutor
  ├─ base_path: PathBuf
  └─ bash_executor: Option<BashExecutor>
       ├─ permissions: BashPermissions  ← Now customizable!
       └─ timeout_secs: u64

BashExecutor::new(permissions: BashPermissions, timeout_secs: u64)
  ↓
Validates commands against custom permissions
```

### Permission Customization Flow

```rust
// 1. Create custom permissions
let mut perms = BashPermissions::new();  // Empty, no rules
perms.add_rule(PermissionRule::Allow("tesseract*".to_string()));
perms.add_rule(PermissionRule::Deny("*".to_string()));  // Deny all else

// 2. Create bash executor with custom permissions
let bash_executor = BashExecutor::new(perms, 120);

// 3. Create tool executor with custom bash executor
let tool_executor = ToolExecutor::builder()
    .base_path(PathBuf::from("."))
    .bash_executor(Some(bash_executor))
    .build();

// 4. Agent uses tool executor - bash commands restricted
agent.execute_with_tools(prompt, session_id, tool_executor).await?;
```

## Implementation Plan

### Phase 1: Update BashExecutor to Accept Custom Permissions

#### 1.1 Modify BashExecutor Constructor

**File**: `manager-tools/src/bash/bash_executor.rs`

Current implementation (lines ~30-45):
```rust
impl BashExecutor {
    pub fn new() -> Self {
        Self {
            permissions: BashPermissions::default(),
            timeout_secs: 120,
        }
    }
}
```

Change to:
```rust
impl BashExecutor {
    /// Create a new BashExecutor with custom permissions
    ///
    /// # Arguments
    /// * `permissions` - Custom bash permission rules
    /// * `timeout_secs` - Command timeout in seconds (default: 120)
    ///
    /// # Examples
    ///
    /// ## Default permissions (backward compatible)
    /// ```rust
    /// use manager_tools::bash::{BashExecutor, BashPermissions};
    ///
    /// let executor = BashExecutor::new(
    ///     BashPermissions::default(),
    ///     120
    /// );
    /// ```
    ///
    /// ## Restricted to specific command
    /// ```rust
    /// use manager_tools::bash::{BashExecutor, BashPermissions, PermissionRule};
    ///
    /// let mut perms = BashPermissions::new();
    /// perms.add_rule(PermissionRule::Allow("tesseract*".to_string()));
    /// perms.add_rule(PermissionRule::Deny("*".to_string()));
    ///
    /// let executor = BashExecutor::new(perms, 120);
    /// ```
    pub fn new(permissions: BashPermissions, timeout_secs: u64) -> Self {
        Self {
            permissions,
            timeout_secs,
        }
    }

    /// Create a BashExecutor with default permissions
    ///
    /// This is a convenience method that uses the default permission set,
    /// which includes common safe commands (ls, cat, git, etc.)
    pub fn with_default_permissions() -> Self {
        Self::new(BashPermissions::default(), 120)
    }
}
```

#### 1.2 Add Helper Functions for Common Permission Patterns

**File**: `manager-tools/src/bash/bash_permissions.rs`

Add convenience methods at the end of the `BashPermissions` impl:

```rust
impl BashPermissions {
    // ... existing methods ...

    /// Create permissions that only allow specific commands
    ///
    /// # Arguments
    /// * `commands` - List of command patterns to allow (e.g., "tesseract*")
    ///
    /// # Examples
    /// ```rust
    /// let perms = BashPermissions::only_allow(vec!["tesseract*", "ls*"]);
    /// ```
    pub fn only_allow(commands: Vec<&str>) -> Self {
        let mut perms = Self::new();

        // Add allow rules for specified commands
        for cmd in commands {
            perms.add_rule(PermissionRule::Allow(cmd.to_string()));
        }

        // Deny everything else
        perms.add_rule(PermissionRule::Deny("*".to_string()));

        perms
    }

    /// Create read-only permissions (ls, cat, grep, etc.)
    pub fn read_only() -> Self {
        let mut perms = Self::new();

        perms.add_rule(PermissionRule::Allow("ls*".to_string()));
        perms.add_rule(PermissionRule::Allow("cat*".to_string()));
        perms.add_rule(PermissionRule::Allow("head*".to_string()));
        perms.add_rule(PermissionRule::Allow("tail*".to_string()));
        perms.add_rule(PermissionRule::Allow("grep*".to_string()));
        perms.add_rule(PermissionRule::Allow("find*".to_string()));
        perms.add_rule(PermissionRule::Allow("wc*".to_string()));
        perms.add_rule(PermissionRule::Allow("pwd".to_string()));

        perms.add_rule(PermissionRule::Deny("*".to_string()));

        perms
    }

    /// Create minimal permissions (only the specified command, no utilities)
    ///
    /// This is the most restrictive option - only the exact command(s) specified
    ///
    /// # Examples
    /// ```rust
    /// // Only allow tesseract command
    /// let perms = BashPermissions::minimal(vec!["tesseract"]);
    /// ```
    pub fn minimal(commands: Vec<&str>) -> Self {
        let mut perms = Self::new();

        for cmd in commands {
            // Allow exact command and with arguments
            perms.add_rule(PermissionRule::Allow(format!("{}*", cmd)));
        }

        // Deny everything else
        perms.add_rule(PermissionRule::Deny("*".to_string()));

        perms
    }
}
```

### Phase 2: Update ToolExecutor to Support Custom BashExecutor

#### 2.1 Add Builder Pattern to ToolExecutor

**File**: `manager-tools/src/tool_executor.rs`

Currently, `ToolExecutor::new()` creates bash executor internally. Add a builder:

```rust
use crate::bash::bash_executor::BashExecutor;

impl ToolExecutor {
    // Keep existing new() for backward compatibility
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            bash_executor: Some(BashExecutor::with_default_permissions()),
        }
    }

    /// Start building a ToolExecutor with custom configuration
    pub fn builder() -> ToolExecutorBuilder {
        ToolExecutorBuilder::default()
    }
}

/// Builder for creating a ToolExecutor with custom configuration
#[derive(Default)]
pub struct ToolExecutorBuilder {
    base_path: Option<PathBuf>,
    bash_executor: Option<Option<BashExecutor>>,
}

impl ToolExecutorBuilder {
    /// Set the base path for file operations
    pub fn base_path(mut self, path: PathBuf) -> Self {
        self.base_path = Some(path);
        self
    }

    /// Set a custom bash executor with specific permissions
    ///
    /// # Examples
    ///
    /// ## With custom bash executor
    /// ```rust
    /// use manager_tools::{ToolExecutor, bash::{BashExecutor, BashPermissions}};
    /// use std::path::PathBuf;
    ///
    /// let perms = BashPermissions::only_allow(vec!["tesseract*"]);
    /// let bash = BashExecutor::new(perms, 120);
    ///
    /// let executor = ToolExecutor::builder()
    ///     .base_path(PathBuf::from("."))
    ///     .bash_executor(Some(bash))
    ///     .build();
    /// ```
    ///
    /// ## Without bash executor (disable bash tool)
    /// ```rust
    /// let executor = ToolExecutor::builder()
    ///     .base_path(PathBuf::from("."))
    ///     .bash_executor(None)
    ///     .build();
    /// ```
    pub fn bash_executor(mut self, executor: Option<BashExecutor>) -> Self {
        self.bash_executor = Some(executor);
        self
    }

    /// Build the ToolExecutor
    pub fn build(self) -> ToolExecutor {
        let base_path = self.base_path.unwrap_or_else(|| PathBuf::from("."));
        let bash_executor = self.bash_executor.unwrap_or_else(|| {
            Some(BashExecutor::with_default_permissions())
        });

        ToolExecutor {
            base_path,
            bash_executor,
        }
    }
}
```

### Phase 3: Export New Public APIs

#### 3.1 Update Module Exports

**File**: `manager-tools/src/bash/mod.rs`

Ensure `BashPermissions` and `PermissionRule` are public:

```rust
pub mod bash_executor;
pub mod bash_permissions;
pub mod types;

pub use bash_executor::BashExecutor;
pub use bash_permissions::{BashPermissions, PermissionRule};
pub use types::{BashRequest, BashResponse};
```

**File**: `manager-tools/src/lib.rs`

Ensure bash module is properly exported:

```rust
pub mod bash;
pub mod tool_executor;
// ... other modules

pub use tool_executor::{ToolExecutor, ToolExecutorBuilder};
pub use bash::{BashExecutor, BashPermissions, PermissionRule};
```

### Phase 4: Documentation & Examples

#### 4.1 Update README

**File**: `manager-tools/README.md`

Add section on custom bash permissions:

```markdown
## Bash Tool Configuration

### Default Permissions

By default, the bash tool allows safe commands like `ls`, `cat`, `git`, `npm`, etc. and denies dangerous commands like `rm -rf /`, `sudo`, etc.

### Custom Permissions

You can create agents with restricted bash access:

#### Example: Tesseract-Only Agent

```rust
use manager_tools::{ToolExecutor, bash::{BashExecutor, BashPermissions}};
use std::path::PathBuf;

// Create permissions that only allow tesseract command
let perms = BashPermissions::minimal(vec!["tesseract"]);

// Create bash executor with restricted permissions
let bash_executor = BashExecutor::new(perms, 120);

// Create tool executor with custom bash
let tool_executor = ToolExecutor::builder()
    .base_path(PathBuf::from("."))
    .bash_executor(Some(bash_executor))
    .build();

// Use with agent
let agent = TesseractAgent::new(/* ... */, tool_executor);
```

#### Example: Read-Only Bash

```rust
use manager_tools::bash::BashPermissions;

// Only allow read operations
let perms = BashPermissions::read_only();
let bash_executor = BashExecutor::new(perms, 120);
```

#### Example: Multiple Specific Commands

```rust
use manager_tools::bash::BashPermissions;

// Allow tesseract and convert commands (for image processing agent)
let perms = BashPermissions::only_allow(vec!["tesseract*", "convert*"]);
let bash_executor = BashExecutor::new(perms, 120);
```

#### Example: Disable Bash Tool

```rust
// Create tool executor without bash access
let tool_executor = ToolExecutor::builder()
    .base_path(PathBuf::from("."))
    .bash_executor(None)  // No bash tool
    .build();
```

### Security Notes

- Command restrictions are evaluated using glob patterns
- First matching rule wins (allow or deny)
- Always add a catch-all deny rule at the end for restricted executors
- Sandboxing (via Codex) still applies regardless of permissions
```

#### 4.2 Add Code Examples

**File**: `manager-tools/examples/custom_bash_permissions.rs`

```rust
//! Example showing how to create tool executors with custom bash permissions

use manager_tools::{
    bash::{BashExecutor, BashPermissions, PermissionRule},
    types::{BashRequest, ToolRequest},
    ToolExecutor,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Example 1: Tesseract-only executor
    println!("Example 1: Tesseract-only permissions");
    let tesseract_perms = BashPermissions::minimal(vec!["tesseract"]);
    let tesseract_executor = create_executor_with_permissions(tesseract_perms);

    // This will succeed
    test_command(&tesseract_executor, "tesseract input.png output").await;

    // This will fail (not allowed)
    test_command(&tesseract_executor, "ls -la").await;

    println!();

    // Example 2: Read-only executor
    println!("Example 2: Read-only permissions");
    let readonly_perms = BashPermissions::read_only();
    let readonly_executor = create_executor_with_permissions(readonly_perms);

    // These will succeed
    test_command(&readonly_executor, "ls -la").await;
    test_command(&readonly_executor, "cat file.txt").await;

    // This will fail (write operation)
    test_command(&readonly_executor, "echo test > file.txt").await;

    println!();

    // Example 3: Multiple specific commands
    println!("Example 3: Multiple specific commands");
    let multi_perms = BashPermissions::only_allow(vec!["tesseract*", "convert*"]);
    let multi_executor = create_executor_with_permissions(multi_perms);

    test_command(&multi_executor, "tesseract input.png output").await;
    test_command(&multi_executor, "convert input.jpg -resize 50% output.jpg").await;
    test_command(&multi_executor, "ls -la").await;  // Will fail

    Ok(())
}

fn create_executor_with_permissions(perms: BashPermissions) -> ToolExecutor {
    let bash_executor = BashExecutor::new(perms, 120);

    ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(Some(bash_executor))
        .build()
}

async fn test_command(executor: &ToolExecutor, command: &str) {
    let request = ToolRequest::Bash(BashRequest {
        command: command.to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    match executor.execute(request).await {
        Ok(_) => println!("✓ Allowed: {}", command),
        Err(e) => println!("✗ Denied: {} - {}", command, e),
    }
}
```

### Phase 5: Testing

#### 5.1 Unit Tests for BashPermissions Helper Methods

**File**: `manager-tools/src/bash/bash_permissions.rs`

Add tests at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_allow() {
        let perms = BashPermissions::only_allow(vec!["tesseract*", "ls*"]);

        // Should allow specified commands
        assert!(perms.check_command("tesseract input.png output").is_ok());
        assert!(perms.check_command("ls -la").is_ok());

        // Should deny others
        assert!(perms.check_command("cat file.txt").is_err());
        assert!(perms.check_command("rm file.txt").is_err());
    }

    #[test]
    fn test_read_only() {
        let perms = BashPermissions::read_only();

        // Should allow read commands
        assert!(perms.check_command("ls -la").is_ok());
        assert!(perms.check_command("cat file.txt").is_ok());
        assert!(perms.check_command("grep pattern file.txt").is_ok());

        // Should deny write commands
        assert!(perms.check_command("echo test > file.txt").is_err());
        assert!(perms.check_command("rm file.txt").is_err());
    }

    #[test]
    fn test_minimal() {
        let perms = BashPermissions::minimal(vec!["tesseract"]);

        // Should allow only specified command
        assert!(perms.check_command("tesseract input.png output").is_ok());

        // Should deny everything else, including safe commands
        assert!(perms.check_command("ls -la").is_err());
        assert!(perms.check_command("cat file.txt").is_err());
    }
}
```

#### 5.2 Integration Tests

**File**: `manager-tools/tests/custom_bash_permissions.rs`

```rust
use manager_tools::{
    bash::{BashExecutor, BashPermissions},
    types::{BashRequest, ToolRequest, ToolResponse},
    ToolExecutor,
};
use std::path::PathBuf;

#[tokio::test]
async fn test_tesseract_only_permissions() {
    let perms = BashPermissions::minimal(vec!["tesseract"]);
    let bash_executor = BashExecutor::new(perms, 120);
    let executor = ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(Some(bash_executor))
        .build();

    // Tesseract should be allowed (testing permission check, not actual execution)
    let request = ToolRequest::Bash(BashRequest {
        command: "tesseract --help".to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    // Note: This will fail if tesseract isn't installed, but we're testing
    // that the permission system allows the command to attempt execution
    let result = executor.execute(request).await;

    // Either succeeds (tesseract installed) or fails with execution error (not permission error)
    // Permission errors contain "not allowed" in the message
    if let Err(e) = result {
        let error_msg = format!("{:?}", e);
        assert!(!error_msg.to_lowercase().contains("not allowed"));
    }
}

#[tokio::test]
async fn test_restricted_executor_denies_other_commands() {
    let perms = BashPermissions::minimal(vec!["tesseract"]);
    let bash_executor = BashExecutor::new(perms, 120);
    let executor = ToolExecutor::builder()
        .base_path(PathBuf::from("."))
        .bash_executor(Some(bash_executor))
        .build();

    // ls should be denied
    let request = ToolRequest::Bash(BashRequest {
        command: "ls -la".to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    let result = executor.execute(request).await;

    assert!(result.is_err());
    let error_msg = format!("{:?}", result.unwrap_err());
    assert!(error_msg.to_lowercase().contains("not allowed")
        || error_msg.to_lowercase().contains("permission denied"));
}

#[tokio::test]
async fn test_default_permissions_backward_compatibility() {
    // Using old API should still work
    let executor = ToolExecutor::new(PathBuf::from("."));

    let request = ToolRequest::Bash(BashRequest {
        command: "echo hello".to_string(),
        working_dir: None,
        timeout_secs: None,
        description: None,
    });

    let result = executor.execute(request).await;

    // Should succeed with default permissions
    assert!(result.is_ok());

    if let Ok(ToolResponse::Bash(response)) = result {
        assert!(response.stdout.contains("hello"));
    }
}
```

## Files Changed

### New Files
- `manager-tools/examples/custom_bash_permissions.rs` - Usage examples
- `manager-tools/tests/custom_bash_permissions.rs` - Integration tests
- `manager-tools/tasks/add-command-restricted-bash-executor.md` - This task document

### Modified Files
- `manager-tools/src/bash/bash_executor.rs` - Update constructor to accept permissions
- `manager-tools/src/bash/bash_permissions.rs` - Add helper methods (only_allow, read_only, minimal)
- `manager-tools/src/tool_executor.rs` - Add builder pattern
- `manager-tools/src/bash/mod.rs` - Export BashPermissions and PermissionRule
- `manager-tools/src/lib.rs` - Export ToolExecutorBuilder and bash types
- `manager-tools/README.md` - Document custom permissions

## Testing & Validation

### Unit Tests
```bash
cd manager-tools
cargo test bash_permissions
```

### Integration Tests
```bash
cd manager-tools
cargo test custom_bash_permissions
```

### Run Example
```bash
cd manager-tools
cargo run --example custom_bash_permissions
```

### Full Build & Quality Checks
```bash
cd manager-tools
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

## Success Criteria

- [ ] `BashExecutor::new()` accepts custom `BashPermissions`
- [ ] `BashExecutor::with_default_permissions()` provides backward compatibility
- [ ] `BashPermissions::only_allow()` helper implemented
- [ ] `BashPermissions::read_only()` helper implemented
- [ ] `BashPermissions::minimal()` helper implemented
- [ ] `ToolExecutor::builder()` pattern implemented
- [ ] Builder supports custom bash executor
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Example code compiles and runs
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Documentation updated
- [ ] Backward compatibility maintained

## Usage by Agents

Once this is complete, agents in the `nocodo-agents` crate can use it like this:

```rust
// In TesseractAgent::new()
let perms = BashPermissions::minimal(vec!["tesseract"]);
let bash_executor = BashExecutor::new(perms, 120);

let tool_executor = ToolExecutor::builder()
    .base_path(base_path)
    .bash_executor(Some(bash_executor))
    .build();

// Agent now has bash access, but only for tesseract command
```

## Notes

- This is a non-breaking change - existing code using `ToolExecutor::new()` continues to work
- The builder pattern is optional - only needed for custom configuration
- Permission validation happens at execution time (existing behavior)
- Sandboxing via Codex still applies in addition to permission rules
- This enables the principle of least privilege for agent bash access
