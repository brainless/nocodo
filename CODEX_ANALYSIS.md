# Codex Project Analysis: Rust Crates and Reusable Modules for Nocodo

## Executive Summary

The Codex project is a highly sophisticated execution and sandboxing framework built in Rust. It provides robust cross-platform process execution with comprehensive sandboxing capabilities on Linux (Landlock + Seccomp), macOS (Seatbelt), and Windows (restricted tokens). The project offers several reusable crates and modules that could significantly benefit the nocodo Bash tool implementation.

---

## 1. EXTERNAL CRATES (External Dependencies)

### Process Execution & Management

#### **1.1 tokio v1.x**
**Purpose**: Asynchronous runtime for Rust
**Location**: `/home/brainless/Projects/codex/codex-rs/Cargo.toml` (line 188)
**Key Features Used**:
- `tokio::process::Command` & `tokio::process::Child` for spawning and managing child processes
- `tokio::spawn()` for concurrent task management
- `tokio::select!` for multiplexing between process termination, timeout, and signal handling
- `tokio::time::timeout()` for timeout enforcement
- `tokio::signal::ctrl_c()` for Ctrl+C handling

**Usage in Codex**:
- Core process spawning: `/home/brainless/Projects/codex/codex-rs/core/src/exec.rs` (lines 458-468)
- Timeout implementation: `/home/brainless/Projects/codex/codex-rs/core/src/exec.rs` (lines 508-527)
- Concurrent I/O handling for stdout/stderr

**Reusability for Nocodo**: ⭐⭐⭐⭐⭐
- **Highly Recommended**: Essential for async process management with timeout support
- Pattern: Use `tokio::select!` to handle process completion, timeouts, and signal handling
- Example pattern from Codex:
```rust
let (exit_status, timed_out) = tokio::select! {
    result = tokio::time::timeout(timeout, child.wait()) => {
        match result {
            Ok(status_result) => (status_result?, false),
            Err(_) => {
                child.start_kill()?;
                (synthetic_exit_status(...), true)
            }
        }
    }
    _ = tokio::signal::ctrl_c() => {
        child.start_kill()?;
        (synthetic_exit_status(...), false)
    }
};
```

#### **1.2 shlex v1.3.0**
**Purpose**: Shell argument parsing/lexing
**Location**: `/home/brainless/Projects/codex/codex-rs/Cargo.toml` (line 175)
**Key Features**:
- Parses shell-quoted command-line strings
- Handles quoted arguments with proper escaping

**Usage in Codex**:
- Referenced in `/home/brainless/Projects/codex/codex-rs/core/src/exec.rs` (line 34)
- Used for command parsing before execution

**Reusability for Nocodo**: ⭐⭐⭐⭐
- **Recommended**: Useful for parsing bash commands passed as strings
- Example use case: Converting user input "echo 'hello world'" into ["echo", "hello world"]

#### **1.3 async-channel v2.3.1**
**Purpose**: Multi-producer, multi-consumer async channels
**Location**: `/home/brainless/Projects/codex/codex-rs/Cargo.toml` (line 103)
**Key Features**:
- Unbounded async channels for inter-task communication
- Used for output aggregation

**Usage in Codex**:
- Output streaming from child processes: `/home/brainless/Projects/codex/codex-rs/core/src/exec.rs` (lines 493-532)
- Collects stdout/stderr in real-time while also emitting events

**Reusability for Nocodo**: ⭐⭐⭐⭐
- **Recommended**: For streaming output to multiple consumers
- Pattern: Aggregate subprocess output while simultaneously emitting delta events

### Sandboxing Crates

#### **1.4 landlock v0.4.1**
**Purpose**: Linux filesystem access control and capability restriction
**Location**: `/home/brainless/Projects/codex/codex-rs/Cargo.toml` (line 135)
**Key Features**:
- Restricts filesystem access to specified paths
- Implements `AccessFs` rules for read/write permissions
- Landlock ABI version management
- No capability drop required

**Usage in Codex**:
- Comprehensive implementation: `/home/brainless/Projects/codex/codex-rs/linux-sandbox/src/landlock.rs`
- Creates rules for read-only filesystem with writable roots:
  - Root filesystem: read-only
  - `/dev/null`: read-write
  - Specified writable paths: read-write
- Installation: Lines 60-82 in landlock.rs

**Reusability for Nocodo**: ⭐⭐⭐⭐
- **Recommended for Linux implementations**
- Can restrict bash tool to only access specific directories
- Example code pattern:
```rust
let access_rw = AccessFs::from_all(abi);
let access_ro = AccessFs::from_read(abi);
let ruleset = Ruleset::default()
    .set_compatibility(CompatLevel::BestEffort)
    .handle_access(access_rw)?
    .create()?
    .add_rules(landlock::path_beneath_rules(&["/"], access_ro))?
    .add_rules(landlock::path_beneath_rules(&["/dev/null"], access_rw))?
    .restrict_self()?;
```

#### **1.5 seccompiler v0.5.0**
**Purpose**: seccomp (secure computing mode) filter generation and application
**Location**: `/home/brainless/Projects/codex/codex-rs/Cargo.toml` (line 167)
**Key Features**:
- Builds BPF (Berkeley Packet Filter) programs for syscall filtering
- Supports conditional rules based on argument inspection
- Architecture-aware code generation (x86_64, aarch64)

**Usage in Codex**:
- Network restriction: `/home/brainless/Projects/codex/codex-rs/linux-sandbox/src/landlock.rs` (lines 87-145)
- Denies outbound network syscalls: `connect`, `bind`, `listen`, `sendto`, `sendmsg`, etc.
- Allows `AF_UNIX` domain sockets but denies other socket families
- Applies filter atomically: `apply_filter(&prog)?`

**Reusability for Nocodo**: ⭐⭐⭐
- **Useful for advanced Linux security**: If restricting network access needed
- More complex to use; may not be necessary if simple filesystem restrictions suffice
- Example pattern:
```rust
let mut rules: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();
// Deny specific syscalls
deny_syscall(libc::SYS_connect);
// Allow AF_UNIX sockets only
let unix_only_rule = SeccompRule::new(vec![
    SeccompCondition::new(0, SeccompCmpArgLen::Dword, SeccompCmpOp::Ne, libc::AF_UNIX as u64)?
])?;
rules.insert(libc::SYS_socket, vec![unix_only_rule]);
```

#### **1.6 portable-pty v0.9.0**
**Purpose**: Cross-platform PTY (pseudo-terminal) support
**Location**: `/home/brainless/Projects/codex/codex-rs/Cargo.toml` (line 156)
**Key Features**:
- Native PTY system abstraction
- Command building and execution in terminal environments
- PTY size configuration
- Child process killing

**Usage in Codex**:
- PTY-based command execution: `/home/brainless/Projects/codex/codex-rs/utils/pty/src/lib.rs`
- Interactive session support with full terminal features
- Real-time output streaming via broadcast channels

**Reusability for Nocodo**: ⭐⭐⭐
- **Useful for interactive scenarios**: If bash tool needs interactive terminal support
- May be overkill for simple non-interactive command execution
- Example pattern:
```rust
let pty_system = native_pty_system();
let pair = pty_system.openpty(PtySize { rows: 24, cols: 80, ... })?;
let mut child = pair.slave.spawn_command(command_builder)?;
```

### System Integration & Permissions

#### **1.7 libc v0.2.175**
**Purpose**: Low-level system calls and C library bindings
**Location**: `/home/brainless/Projects/codex/codex-rs/Cargo.toml` (line 137)
**Key Features**:
- Process hardening calls: `prctl()`, `ptrace()`, `setrlimit()`
- Environment variable management
- Signal handling

**Usage in Codex**:
- Process hardening: `/home/brainless/Projects/codex/codex-rs/process-hardening/src/lib.rs`
  - `PR_SET_DUMPABLE` to prevent ptrace attachment
  - `PT_DENY_ATTACH` on macOS
  - `RLIMIT_CORE` to disable core dumps
- Child process lifecycle: `/home/brainless/Projects/codex/codex-rs/core/src/spawn.rs` (lines 70-75)
  - `PR_SET_PDEATHSIG` to kill children when parent dies

**Reusability for Nocodo**: ⭐⭐⭐⭐
- **Recommended for security-hardened execution**
- Prevents debuggers from attaching
- Ensures child processes are cleaned up on parent death

#### **1.8 windows-sys v0.52 (with extensive features)**
**Purpose**: Windows system API bindings
**Location**: `/home/brainless/Projects/codex/codex-rs/windows-sandbox-rs/Cargo.toml` (lines 20-42)
**Key Features**:
- Job Objects for process group management
- Security & ACL manipulation
- Restricted token creation
- Pipes and I/O management

**Usage in Codex**:
- Windows sandbox implementation: `/home/brainless/Projects/codex/codex-rs/windows-sandbox-rs/src/lib.rs`
- Process spawning with restricted privileges
- Output capture via pipes

**Reusability for Nocodo**: ⭐⭐⭐
- **Required for Windows support**: If cross-platform support needed
- Handles complex Windows security model

---

## 2. INTERNAL CRATES (Custom Codex Crates)

### Critical Execution & Sandbox Crates

#### **2.1 codex-exec** ⭐⭐⭐⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/exec/`
**Purpose**: Primary CLI interface and event-driven execution orchestration
**Key Components**:
- `lib.rs` (427 lines): Main execution loop and event handling
- `exec_events.rs` (246 lines): Event type definitions
- `event_processor.rs` (45 lines): Event processing trait
- `event_processor_with_human_output.rs` (599 lines): Human-readable output formatting
- `event_processor_with_jsonl_output.rs` (501 lines): Machine-readable JSONL output

**Key Functionality**:
- Process execution with timeout enforcement
- Event streaming (real-time output deltas)
- Sandbox mode selection (None, Linux Seccomp, macOS Seatbelt, Windows Restricted Token)
- Approval workflows
- Login management
- Model configuration

**Code References**:
- Timeout handling: `/home/brainless/Projects/codex/codex-rs/exec/src/lib.rs` (implicit in event loop)
- Output streaming: Event processors handle real-time output emission
- Sandbox integration: Works with core module for sandboxed execution

**Reusability for Nocodo**: ⭐⭐⭐⭐⭐
- **Highly Recommended for Architecture**: Can be heavily adapted for nocodo's Bash tool
- Pattern: Event-driven architecture separates concerns
- JSON output format could be reused for structured command results
- **Copy these patterns**:
  1. Event streaming architecture for real-time output
  2. Output event emission during execution (before process completion)
  3. Timeout integration with execution
  4. Sandbox mode selection logic

#### **2.2 codex-core** ⭐⭐⭐⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/core/`
**Purpose**: Central execution engine and sandbox management
**Key Modules**:
- `exec.rs` (693 lines): Process execution, timeout handling, output streaming
- `bash.rs` (290+ lines): Shell command parsing using tree-sitter
- `spawn.rs` (108 lines): Child process spawning with sandbox awareness
- `sandboxing/mod.rs`: Sandbox manager and transformation logic
- `process-hardening/lib.rs`: Security hardening before execution

**Critical Functions**:
1. **`execute_exec_env()`** - Main execution entry point with timing
2. **`consume_truncated_output()`** - Streams stdout/stderr with timeout enforcement
3. **`read_capped()`** - Async output reading with delta event emission
4. **`is_likely_sandbox_denied()`** - Detects sandbox failures in output
5. **`spawn_child_async()`** - Cross-platform process spawning
6. **`parse_shell_lc_plain_commands()`** - Safe bash command parsing

**Code Patterns**:

Process Execution with Timeout (lines 432-468 in exec.rs):
```rust
async fn exec(
    params: ExecParams,
    sandbox: SandboxType,
    sandbox_policy: &SandboxPolicy,
    stdout_stream: Option<StdoutStream>,
) -> Result<RawExecToolCallOutput> {
    let timeout = params.timeout_duration();
    let child = spawn_child_async(
        PathBuf::from(program),
        args.into(),
        arg0_ref,
        cwd,
        sandbox_policy,
        StdioPolicy::RedirectForShellTool,
        env,
    ).await?;
    consume_truncated_output(child, timeout, stdout_stream).await
}
```

Timeout Enforcement with Signal Handling (lines 508-527 in exec.rs):
```rust
let (exit_status, timed_out) = tokio::select! {
    result = tokio::time::timeout(timeout, child.wait()) => {
        match result {
            Ok(status_result) => (status_result?, false),
            Err(_) => {
                child.start_kill()?;
                (synthetic_exit_status(EXIT_CODE_SIGNAL_BASE + TIMEOUT_CODE), true)
            }
        }
    }
    _ = tokio::signal::ctrl_c() => {
        child.start_kill()?;
        (synthetic_exit_status(EXIT_CODE_SIGNAL_BASE + SIGKILL_CODE), false)
    }
};
```

Output Streaming with Channels (lines 493-549 in exec.rs):
```rust
let (agg_tx, agg_rx) = async_channel::unbounded::<Vec<u8>>();
let stdout_handle = tokio::spawn(read_capped(
    BufReader::new(stdout_reader),
    stdout_stream.clone(),
    false,
    Some(agg_tx.clone()),
));
// ... wait for handles ...
drop(agg_tx);
let mut combined_buf = Vec::with_capacity(...);
while let Ok(chunk) = agg_rx.recv().await {
    append_all(&mut combined_buf, &chunk);
}
```

Real-time Delta Event Emission (lines 552-604 in exec.rs):
```rust
async fn read_capped<R: AsyncRead + Unpin + Send + 'static>(
    mut reader: R,
    stream: Option<StdoutStream>,
    is_stderr: bool,
    aggregate_tx: Option<Sender<Vec<u8>>>,
) -> io::Result<StreamOutput<Vec<u8>>> {
    loop {
        let n = reader.read(&mut tmp).await?;
        if n == 0 { break; }
        
        if let Some(stream) = &stream 
            && emitted_deltas < MAX_EXEC_OUTPUT_DELTAS_PER_CALL {
            let chunk = tmp[..n].to_vec();
            let msg = EventMsg::ExecCommandOutputDelta(ExecCommandOutputDeltaEvent {
                call_id: stream.call_id.clone(),
                stream: if is_stderr { ExecOutputStream::Stderr } else { ExecOutputStream::Stdout },
                chunk,
            });
            let _ = stream.tx_event.send(event).await;
            emitted_deltas += 1;
        }
        
        if let Some(tx) = &aggregate_tx {
            let _ = tx.send(tmp[..n].to_vec()).await;
        }
    }
}
```

**Bash Parsing** (bash.rs):
- Uses `tree-sitter` and `tree-sitter-bash` for safe command parsing
- `parse_shell_lc_plain_commands()` - Extracts commands from `bash -lc "..."`
- Validates only simple commands with safe operators (`&&`, `||`, `;`, `|`)
- Rejects parentheses, redirections, substitutions, control flow

**Reusability for Nocodo**: ⭐⭐⭐⭐⭐
- **Core asset for bash tool reuse**
- Can be adapted with minimal changes
- Direct patterns for:
  - Timeout handling with signal integration
  - Output streaming with real-time delta events
  - Sandbox integration
  - Bash command validation

#### **2.3 codex-linux-sandbox** ⭐⭐⭐⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/linux-sandbox/`
**Purpose**: Linux-specific sandboxing via Landlock + Seccomp
**Key Components**:
- `landlock.rs` (146 lines): Filesystem restrictions and network filtering
- `linux_run_main.rs` (57 lines): Entry point that applies sandbox policy
- `main.rs`: Binary entry point

**Key Functions**:
1. **`apply_sandbox_policy_to_current_thread()`** - Main sandbox entry point
2. **`install_filesystem_landlock_rules_on_current_thread()`** - Filesystem ACLs
3. **`install_network_seccomp_filter_on_current_thread()`** - Network restrictions

**Code Reference** (landlock.rs):
```rust
pub(crate) fn apply_sandbox_policy_to_current_thread(
    sandbox_policy: &SandboxPolicy,
    cwd: &Path,
) -> Result<()> {
    if !sandbox_policy.has_full_network_access() {
        install_network_seccomp_filter_on_current_thread()?;
    }
    if !sandbox_policy.has_full_disk_write_access() {
        let writable_roots = sandbox_policy
            .get_writable_roots_with_cwd(cwd)
            .into_iter()
            .map(|writable_root| writable_root.root)
            .collect();
        install_filesystem_landlock_rules_on_current_thread(writable_roots)?;
    }
    Ok(())
}
```

Filesystem Rules (lines 59-82):
```rust
fn install_filesystem_landlock_rules_on_current_thread(writable_roots: Vec<PathBuf>) -> Result<()> {
    let abi = ABI::V5;
    let access_rw = AccessFs::from_all(abi);
    let access_ro = AccessFs::from_read(abi);
    
    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::BestEffort)
        .handle_access(access_rw)?
        .create()?
        .add_rules(landlock::path_beneath_rules(&["/"], access_ro))?
        .add_rules(landlock::path_beneath_rules(&["/dev/null"], access_rw))?
        .set_no_new_privs(true);
    
    if !writable_roots.is_empty() {
        ruleset = ruleset.add_rules(landlock::path_beneath_rules(&writable_roots, access_rw))?;
    }
    
    let status = ruleset.restrict_self()?;
    if status.ruleset == landlock::RulesetStatus::NotEnforced {
        return Err(CodexErr::Sandbox(SandboxErr::LandlockRestrict));
    }
    Ok(())
}
```

Network Seccomp Filter (lines 87-145):
- Denies: `connect`, `bind`, `listen`, `accept`, `sendto`, `sendmsg`, etc.
- Allows: `AF_UNIX` sockets only for inter-process communication
- Uses BPF for efficient syscall filtering

**Reusability for Nocodo**: ⭐⭐⭐⭐⭐
- **Excellent for Linux bash tool sandboxing**
- Can be directly integrated or wrapped
- Use approach: Either call as separate binary or embed module
- Filesystem restrictions are immediately applicable
- Network restrictions could be made optional

#### **2.4 codex-process-hardening** ⭐⭐⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/process-hardening/`
**Purpose**: Security hardening before process execution
**Key Functions** (lib.rs, 115 lines):
- `pre_main_hardening()` - Called via `#[ctor::ctor]` before main
- `pre_main_hardening_linux()` - Linux-specific hardening:
  - `prctl(PR_SET_DUMPABLE, 0)` - Disable ptrace attachment
  - `setrlimit(RLIMIT_CORE, 0)` - Disable core dumps
  - Remove `LD_*` environment variables
- `pre_main_hardening_macos()` - macOS-specific:
  - `ptrace(PT_DENY_ATTACH)` - Prevent debugger attachment
  - Remove `DYLD_*` environment variables
- `pre_main_hardening_windows()` - Windows stub (TODO)

**Code Reference**:
```rust
pub fn pre_main_hardening() {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    pre_main_hardening_linux();
    
    #[cfg(target_os = "macos")]
    pre_main_hardening_macos();
    
    #[cfg(windows)]
    pre_main_hardening_windows();
}

#[cfg(any(target_os = "linux", target_os = "android"))]
pub(crate) fn pre_main_hardening_linux() {
    // Disable ptrace attach / mark process non-dumpable.
    let ret_code = unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
    if ret_code != 0 {
        eprintln!("ERROR: prctl(PR_SET_DUMPABLE, 0) failed");
        std::process::exit(PRCTL_FAILED_EXIT_CODE);
    }
    set_core_file_size_limit_to_zero();
    // Remove LD_* environment variables
    let ld_keys: Vec<String> = std::env::vars()
        .filter_map(|(key, _)| if key.starts_with("LD_") { Some(key) } else { None })
        .collect();
    for key in ld_keys {
        unsafe { std::env::remove_var(key); }
    }
}
```

**Reusability for Nocodo**: ⭐⭐⭐⭐
- **Recommended for hardened execution**
- Prevents debuggers and core dumps
- Simple to integrate via `#[ctor]` attribute

#### **2.5 codex-async-utils** ⭐⭐⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/async-utils/`
**Purpose**: Async utilities for cancellation
**Key Components**:
- `OrCancelExt` trait for futures
- Integration with `tokio::util::CancellationToken`

**Code** (lib.rs, 87 lines):
```rust
#[async_trait]
pub trait OrCancelExt: Sized {
    type Output;
    async fn or_cancel(self, token: &CancellationToken) -> Result<Self::Output, CancelErr>;
}

#[async_trait]
impl<F> OrCancelExt for F where F: Future + Send, F::Output: Send {
    type Output = F::Output;
    
    async fn or_cancel(self, token: &CancellationToken) -> Result<Self::Output, CancelErr> {
        tokio::select! {
            _ = token.cancelled() => Err(CancelErr::Cancelled),
            res = self => Ok(res),
        }
    }
}
```

**Reusability for Nocodo**: ⭐⭐⭐
- **Useful utility trait** for cancellable operations
- Could be used for timeout + cancellation of bash operations
- Simplifies `tokio::select!` patterns

#### **2.6 codex-utils-pty** ⭐⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/utils/pty/`
**Purpose**: Interactive PTY-based command execution
**Key Structure**:
- `ExecCommandSession` - Wraps running process
- `SpawnedPty` - PTY session with output/exit receivers
- `spawn_pty_process()` - Async PTY spawning

**Code Reference** (lib.rs, 211 lines):
```rust
pub async fn spawn_pty_process(
    program: &str,
    args: &[String],
    cwd: &Path,
    env: &HashMap<String, String>,
) -> Result<SpawnedPty> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize { rows: 24, cols: 80, ... })?;
    let mut child = pair.slave.spawn_command(command_builder)?;
    // ... setup reader/writer/wait tasks ...
}

pub struct ExecCommandSession {
    writer_tx: mpsc::Sender<Vec<u8>>,
    output_tx: broadcast::Sender<Vec<u8>>,
    killer: StdMutex<Option<Box<dyn portable_pty::ChildKiller + Send + Sync>>>,
    reader_handle: StdMutex<Option<JoinHandle<()>>>,
    // ...
}
```

**Reusability for Nocodo**: ⭐⭐⭐
- **Useful for interactive bash scenarios**
- May be overkill for non-interactive command execution
- Good reference for async I/O patterns

#### **2.7 codex-windows-sandbox** ⭐⭐⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/windows-sandbox-rs/`
**Purpose**: Windows process sandboxing via restricted tokens and job objects
**Key Functions**:
- `run_windows_sandbox_capture()` - Main sandboxing function
- `preflight_audit_everyone_writable()` - Security audit
- Windows-specific ACL and capability management

**Implementation Features**:
- Restricted token creation
- Job object process group management
- Pipe-based I/O capture
- Timeout support via `WaitForSingleObject()`
- Proper environment variable setup for sandboxing

**Reusability for Nocodo**: ⭐⭐⭐⭐
- **Necessary for Windows support**
- Can be used as-is or wrapped
- Handles complex Windows security model

---

## 3. UTILITY CRATES (Supporting Infrastructure)

#### **3.1 codex-protocol** ⭐⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/protocol/`
**Purpose**: Protocol definitions (Event, SandboxPolicy, etc.)
**Key Types**:
- `Event` - Event wrapper with ID
- `EventMsg` - Event message variants
- `ExecCommandOutputDeltaEvent` - Real-time output delta
- `SandboxPolicy` - Sandbox configuration
- `ExecOutputStream` - Stdout/Stderr marker

**Reusability for Nocodo**: ⭐⭐⭐⭐
- **Consider adopting protocol types** for nocodo's event system
- Can be modified for bash-specific events
- Enables type-safe event handling

#### **3.2 codex-apply-patch** ⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/apply-patch/`
**Purpose**: Apply unified diffs to source files
**Reusability for Nocodo**: ⭐⭐
- Not directly relevant to bash tool execution
- Could be useful if nocodo has file modification features

#### **3.3 codex-file-search** ⭐⭐
**Location**: `/home/brainless/Projects/codex/codex-rs/file-search/`
**Purpose**: Fast file searching (likely with ripgrep)
**Reusability for Nocodo**: ⭐⭐
- Could complement bash tool for search operations
- Not core to process execution

---

## 4. KEY PATTERNS & ARCHITECTURES

### Pattern 1: Event-Driven Execution

**Source**: `exec.rs`, `event_processor.rs`

The Codex execution model uses an event stream to communicate process state:
```
Process Spawned
    ↓
Process Running (emit OutputDelta events every N bytes)
    ↓
Process Completed (emit ExitEvent)
    ↓
Response Generation
```

**Advantage**: Enables real-time output streaming without blocking on process completion.

**Nocodo Application**: Stream bash output as it's generated, not after command completes.

### Pattern 2: Timeout Enforcement via tokio::select!

**Source**: `exec.rs` lines 508-527

Multiplexes between three events:
1. Process completion
2. Timeout expiration
3. User signal (Ctrl+C)

```rust
tokio::select! {
    result = tokio::time::timeout(timeout, child.wait()) => { ... }
    _ = tokio::signal::ctrl_c() => { ... }
}
```

**Nocodo Application**: Ensure bash tool respects timeout and can be interrupted.

### Pattern 3: Concurrent Output Streaming

**Source**: `exec.rs` lines 493-549

Uses three channels to handle output:
1. **Stdout channel** → real-time delta events (capped)
2. **Stderr channel** → real-time delta events (capped)
3. **Aggregate channel** → collect full output

Spawns separate tasks for reading each stream.

**Nocodo Application**: Emit OutputDelta events for real-time feedback while collecting full output.

### Pattern 4: Sandbox Policy Transformation

**Source**: `sandboxing/mod.rs`

Converts generic `CommandSpec` into platform-specific sandboxed `ExecEnv`:
- Linux: Wraps command with `codex-linux-sandbox` binary
- macOS: Wraps with Seatbelt executable
- Windows: In-process sandbox with restricted token

Unified execution interface regardless of platform.

**Nocodo Application**: Similar abstraction layer for bash sandbox modes.

### Pattern 5: Process Spawning with Parent Death Signal

**Source**: `spawn.rs` lines 68-86

On Linux, sets up `PR_SET_PDEATHSIG` so child dies if parent dies:
```rust
unsafe {
    cmd.pre_exec(|| {
        if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
            return Err(std::io::Error::last_os_error());
        }
        if libc::getppid() == 1 {
            libc::raise(libc::SIGTERM);
        }
        Ok(())
    });
}
```

**Nocodo Application**: Ensure child bash processes are cleaned up on parent death.

---

## 5. SUMMARY TABLE: CRATE REUSABILITY

| Crate | Category | Reusability | Key Use | Notes |
|-------|----------|-------------|---------|-------|
| **tokio** | Runtime | ⭐⭐⭐⭐⭐ | Async process mgmt, timeout | Essential |
| **shlex** | Parsing | ⭐⭐⭐⭐ | Shell arg parsing | Useful but optional |
| **async-channel** | Async | ⭐⭐⭐⭐ | Output streaming | Core pattern |
| **landlock** | Sandbox (Linux) | ⭐⭐⭐⭐ | Filesystem restrictions | Linux only |
| **seccompiler** | Sandbox (Linux) | ⭐⭐⭐ | Network restrictions | Advanced |
| **portable-pty** | I/O | ⭐⭐⭐ | Interactive sessions | Optional |
| **libc** | System | ⭐⭐⭐⭐ | Process hardening | Recommended |
| **windows-sys** | Windows | ⭐⭐⭐⭐ | Windows sandbox | Windows only |
| **tree-sitter** | Parsing | ⭐⭐⭐ | Bash parsing | Useful |
| **codex-core** | Internal | ⭐⭐⭐⭐⭐ | Execution engine | Highly adaptable |
| **codex-exec** | Internal | ⭐⭐⭐⭐⭐ | Event loop | Architecture model |
| **codex-linux-sandbox** | Internal | ⭐⭐⭐⭐⭐ | Linux sandboxing | Direct reuse |
| **codex-process-hardening** | Internal | ⭐⭐⭐⭐ | Security | Recommended |
| **codex-async-utils** | Internal | ⭐⭐⭐ | Cancellation | Nice-to-have |
| **codex-windows-sandbox** | Internal | ⭐⭐⭐⭐ | Windows sandbox | Windows only |

---

## 6. RECOMMENDATIONS FOR NOCODO

### Phase 1: Core Process Execution (Minimum Viable)
1. **Adopt**: `tokio` (async runtime)
2. **Adopt**: Timeout pattern from `codex-core/exec.rs`
3. **Adopt**: Output streaming pattern from `codex-exec`
4. **Adopt**: `codex-core` bash.rs for command parsing validation
5. **Consider**: `shlex` for arg parsing

### Phase 2: Output Streaming (Enhanced)
1. **Adopt**: Event-driven architecture from `codex-exec`
2. **Adopt**: Concurrent output handling from `codex-core/exec.rs`
3. **Adopt**: `async-channel` for delta events
4. **Implement**: Output delta event emission (real-time feedback)

### Phase 3: Linux Sandboxing
1. **Direct Integration**: `codex-linux-sandbox` crate/module
2. **Adopt**: Landlock + Seccomp patterns
3. **Optional**: Network restriction via Seccomp (if needed)

### Phase 4: Cross-Platform Support
1. **Windows**: Wrap or integrate `codex-windows-sandbox`
2. **macOS**: Implement Seatbelt wrapper (reference Codex patterns)
3. **Abstraction**: Sandbox manager pattern from `codex-core/sandboxing/`

### Phase 5: Security Hardening
1. **Adopt**: `codex-process-hardening` module
2. **Recommended**: `PR_SET_PDEATHSIG` from `spawn.rs`
3. **Optional**: `libc` process hardening calls

---

## 7. FILE REFERENCES FOR QUICK LOOKUP

| Functionality | File | Line Range |
|---------------|------|-----------|
| Timeout handling | `core/src/exec.rs` | 508-527 |
| Output streaming | `core/src/exec.rs` | 493-604 |
| Output delta events | `core/src/exec.rs` | 552-604 |
| Process spawning | `core/src/spawn.rs` | 38-107 |
| Parent death signal | `core/src/spawn.rs` | 68-86 |
| Bash parsing (safe) | `core/src/bash.rs` | 24-89 |
| Sandbox selection | `core/src/sandboxing/mod.rs` | 69-86 |
| Linux sandbox | `linux-sandbox/src/landlock.rs` | 30-82 |
| Network seccomp | `linux-sandbox/src/landlock.rs` | 87-145 |
| Process hardening | `process-hardening/src/lib.rs` | 27-92 |
| Windows sandbox | `windows-sandbox-rs/src/lib.rs` | 178-250+ |
| PTY spawning | `utils/pty/src/lib.rs` | 109-210 |

---

## CONCLUSION

The Codex project provides excellent reference material and reusable code for building a robust bash tool in nocodo. The most valuable assets are:

1. **Architectural patterns** (event-driven, timeout handling, output streaming)
2. **Reusable modules** (exec, core, linux-sandbox, process-hardening)
3. **Security implementations** (Landlock, Seccomp, platform-specific hardening)
4. **Cross-platform abstractions** (SandboxManager pattern)

The `codex-core` module is the crown jewel—its execution engine handles timeouts, output streaming, sandbox integration, and error handling in a clean, testable way. With adaptation for bash-specific concerns, it could significantly reduce development time for nocodo's Bash tool.
