# nocodo MVP Architecture Overview

> This document synthesizes the key architectural decisions from `mvp_high_level.txt` and provides a clear understanding of the MVP approach.

## Key Architectural Decisions

Based on the voice notes and MVP requirements, the nocodo architecture has been simplified for the MVP to focus on local-only operation:

### MVP Scope (Current Implementation)
- **Local-only operation** on Linux laptop
- **Two core components** working together
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
├─────────────────────────────────┬────────────────────┤
│          Manager Daemon         │   Manager Web      │
│       (Rust + Actix)           │   (SolidJS)        │
├─────────────────────────────────┼────────────────────┤
│                                 │                    │
│   HTTP Server                   │   Dev Server       │
│   localhost:8081               │   localhost:3000   │
│   SQLite DB                     │   API Proxy        │
│   File System                   │   WebSocket        │
│   Unix Socket (future use)      │                    │
│                                 │                    │
└─────────────────────────────────┴────────────────────┘
```

## Separation of Concerns

### Manager Daemon
- **Scope**: Project management and API services
- **Purpose**: Central coordination and data management
- **Responsibilities**:
  - Project CRUD operations and lifecycle management
  - AI session orchestration and tracking
  - File system operations and project structure
  - HTTP API and WebSocket communication
  - SQLite database management

### Manager Web App
- **Scope**: User interface and interaction
- **Purpose**: Chat-based interface for AI development
- **Responsibilities**:
  - AI chat interface and conversation management
  - Project dashboard and file management UI
  - Real-time updates via WebSocket communication
  - User interaction and input handling

## Communication Flow

### Primary Communication Paths
1. **User ↔ Manager Web**: HTTP/WebSocket on localhost:3000 (dev server)
2. **Manager Web ↔ Manager Daemon**: HTTP/WebSocket via API proxy to localhost:8081
3. **AI Tools ↔ Manager Daemon**: Direct HTTP API integration on localhost:8081

### Data Flow Example
```
User Request (Web) → Manager Web → Manager Daemon → Project Context → AI Tool
                                                                   ↓
User Response (Web) ← Manager Web ← Manager Daemon ← AI Response ← AI Tool
```

## Key Architectural Principles

### 1. Boundary Clarity
- **Manager Daemon**: System boundary - manages projects, data, and APIs
- **Manager Web**: User interface boundary - handles interaction and presentation
- **Clear separation prevents feature creep and maintains focus**

### 2. Local-First MVP
- All components run on user's Linux laptop
- No cloud dependencies or authentication required
- Manager daemon provides API on localhost:8081
- Web app runs on localhost:3000 with API proxy

### 3. AI Tool Integration Strategy
- AI tools integrate directly with Manager Daemon via HTTP API
- Manager daemon coordinates AI sessions and project context
- Web app provides user interface for AI interactions
- Separation allows different AI tools to use the same infrastructure

### 4. Future-Ready Design
- Architecture supports cloud deployment (post-MVP)
- Component separation enables distributed deployment
- Local development environment matches production patterns

## MVP Development Strategy

### Phase 1: Core Infrastructure
- Manager daemon with HTTP API server
- SQLite database and configuration management
- Basic project data models

### Phase 2: Project Management  
- Project CRUD operations
- File system operations
- Basic project templates

### Phase 3: AI Integration
- AI session management
- Direct AI tool integration framework
- Context-aware project analysis

### Phase 4: Web Interface
- SolidJS app with chat interface
- Real-time WebSocket communication
- Project dashboard and file browser UI

### Phase 5: Polish & Integration
- End-to-end testing
- Documentation and installation
- Performance optimization

## Success Metrics

The MVP architecture will be considered successful when:

1. ✅ Both components start and communicate successfully
2. ✅ AI tools can successfully integrate with Manager Daemon
3. ✅ Projects can be created, analyzed, and managed
4. ✅ Web interface provides functional chat and project management
5. ✅ Real-time communication works between Web app and Manager
6. ✅ Basic project operations work end-to-end

This architecture provides a solid foundation for AI-assisted development while maintaining clear boundaries and enabling future expansion to cloud-based deployment.

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

1. ✅ Both components start and communicate successfully
2. ✅ AI tools can successfully integrate with Manager Daemon
3. ✅ Projects can be created, analyzed, and managed
4. ✅ Web interface provides functional chat and project management
5. ✅ Real-time communication works between Web app and Manager
6. ✅ Basic project operations work end-to-end

This architecture provides a solid foundation for AI-assisted development while maintaining clear boundaries and enabling future expansion to cloud-based deployment.
