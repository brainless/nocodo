# PTY-based Terminal Streaming Runner Implementation

This document describes the implementation of the PTY-based terminal streaming runner for interactive CLI coding agents as specified in GitHub issue #58.

## Overview

The implementation provides true full-duplex, keystroke-level interactive sessions with CLI coding agents using pseudo-terminals (PTY). This allows support for terminal UIs, ANSI control sequences, and real-time interaction.

## Architecture

### Core Components

1. **TerminalRunner** (`terminal_runner.rs`)
   - Manages PTY sessions for interactive tools
   - Spawns processes with PTY allocation
   - Handles input/output streaming
   - Manages session lifecycle

2. **TerminalSession** (`models.rs`)
   - Database model for terminal sessions
   - Tracks session state, dimensions, and metadata

3. **WebSocket Integration** (`websocket.rs`)
   - Dual-channel WebSocket support (binary for output, text for control)
   - Terminal-specific connection handler
   - Real-time bidirectional communication

4. **Database Support** (`database.rs`)
   - Terminal session persistence
   - Transcript storage with size limits
   - Session metadata tracking

5. **HTTP API** (`handlers.rs`)
   - RESTful endpoints for session management
   - Tool registry access
   - Session control operations

## API Endpoints

### Tool Management
- `GET /api/tools` - List available tools from registry

### Terminal Sessions
- `POST /api/terminals` - Create new terminal session
- `GET /api/terminals/{id}` - Get session info
- `POST /api/terminals/{id}/input` - Send input
- `POST /api/terminals/{id}/resize` - Resize terminal
- `GET /api/terminals/{id}/transcript` - Get session transcript
- `POST /api/terminals/{id}/terminate` - Terminate session

### WebSocket
- `GET /ws/terminals/{id}` - Terminal WebSocket connection
  - Binary frames: terminal output
  - Text frames: control messages (JSON)

## Control Messages

Terminal control messages are JSON objects with the following schema:

```json
{
  "type": "input",
  "data": "base64-encoded-input-bytes"
}
```

```json
{
  "type": "resize", 
  "cols": 80,
  "rows": 24
}
```

```json
{
  "type": "ping"
}
```

## Tool Registry

The system maintains a registry of supported CLI tools with configurations:

```yaml
tools:
  claude:
    command: "claude"
    args: ["--print"]
    requires_pty: true
    working_dir: project
  gemini:
    command: "gemini"
    args: ["--interactive"]  
    requires_pty: true
    working_dir: project
  qwen:
    command: "qwen-code"
    args: ["--interactive"]
    requires_pty: true
    working_dir: project
```

## Security Features

- Tool allowlist prevents arbitrary command execution
- Project-scoped working directories
- Sanitized environment variables
- Transcript size limits (20MB default)
- Session timeouts (10 minutes default)
- Path canonicalization and containment checks

## Configuration

The terminal runner is enabled by default and can be controlled via environment variable:

```bash
NOCODO_TERMINAL_RUNNER_ENABLED=1  # Enable (default)
NOCODO_TERMINAL_RUNNER_ENABLED=0  # Disable
```

## Database Schema

### Terminal Sessions
- `id`, `work_id`, `message_id` - Identifiers
- `tool_name` - Tool being executed
- `status` - running|completed|failed
- `requires_pty`, `interactive` - Capabilities
- `cols`, `rows` - Terminal dimensions
- `started_at`, `ended_at`, `exit_code` - Lifecycle
- `project_context` - Associated project info

### Terminal Transcripts
- Session transcript storage as binary blobs
- Size-limited with automatic truncation
- Indexed by session_id for efficient retrieval

## Dependencies

- `portable-pty` - Cross-platform PTY implementation
- `base64` - Binary data encoding for WebSocket transport
- `tokio` - Async runtime for concurrent I/O
- `actix-web` - WebSocket and HTTP server framework

## Testing

Basic unit tests are included to verify:
- Terminal runner initialization
- Tool registry population  
- Session model creation
- Database integration

## Future Enhancements

- Multi-client session support (viewer vs controller roles)
- Session recording and playback
- Terminal themes and customization
- Resource quotas and limits
- Advanced security sandboxing

## Implementation Status

✅ Core PTY runner implementation
✅ WebSocket integration with binary/text channels
✅ Database persistence layer
✅ HTTP API endpoints
✅ Tool registry system
✅ Basic security measures
✅ Unit tests
⚠️ Web UI integration (future work)
⚠️ Advanced sandboxing (future work)