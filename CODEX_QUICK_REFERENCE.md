# Codex - Quick Reference for Nocodo Bash Tool

## Most Important Assets

### 1. Core Execution Engine
**Module**: `codex-core` (`/home/brainless/Projects/codex/codex-rs/core/`)

**Files to Study**:
- `exec.rs` - Process execution, timeouts, output streaming (693 lines)
- `spawn.rs` - Cross-platform process spawning (108 lines)
- `bash.rs` - Safe bash command parsing with tree-sitter (290+ lines)
- `sandboxing/mod.rs` - Sandbox manager pattern

**Key Patterns to Copy**:
1. Timeout handling with `tokio::select!`
2. Output streaming with async channels
3. Real-time delta event emission
4. Sandbox integration layer

### 2. Event-Driven Architecture
**Module**: `codex-exec` (`/home/brainless/Projects/codex/codex-rs/exec/`)

**Pattern**: Separate execution from output handling
- Emit events as process runs
- Don't wait for completion to stream output
- Support both human and machine-readable formats

### 3. Linux Sandboxing
**Module**: `codex-linux-sandbox` (`/home/brainless/Projects/codex/codex-rs/linux-sandbox/`)

**Direct Reuse Options**:
- Option A: Use as separate binary (call as subprocess)
- Option B: Embed the module directly in nocodo
- Option C: Copy patterns for your own implementation

**Key Features**:
- Landlock filesystem restrictions
- Seccomp network filtering
- Safe defaults (read-only root, writable paths configurable)

### 4. Security Hardening
**Module**: `codex-process-hardening` (`/home/brainless/Projects/codex/codex-rs/process-hardening/`)

**What It Does**:
- Disable ptrace attachment
- Disable core dumps
- Remove dangerous environment variables
- Called automatically via `#[ctor::ctor]`

---

## Copy These Code Patterns

### Pattern 1: Timeout with Signal Handling
```rust
// From: codex-rs/core/src/exec.rs (lines 508-527)

let (exit_status, timed_out) = tokio::select! {
    result = tokio::time::timeout(timeout, child.wait()) => {
        match result {
            Ok(status_result) => (status_result?, false),
            Err(_) => {
                child.start_kill()?;  // Kill process on timeout
                (synthetic_exit_status(TIMEOUT_CODE), true)
            }
        }
    }
    _ = tokio::signal::ctrl_c() => {
        child.start_kill()?;  // Handle Ctrl+C gracefully
        (synthetic_exit_status(SIGKILL_CODE), false)
    }
};
```

### Pattern 2: Real-Time Output Streaming
```rust
// From: codex-rs/core/src/exec.rs (lines 493-549)

// Create channels for output aggregation
let (agg_tx, agg_rx) = async_channel::unbounded::<Vec<u8>>();

// Spawn concurrent tasks for stdout and stderr
let stdout_handle = tokio::spawn(read_capped(
    BufReader::new(stdout_reader),
    stdout_stream.clone(),  // Stream for real-time events
    false,
    Some(agg_tx.clone()),   // Aggregation channel
));

// Wait for completion and collect output
drop(agg_tx);  // Close to signal end
let mut combined_buf = Vec::new();
while let Ok(chunk) = agg_rx.recv().await {
    combined_buf.extend_from_slice(&chunk);
}
```

### Pattern 3: Delta Event Emission During Execution
```rust
// From: codex-rs/core/src/exec.rs (lines 552-604)

async fn read_capped<R: AsyncRead + Unpin + Send + 'static>(
    mut reader: R,
    stream: Option<StdoutStream>,
    is_stderr: bool,
    aggregate_tx: Option<Sender<Vec<u8>>>,
) -> io::Result<StreamOutput<Vec<u8>>> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let mut emitted_deltas: usize = 0;
    
    loop {
        let n = reader.read(&mut tmp).await?;
        if n == 0 { break; }
        
        // Emit real-time delta event
        if let Some(stream) = &stream 
            && emitted_deltas < MAX_EXEC_OUTPUT_DELTAS_PER_CALL {
            let chunk = tmp[..n].to_vec();
            let msg = EventMsg::OutputDelta(OutputDeltaEvent {
                call_id: stream.call_id.clone(),
                stream: if is_stderr { Stderr } else { Stdout },
                chunk,
            });
            let _ = stream.tx_event.send(Event { ... }).await;
            emitted_deltas += 1;
        }
        
        // Also send to aggregation channel
        if let Some(tx) = &aggregate_tx {
            let _ = tx.send(tmp[..n].to_vec()).await;
        }
        
        buf.extend_from_slice(&tmp[..n]);
    }
    Ok(StreamOutput { text: buf, ... })
}
```

### Pattern 4: Cross-Platform Sandbox Selection
```rust
// From: codex-rs/core/src/sandboxing/mod.rs (lines 88-142)

pub(crate) fn transform(
    &self,
    spec: &CommandSpec,
    policy: &SandboxPolicy,
    sandbox: SandboxType,
    sandbox_policy_cwd: &Path,
    codex_linux_sandbox_exe: Option<&PathBuf>,
) -> Result<ExecEnv, SandboxTransformError> {
    let (command, sandbox_env, arg0_override) = match sandbox {
        SandboxType::None => (command, HashMap::new(), None),
        SandboxType::MacosSeatbelt => {
            // Wrap with seatbelt executable
            let args = create_seatbelt_command_args(command, policy, sandbox_policy_cwd);
            // ... create seatbelt-wrapped command ...
        }
        SandboxType::LinuxSeccomp => {
            // Wrap with codex-linux-sandbox executable
            let exe = codex_linux_sandbox_exe.ok_or(...)?;
            let args = create_linux_sandbox_command_args(command, policy, sandbox_policy_cwd);
            // ... create sandbox-wrapped command ...
        }
        #[cfg(target_os = "windows")]
        SandboxType::WindowsRestrictedToken => {
            // In-process sandbox via windows-sandbox crate
            (command, HashMap::new(), None)
        }
    };
    Ok(ExecEnv { command, sandbox_env, ... })
}
```

### Pattern 5: Safe Bash Command Parsing
```rust
// From: codex-rs/core/src/bash.rs (lines 24-89)

pub fn parse_shell_lc_plain_commands(command: &[String]) -> Option<Vec<Vec<String>>> {
    let [shell, flag, script] = command else {
        return None;
    };
    
    if flag != "-lc" || !(shell == "bash" || shell == "zsh") {
        return None;
    }
    
    let tree = try_parse_shell(script)?;
    try_parse_word_only_commands_sequence(&tree, script)
}

// Validates only simple commands with safe operators:
// - Allowed operators: &&, ||, ;, |
// - Rejected: parentheses, redirections, substitutions, control flow
// - Safe for executing untrusted commands
```

### Pattern 6: Process Hardening
```rust
// From: codex-rs/process-hardening/src/lib.rs

pub fn pre_main_hardening() {
    #[cfg(target_os = "linux")]
    {
        // Disable ptrace attachment
        unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
        
        // Disable core dumps
        let rlim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
        unsafe { libc::setrlimit(libc::RLIMIT_CORE, &rlim) };
        
        // Remove LD_* environment variables
        std::env::vars()
            .filter_map(|(k, _)| if k.starts_with("LD_") { Some(k) } else { None })
            .for_each(|k| unsafe { std::env::remove_var(k) });
    }
}
```

### Pattern 7: Filesystem Restrictions (Linux)
```rust
// From: codex-rs/linux-sandbox/src/landlock.rs (lines 59-82)

fn install_filesystem_landlock_rules(writable_roots: Vec<PathBuf>) -> Result<()> {
    let abi = ABI::V5;
    let access_rw = AccessFs::from_all(abi);
    let access_ro = AccessFs::from_read(abi);
    
    let ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(access_rw)?
        .create()?
        .add_rules(landlock::path_beneath_rules(&["/"], access_ro))?  // Root: read-only
        .add_rules(landlock::path_beneath_rules(&["/dev/null"], access_rw))?  // /dev/null: R/W
        .set_no_new_privs(true);
    
    if !writable_roots.is_empty() {
        ruleset.add_rules(landlock::path_beneath_rules(&writable_roots, access_rw))?
    }
    
    ruleset.restrict_self()?;
    Ok(())
}
```

---

## Critical Dependencies to Add to Cargo.toml

```toml
[dependencies]
tokio = { version = "1", features = [
    "io-std",
    "macros",
    "process",
    "rt-multi-thread",
    "signal",
    "time",
] }
async-channel = "2"
async-trait = "0.1"
shlex = "1.3"
tree-sitter = "0.25"
tree-sitter-bash = "0.25"

[target.'cfg(target_os = "linux")'.dependencies]
landlock = "0.4"
seccompiler = "0.5"
libc = "0.2"

[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_System_Threading",
    "Win32_System_Pipes",
    "Win32_Security",
    # ... other features as needed
] }
```

---

## Which Files to Copy/Reference

### Definitely Copy These:
1. `/home/brainless/Projects/codex/codex-rs/core/src/exec.rs` - Core timeout & streaming logic
2. `/home/brainless/Projects/codex/codex-rs/core/src/bash.rs` - Safe bash parsing
3. `/home/brainless/Projects/codex/codex-rs/process-hardening/src/lib.rs` - Security hardening
4. `/home/brainless/Projects/codex/codex-rs/linux-sandbox/src/landlock.rs` - Sandbox implementation

### Use as Reference (Don't Copy Directly):
1. `/home/brainless/Projects/codex/codex-rs/exec/src/lib.rs` - Event loop architecture
2. `/home/brainless/Projects/codex/codex-rs/core/src/spawn.rs` - Process spawning patterns
3. `/home/brainless/Projects/codex/codex-rs/core/src/sandboxing/mod.rs` - Sandbox abstraction

---

## Quick Start for Nocodo Implementation

### Step 1: Core Execution (1-2 days)
- [ ] Add tokio with process features
- [ ] Copy timeout pattern from codex-core/exec.rs
- [ ] Implement basic bash tool command execution
- [ ] Add timeout support

### Step 2: Output Streaming (1 day)
- [ ] Add async-channel dependency
- [ ] Implement real-time output delta events
- [ ] Test with long-running commands

### Step 3: Linux Sandbox (2-3 days)
- [ ] Add landlock and seccompiler dependencies
- [ ] Either:
  - Option A: Copy codex-linux-sandbox module directly
  - Option B: Wrap codex-linux-sandbox as external binary
- [ ] Test sandbox restrictions

### Step 4: Other Platforms (3-5 days)
- [ ] Windows: Integrate or wrap windows-sandbox crate
- [ ] macOS: Implement Seatbelt wrapper (moderate complexity)
- [ ] Implement SandboxManager abstraction layer

### Step 5: Security Hardening (1 day)
- [ ] Integrate process-hardening module
- [ ] Add PR_SET_PDEATHSIG for child cleanup
- [ ] Test on target platforms

---

## Common Mistakes to Avoid

1. **Don't block on output reading**
   - Use async I/O (tokio)
   - Emit delta events while process still running
   - Don't wait for process completion before streaming

2. **Don't ignore timeouts**
   - Use tokio::time::timeout() for ALL process waits
   - Call child.start_kill() on timeout
   - Return timeout status in result

3. **Don't handle sandboxing platform-agnostically**
   - Use conditional compilation or runtime selection
   - Each platform has different mechanisms
   - Use SandboxManager pattern for abstraction

4. **Don't leak child processes**
   - Set PR_SET_PDEATHSIG on Linux
   - Use job objects on Windows
   - Enable kill_on_drop for processes

5. **Don't trust user input in bash commands**
   - Use bash.rs parsing to validate commands
   - Only allow simple commands with safe operators
   - Reject complex constructs

---

## Testing Your Implementation

### Required Tests:
1. [ ] Process execution returns correct exit code
2. [ ] Output is captured completely
3. [ ] Timeout kills process
4. [ ] Ctrl+C is handled gracefully
5. [ ] Child processes are cleaned up on parent exit
6. [ ] Sandbox restrictions work (Linux)
7. [ ] Real-time output events are emitted
8. [ ] Large outputs are handled correctly

### Performance Benchmarks:
1. Simple command execution: < 100ms
2. Output streaming latency: < 50ms per delta
3. Sandbox overhead: < 10% CPU
4. Memory usage: < 50MB for typical operations

---

## Getting Help

For specific implementation questions:
1. Check `CODEX_ANALYSIS.md` for detailed info
2. Look at file references section for line numbers
3. Reference Codex source directly (well-documented)
4. Test patterns incrementally

---

## Summary

The Codex project provides:
- **Proven patterns** for async process execution
- **Battle-tested code** for timeouts and output streaming
- **Sandbox implementations** for Linux, Windows, macOS
- **Security hardening** practices

Adopt these patterns and you'll have a robust bash tool implementation that handles edge cases most custom solutions miss.
