# Manager Web App Specification

## Overview

The Manager Web app is a single-page application that provides a Lovable-like chat interface for users to build software. It communicates with the Manager daemon via a RESTful API and WebSockets. Built with SolidJS, TypeScript, and Tailwind CSS, it offers a real-time, interactive environment for AI-assisted development.

## Architecture

### Core Components

1. **Chat Interface** - The primary user interface for interacting with the AI
2. **Project Dashboard** - Overview of projects, status, and recent activity
3. **File Explorer** - Browse, view, and edit project files
4. **Code Editor** - Integrated Monaco editor for code viewing and editing
5. **Terminal Emulator** - Xterm.js-based terminal for direct command execution
6. **Real-time Engine** - WebSocket client for real-time updates
7. **State Management** - Solid Store for managing application state
8. **API Client** - Native fetch with Solid Query for communicating with the Manager daemon

### Technology Stack

- **Framework**: SolidJS with Vite
- **Language**: TypeScript
- **Styling**: Tailwind CSS
- **UI Components**: Solid UI components
- **State Management**: Solid Store
- **API Communication**: Solid Query + native fetch (REST), WebSocket (real-time)
- **Code Editor**: Monaco Editor
- **Terminal**: Xterm.js
- **Routing**: Solid Router
- **Build Tool**: Vite
- **Type Safety**: Generated types from Manager app via ts-rs

## User Interface Design

### Main Layout

```
┌─────────────────────────────────────────────────────────┐
│ Header (Project Selector, Notifications, User Menu)     │
├─────────────────┬─────────────────┬────────────────────┤
│   Sidebar       │   Main Content  │   Context Panel    │
│   (Projects,    │   (Chat,        │   (File Explorer,  │
│   File Explorer,│   Dashboard,    │   Terminal,        │
│   Settings)     │   Code Editor)  │   Help)            │
└─────────────────┴─────────────────┴────────────────────┘
```

### Key Screens

#### 1. Chat Interface

- Main interaction area for AI-driven development
- Rich markdown support for code blocks, lists, etc.
- Inline actions (e.g., apply code changes, run tests)
- File drag-and-drop for context sharing
- Command palette for quick actions

#### 2. Project Dashboard

- Project overview cards with status and metadata
- Recent activity feed (commits, deployments, etc.)
- Quick links to common project actions
- Resource usage monitoring (CPU, memory)

#### 3. File Explorer & Code Editor

- Tree-based file explorer
- Monaco-based code editor with syntax highlighting
- In-editor diff view for AI-suggested changes
- File operations (create, rename, delete)
- Real-time collaborative editing (future)

#### 4. Integrated Terminal

- Xterm.js-based terminal emulator
- Direct access to the Operator server shell
- Secure WebSocket communication
- Multiple terminal tabs

## Component Architecture

### Core Components

```typescript
import { createSignal, createEffect, Component } from 'solid-js';
import { createStore } from 'solid-js/store';

// Chat Component with SolidJS
interface ChatProps {
  // Generated types from Manager API via ts-rs
  messages: Message[];
  onSendMessage: (text: string) => void;
}

const Chat: Component<ChatProps> = (props) => {
  const [message, setMessage] = createSignal('');
  const [isTyping, setIsTyping] = createSignal(false);
  
  const handleSendMessage = () => {
    const text = message();
    if (text.trim()) {
      props.onSendMessage(text);
      setMessage('');
    }
  };
  
  return (
    <div class="chat-container">
      {/* Message list and input form */}
    </div>
  );
};

// Project Dashboard Component
interface DashboardProps {
  projects: Project[];
  loading: boolean;
}

const ProjectDashboard: Component<DashboardProps> = (props) => {
  createEffect(() => {
    // Fetch projects on component mount
    // This would integrate with Solid Query
  });
  
  return (
    <div class="dashboard-grid">
      {/* Project cards */}
    </div>
  );
};

// Code Editor Component
interface CodeEditorProps {
  file: ProjectFile;
  onChange: (content: string) => void;
}

const CodeEditor: Component<CodeEditorProps> = (props) => {
  const [content, setContent] = createSignal('');
  let editorRef: HTMLDivElement;
  
  createEffect(() => {
    // Load file content when file changes
    if (props.file) {
      setContent(props.file.content || '');
    }
  });
  
  return (
    <div 
      ref={editorRef} 
      class="monaco-editor-container"
      // Monaco editor will be mounted here
    />
  );
};
```

### State Management (Solid Store)

```typescript
import { createStore } from 'solid-js/store';
import { createContext, useContext, ParentComponent } from 'solid-js';

// Global app state with Solid Store
interface AppState {
  chat: {
    messages: Message[];
    isTyping: boolean;
    session: AiSession | null;
  };
  projects: {
    list: Project[];
    current: Project | null;
    loading: boolean;
  };
  files: {
    tree: FileNode[];
    openFiles: ProjectFile[];
    activeFile: ProjectFile | null;
  };
  terminal: {
    sessions: TerminalSession[];
    activeSession: string | null;
  };
  ui: {
    sidebarOpen: boolean;
    theme: 'light' | 'dark';
    notifications: Notification[];
  };
}

const AppStateContext = createContext<{
  state: AppState;
  setState: (path: any, value: any) => void;
}>();

const AppStateProvider: ParentComponent = (props) => {
  const [state, setState] = createStore<AppState>({
    chat: { messages: [], isTyping: false, session: null },
    projects: { list: [], current: null, loading: false },
    files: { tree: [], openFiles: [], activeFile: null },
    terminal: { sessions: [], activeSession: null },
    ui: { sidebarOpen: true, theme: 'light', notifications: [] },
  });

  const store = {
    state,
    setState,
  };

  return (
    <AppStateContext.Provider value={store}>
      {props.children}
    </AppStateContext.Provider>
  );
};

const useAppState = () => {
  const context = useContext(AppStateContext);
  if (!context) {
    throw new Error('useAppState must be used within AppStateProvider');
  }
  return context;
};
```

## API & WebSocket Integration

### API Client (Native Fetch + Solid Query)

```typescript
// API client using native fetch with TypeScript types from ts-rs
class ApiClient {
  private baseURL = '/api';
  
  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const token = localStorage.getItem('authToken');
    const url = `${this.baseURL}${endpoint}`;
    
    const response = await fetch(url, {
      headers: {
        'Content-Type': 'application/json',
        ...(token && { Authorization: `Bearer ${token}` }),
        ...options.headers,
      },
      ...options,
    });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    return response.json();
  }
  
  // Strongly typed API methods using ts-rs generated types
  async fetchProjects(): Promise<Project[]> {
    return this.request('/projects');
  }
  
  async createProject(data: CreateProjectRequest): Promise<Project> {
    return this.request('/projects', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }
  
  async sendMessageToAi(data: AiQueryRequest): Promise<AiResponse> {
    return this.request('/ai/sessions/query', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }
}

const apiClient = new ApiClient();

// Solid Query integration for reactive data fetching
import { createQuery } from '@tanstack/solid-query';

export const useProjects = () => {
  return createQuery({
    queryKey: ['projects'],
    queryFn: () => apiClient.fetchProjects(),
  });
};
```

### WebSocket Client

```typescript
class RealtimeClient {
  private socket: WebSocket;
  
  constructor(url: string) {
    this.socket = new WebSocket(url);
    
    this.socket.onopen = () => console.log('WebSocket connected');
    this.socket.onclose = () => console.log('WebSocket disconnected');
    this.socket.onerror = (error) => console.error('WebSocket error:', error);
  }
  
  listen(store: Store) {
    this.socket.onmessage = (event) => {
      const message = JSON.parse(event.data);
      
      switch (message.type) {
        case 'PROJECT_UPDATE':
          store.dispatch(updateProject(message.payload));
          break;
        case 'AI_RESPONSE':
          store.dispatch(addMessage(message.payload));
          break;
        // ... other event types
      }
    };
  }
  
  send(message: any) {
    this.socket.send(JSON.stringify(message));
  }
}

// Usage
const realtimeClient = new RealtimeClient('wss://manager.local/ws');
realtimeClient.listen(store);
```

## Routing (Solid Router)

```typescript
import { Router, Route, Routes } from '@solidjs/router';
import { Component, lazy } from 'solid-js';

// Lazy load components for code splitting
const LoginPage = lazy(() => import('./pages/LoginPage'));
const DashboardPage = lazy(() => import('./pages/DashboardPage'));
const ChatPage = lazy(() => import('./pages/ChatPage'));
const FilesPage = lazy(() => import('./pages/FilesPage'));
const TerminalPage = lazy(() => import('./pages/TerminalPage'));
const SettingsPage = lazy(() => import('./pages/SettingsPage'));

const App: Component = () => {
  return (
    <Router>
      <Routes>
        <Route path="/login" component={LoginPage} />
        <Route path="/" component={AuthGuard}>
          <Route path="/dashboard" component={DashboardPage} />
          <Route path="/project/:id" component={ProjectLayout}>
            <Route path="/chat" component={ChatPage} />
            <Route path="/files" component={FilesPage} />
            <Route path="/terminal" component={TerminalPage} />
          </Route>
          <Route path="/settings" component={SettingsPage} />
        </Route>
      </Routes>
    </Router>
  );
};

export default App;
```

## Security Considerations

1. **Authentication**: JWT-based authentication with secure token storage
2. **XSS Protection**: SolidJS built-in XSS protection, careful use of innerHTML
3. **CSRF Protection**: Use of CSRF tokens for API requests
4. **Content Security Policy (CSP)**: Implement a strict CSP to prevent unauthorized scripts
5. **Input Validation**: All user inputs are validated and sanitized
6. **WebSocket Security**: Secure WebSocket connections (WSS), origin checks
7. **Type Safety**: Full TypeScript coverage with ts-rs generated types from Rust backend

## Testing Strategy

### Unit Tests (Solid Testing Library)

```typescript
import { render, screen, fireEvent } from 'solid-testing-library';
import { ChatInput } from './ChatInput';

describe('ChatInput', () => {
  it('sends message on submit', () => {
    const onSendMessage = jest.fn();
    render(() => <ChatInput onSendMessage={onSendMessage} />);
    
    const input = screen.getByRole('textbox');
    const button = screen.getByRole('button');
    
    fireEvent.change(input, { target: { value: 'Hello, world!' } });
    fireEvent.click(button);
    
    expect(onSendMessage).toHaveBeenCalledWith('Hello, world!');
  });
});
```

### E2E Tests (Cypress/Playwright)

```javascript
// cypress/integration/chat.spec.js
describe('Chat functionality', () => {
  it('allows users to send and receive messages', () => {
    cy.visit('/project/1/chat');
    
    cy.get('[data-cy=chat-input]').type('Create a Python script');
    cy.get('[data-cy=send-button]').click();
    
    cy.contains('Create a Python script').should('be.visible');
    cy.contains('Okay, here is a Python script').should('be.visible');
  });
});
```

## Clarification Questions

1. **Real-time Collaboration**: Is real-time collaborative editing a priority?
2. **Offline Support**: Should the app have any offline capabilities?
3. **Theme Customization**: How much theme customization should be supported?
4. **Accessibility Standards**: What level of WCAG compliance is required?
5. **Browser Support**: Which browsers and versions should be targeted?
6. **Mobile Experience**: Is a mobile-responsive design required?
7. **Performance Budgets**: What are the target performance metrics (e.g., LCP, FID)?
8. **Data Persistence**: How should local UI state be persisted?

## Future Enhancements

- Real-time collaborative editing in the code editor
- Advanced data visualization and dashboards
- Voice-to-text input for chat
- Integration with project management tools (Jira, Trello)
- Extensible plugin system for custom components
- Version control integration (Git history, branching)
- AI-powered code completion and suggestions in the editor
- Team management and permissions
- In-app deployment pipeline visualization
