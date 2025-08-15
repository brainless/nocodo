# Bootstrap Web App Specification

## Overview

The Bootstrap Web app is a client-side web application that provides a user-friendly interface for managing cloud provider credentials, server lifecycle, and authentication. Built with SolidJS, Tailwind CSS, and Solid UI, it communicates exclusively with the local Bootstrap app running on `localhost`.

## Architecture

### Components

1. **Authentication Module** - Handles nocodo.com login and session management
2. **Provider Management** - Cloud provider credential and configuration interface
3. **Server Dashboard** - Operator server status and control interface
4. **Security Settings** - Encryption and local security management
5. **Status Monitor** - Real-time server and system status display
6. **Navigation Shell** - Application layout and routing

### Technology Stack

- **Framework**: SolidJS with TypeScript
- **Styling**: Tailwind CSS
- **UI Components**: Solid UI (shadcn/ui port for Solid)
- **State Management**: Solid Store
- **HTTP Client**: Solid Query + native fetch
- **Routing**: Solid Router
- **Build Tool**: Vite
- **Type Safety**: Generated types from Bootstrap app via ts-rs

## User Interface Design

### Layout Structure

```
┌─────────────────────────────────────────────────┐
│ Header (Status Bar + User Info)                 │
├─────────────────────────────────────────────────┤
│ Sidebar Navigation │ Main Content Area          │
│ - Dashboard        │                            │
│ - Cloud Providers  │ ┌─────────────────────────┐│
│ - Servers          │ │                         ││
│ - Security         │ │     Dynamic Content     ││
│ - Settings         │ │                         ││
│                    │ └─────────────────────────┘│
├─────────────────────────────────────────────────┤
│ Footer (Connection Status + Version)            │
└─────────────────────────────────────────────────┘
```

### Key Screens

#### 1. Authentication Screen

- Login form for nocodo.com credentials
- "Remember me" option for local session persistence
- Connection status indicator
- Error handling for authentication failures

#### 2. Dashboard

- Quick overview of server status
- Recent activities and logs
- Resource usage summary (if server is running)
- Quick actions (start/stop server, create new server)

#### 3. Cloud Providers Management

- List of supported cloud providers
- Add/remove API credentials interface
- Test connection functionality
- Credential masking for security
- Provider-specific configuration options

#### 4. Server Management

- Server list with status indicators
- Create new server wizard
- Server details and configuration
- Start/stop/destroy server controls
- Server logs viewer
- Image management interface

#### 5. Security Settings

- Local encryption password management
- Session timeout configuration
- Audit log viewer
- Security recommendations

## Component Architecture

### Core Components

```typescript
// Authentication
interface AuthStore {
  user: User | null;
  isAuthenticated: boolean;
  login: (credentials: LoginRequest) => Promise<void>;
  logout: () => void;
  refreshToken: () => Promise<void>;
}

// Cloud Provider Management
interface ProviderStore {
  providers: CloudProvider[];
  credentials: ProviderCredentials[];
  addCredential: (provider: string, creds: any) => Promise<void>;
  testConnection: (provider: string, credId: string) => Promise<boolean>;
}

// Server Management
interface ServerStore {
  servers: OperatorServer[];
  currentServer: OperatorServer | null;
  createServer: (config: ServerConfig) => Promise<void>;
  startServer: (serverId: string) => Promise<void>;
  stopServer: (serverId: string) => Promise<void>;
}
```

### UI Components

```typescript
// Form Components
const CredentialForm = (props: {
  provider: CloudProvider;
  onSubmit: (data: any) => void;
}) => JSX.Element;

const ServerCreationWizard = (props: {
  providers: CloudProvider[];
  onComplete: (config: ServerConfig) => void;
}) => JSX.Element;

// Status Components
const ServerStatusCard = (props: {
  server: OperatorServer;
  onAction: (action: string) => void;
}) => JSX.Element;

const ConnectionStatus = (props: {
  isConnected: boolean;
  lastSeen?: Date;
}) => JSX.Element;

// Data Display Components
const LogViewer = (props: {
  logs: LogEntry[];
  onRefresh: () => void;
}) => JSX.Element;

const ResourceMonitor = (props: {
  server: OperatorServer;
}) => JSX.Element;
```

## API Integration

### HTTP Client Setup

```typescript
class BootstrapApiClient {
  private baseUrl = 'http://localhost:8080';
  
  async request<T>(
    path: string, 
    options: RequestInit = {}
  ): Promise<ApiResponse<T>> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    return response.json();
  }
  
  // Specific API methods
  async login(credentials: LoginRequest): Promise<LoginResponse> {
    return this.request('/auth/login', {
      method: 'POST',
      body: JSON.stringify(credentials),
    });
  }
  
  async getProviders(): Promise<CloudProvider[]> {
    return this.request('/providers');
  }
  
  async getServers(): Promise<OperatorServer[]> {
    return this.request('/operator');
  }
}
```

### Real-time Updates

```typescript
// Server status polling
const useServerStatus = () => {
  const [status, setStatus] = createSignal<ServerStatus>();
  
  createEffect(() => {
    const interval = setInterval(async () => {
      try {
        const response = await api.getServerStatus();
        setStatus(response.data);
      } catch (error) {
        console.error('Failed to fetch server status:', error);
      }
    }, 5000); // Poll every 5 seconds
    
    onCleanup(() => clearInterval(interval));
  });
  
  return status;
};
```

## State Management

### Global State Structure

```typescript
interface AppState {
  auth: {
    user: User | null;
    isAuthenticated: boolean;
    sessionExpires: Date | null;
  };
  providers: {
    available: CloudProvider[];
    credentials: ProviderCredentials[];
    loading: boolean;
  };
  servers: {
    list: OperatorServer[];
    current: OperatorServer | null;
    logs: LogEntry[];
    loading: boolean;
  };
  ui: {
    sidebarOpen: boolean;
    currentPage: string;
    notifications: Notification[];
  };
}
```

### Store Implementation

```typescript
const [appState, setAppState] = createStore<AppState>({
  auth: { user: null, isAuthenticated: false, sessionExpires: null },
  providers: { available: [], credentials: [], loading: false },
  servers: { list: [], current: null, logs: [], loading: false },
  ui: { sidebarOpen: true, currentPage: 'dashboard', notifications: [] },
});

// Store actions
const appActions = {
  auth: {
    login: async (credentials: LoginRequest) => {
      setAppState('auth', 'loading', true);
      try {
        const response = await api.login(credentials);
        setAppState('auth', {
          user: response.user,
          isAuthenticated: true,
          sessionExpires: new Date(response.expires_at),
        });
      } catch (error) {
        // Handle error
      } finally {
        setAppState('auth', 'loading', false);
      }
    },
  },
  // ... other actions
};
```

## Routing

```typescript
const App = () => {
  return (
    <Router>
      <Routes>
        <Route path="/login" component={LoginPage} />
        <Route path="/" element={<AuthGuard />}>
          <Route path="/" component={Dashboard} />
          <Route path="/providers" component={ProvidersPage} />
          <Route path="/servers" component={ServersPage} />
          <Route path="/servers/:id" component={ServerDetailPage} />
          <Route path="/security" component={SecurityPage} />
          <Route path="/settings" component={SettingsPage} />
        </Route>
        <Route path="*" element={<Navigate href="/" />} />
      </Routes>
    </Router>
  );
};
```

## Security Considerations

### Local Storage

- Session tokens stored in secure localStorage
- Automatic token cleanup on logout
- No sensitive credentials stored in browser storage
- CSRF protection for API calls

### Communication Security

- All API calls to localhost only
- No external API calls except to nocodo.com for auth
- Input validation and sanitization
- XSS protection through SolidJS built-in security

### Error Handling

```typescript
const ErrorBoundary = (props: { children: any }) => {
  return (
    <ErrorBoundary
      fallback={(err, retry) => (
        <div class="error-container">
          <h2>Something went wrong</h2>
          <p>{err.message}</p>
          <button onClick={retry}>Try again</button>
        </div>
      )}
    >
      {props.children}
    </ErrorBoundary>
  );
};
```

## User Experience Features

### Loading States

- Skeleton loaders for data fetching
- Progress indicators for long operations (server creation)
- Optimistic updates where appropriate

### Accessibility

- WCAG 2.1 AA compliance
- Keyboard navigation support
- Screen reader compatibility
- High contrast mode support
- Focus management

### Responsive Design

- Mobile-first approach
- Responsive layout for different screen sizes
- Touch-friendly interface elements
- Adaptive sidebar for mobile devices

## Development Workflow

### Project Structure

```
src/
├── components/
│   ├── ui/           # Reusable UI components
│   ├── forms/        # Form components
│   └── layout/       # Layout components
├── pages/            # Page components
├── stores/           # State management
├── api/              # API client and types
├── utils/            # Utility functions
├── styles/           # Global styles
└── types/            # TypeScript type definitions
```

### Build Configuration

```typescript
// vite.config.ts
export default defineConfig({
  plugins: [solid()],
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8080',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ''),
      },
    },
  },
  build: {
    target: 'esnext',
    outDir: 'dist',
  },
});
```

### Development Scripts

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "test": "vitest",
    "lint": "eslint src --ext .ts,.tsx",
    "type-check": "tsc --noEmit"
  }
}
```

## Testing Strategy

### Unit Tests

```typescript
import { render, screen } from 'solid-testing-library';
import { ServerStatusCard } from './ServerStatusCard';

describe('ServerStatusCard', () => {
  it('renders server information correctly', () => {
    const mockServer = {
      id: '1',
      status: 'running',
      name: 'test-server',
    };
    
    render(() => <ServerStatusCard server={mockServer} />);
    
    expect(screen.getByText('test-server')).toBeInTheDocument();
    expect(screen.getByText('Running')).toBeInTheDocument();
  });
});
```

### Integration Tests

- E2E tests for critical user flows
- API integration tests
- Authentication flow testing
- Server management workflow tests

## Performance Optimization

### Bundle Optimization

- Code splitting by routes
- Dynamic imports for heavy components
- Tree shaking for unused code
- Asset optimization and compression

### Runtime Performance

- Memoization of expensive computations
- Efficient state updates
- Lazy loading of non-critical components
- Virtual scrolling for large lists

## Clarification Questions

1. **Offline Functionality**: Should the app work when the Bootstrap app is not running?
2. **Multi-language Support**: Do we need internationalization (i18n) support?
3. **Theme Customization**: Should users be able to customize the UI theme?
4. **Keyboard Shortcuts**: What keyboard shortcuts should be implemented?
5. **Data Export**: Should users be able to export configuration or logs?
6. **Browser Support**: What browsers should we target (modern browsers only)?
7. **PWA Features**: Should this be a Progressive Web App with offline capabilities?
8. **Notification System**: What types of notifications should be shown to users?

## Future Enhancements

- Dark/light theme toggle
- Advanced server configuration UI
- Real-time collaboration features
- Dashboard customization
- Notification system with email/SMS
- Advanced logging and debugging tools
- Integration with external monitoring tools
- Mobile-responsive improvements
- Progressive Web App features
