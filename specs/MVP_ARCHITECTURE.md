# nocodo MVP Architecture Overview

> This document synthesizes the key architectural decisions from `mvp_high_level.txt` and provides a clear understanding of the MVP approach.

## Key Architectural Decisions

Based on the voice notes and MVP requirements, the nocodo architecture has been simplified for the MVP to focus on local-only operation:

### MVP Scope (Current Implementation)
- **Local-only operation** on Linux laptop
- **Three core components** working together
- **No cloud dependencies** for MVP
- **Focus on repository-level AI assistance**

### Post-MVP Scope (Future)
- Cloud server provisioning (Bootstrap apps)
- Multi-tenant deployment
- Public URLs and domain management
- Advanced CI/CD integrations

## Component Architecture

```
┌─────────────────────────────────────────────────────────┐
│                 Linux Laptop (Local)                    │
├─────────────────┬─────────────────┬────────────────────┤
│   nocodo CLI    │  Manager Daemon │   Manager Web      │
│   (Rust)        │  (Rust + Actix) │   (SolidJS)        │
├─────────────────┼─────────────────┼────────────────────┤
│                 │                 │                    │
│   AI Tools      │   Unix Socket   │   HTTP Server      │
│   Claude Code   │   Server        │   localhost:8081   │
│   Gemini CLI    │   SQLite DB     │   Static Files     │
│   etc.          │   File System   │   WebSocket        │
│                 │                 │                    │
└─────────────────┴─────────────────┴────────────────────┘
```

## Separation of Concerns

### nocodo CLI
- **Scope**: Single repository/project focus
- **Purpose**: AI tool companion within a project
- **Responsibilities**:
  - Project analysis and understanding
  - Context-aware prompt generation
  - Code quality guardrails
  - Direct AI tool integration

### Manager Daemon
- **Scope**: Higher-level orchestration and project management
- **Purpose**: Coordinate between components and manage multiple projects
- **Responsibilities**:
  - Project switching and multi-project management
  - External integrations (GitHub API, CI/CD monitoring)
  - Communication hub between CLI and Web app
  - System-level concerns and configuration

### Manager Web App
- **Scope**: User interface and interaction
- **Purpose**: Lovable-like chat interface for AI development
- **Responsibilities**:
  - AI chat interface
  - Project dashboard and file management
  - Real-time updates and collaboration
  - Visual project overview

## Communication Flow

### Primary Communication Paths
1. **User ↔ Manager Web**: HTTP/WebSocket on localhost:8081
2. **nocodo CLI ↔ Manager Daemon**: Unix socket at `/tmp/nocodo-manager.sock`
3. **AI Tools → nocodo CLI**: Command execution and context requests
4. **Manager Web ↔ Manager Daemon**: Internal API calls and WebSocket updates

### Data Flow Example
```
User Request (Web) → Manager Daemon → Project Context → nocodo CLI → AI Tool
                                                                        ↓
User Response (Web) ← Manager Daemon ← Validation & Guardrails ← nocodo CLI
```

## Key Architectural Principles

### 1. Boundary Clarity
- **nocodo CLI**: Repository boundary - works within a single project
- **Manager**: System boundary - manages multiple projects and external concerns
- **Clear separation prevents feature creep and maintains focus**

### 2. Local-First MVP
- All components run on user's Linux laptop
- No cloud dependencies or authentication required
- Replicates typical server environment locally
- Enables rapid development and testing

### 3. AI Tool Integration Strategy
- AI tools call nocodo CLI for context and guidance
- nocodo CLI provides structured prompts and guardrails
- Manager coordinates AI sessions across projects
- Separation allows different AI tools to use the same infrastructure

### 4. Future-Ready Design
- Architecture supports cloud deployment (post-MVP)
- Component separation enables distributed deployment
- Local development environment matches production patterns

## MVP Development Strategy

### Phase 1: Core Infrastructure
- Manager daemon with basic HTTP server
- SQLite database and configuration
- Unix socket communication

### Phase 2: Project Management  
- Project CRUD operations
- File system operations
- Basic project templates

### Phase 3: AI Integration
- nocodo CLI basic structure
- AI tool integration framework
- Context-aware analysis

### Phase 4: Web Interface
- SolidJS app with chat interface
- Real-time WebSocket communication
- Project dashboard and file browser

### Phase 5: Polish & Integration
- End-to-end testing
- Documentation and installation
- Performance optimization

## Technical Constraints

### MVP Limitations
- Local-only operation (no cloud deployment)
- Single-user environment
- Basic file operations (no advanced editing)
- Limited AI tool support initially

### Post-MVP Expansion
- Multi-tenant cloud deployment
- Advanced project templates
- Full CI/CD integration
- Public URL management
- Collaborative features

## Success Metrics

The MVP architecture will be considered successful when:

1. ✅ All three components start and communicate successfully
2. ✅ AI tools can successfully use nocodo CLI for project context
3. ✅ Projects can be created, analyzed, and managed
4. ✅ Web interface provides functional chat and project management
5. ✅ Real-time communication works between all components
6. ✅ Basic guardrails prevent common development mistakes

This architecture provides a solid foundation for AI-assisted development while maintaining clear boundaries and enabling future expansion to cloud-based deployment.
