# nocodo Manager Web App

The Manager Web App provides a chat-based interface for AI-assisted development, running at `localhost:8081`.

## Development

### Prerequisites
- Node.js (LTS version)
- npm

### Setup
```bash
# Install dependencies
npm install

# Start development server
npm run dev
```

### Available Scripts
- `npm run dev` - Start development server
- `npm run build` - Build for production
- `npm run preview` - Preview production build
- `npm run test` - Run unit tests with Vitest
- `npm run test:run` - Run unit tests (CI mode)
- `npm run test:ui` - Run unit tests with UI
- `npm run test:e2e` - Run end-to-end tests with Playwright
- `npm run test:e2e:ui` - Run E2E tests with Playwright UI
- `npm run lint` - Run ESLint
- `npm run lint:fix` - Run ESLint with auto-fix
- `npm run format` - Format code with Prettier
- `npm run format:check` - Check code formatting
- `npm run typecheck` - Run TypeScript type checking

## Testing

### Unit Tests
Unit tests are written with Vitest and @solidjs/testing-library:

```bash
# Run unit tests
npm run test

# Run with UI
npm run test:ui

# Run in CI mode
npm run test:run
```

### End-to-End Tests
E2E tests are written with Playwright and cover the complete user workflow:

```bash
# Install Playwright browsers (first time only)
npx playwright install

# Run E2E tests
npm run test:e2e

# Run with UI for debugging
npm run test:e2e:ui

# Run specific test file
npx playwright test work-creation.test.ts

# Run tests in specific browser
npx playwright test --project=chromium
```

### E2E Test Structure
```
src/__tests__/e2e/
├── setup.ts                 # Test configuration and mocking
├── work-creation.test.ts    # Work creation scenarios
├── agent-integration.test.ts # Agent integration tests
├── file-operations.test.ts  # File listing/reading tests
├── error-handling.test.ts   # Error scenario tests
└── websocket-communication.test.ts # WebSocket tests
```

### Test Scenarios Covered
- ✅ Work creation with various prompts
- ✅ Agent integration with file listing
- ✅ File reading operations
- ✅ Error handling for invalid requests
- ✅ WebSocket real-time communication
- ✅ Form validation and user feedback

### Running Tests Locally
1. Start the Manager daemon: `nocodo-manager --config ~/.config/nocodo/manager.toml`
2. Start the web app: `npm run dev`
3. In another terminal, run E2E tests: `npm run test:e2e`

### CI Integration
E2E tests run automatically in GitHub Actions on:
- Pull requests affecting `manager-web/` directory
- Pushes to main branch affecting `manager-web/` directory

The CI pipeline includes:
- Type checking
- Code formatting
- Linting
- Unit tests
- E2E tests
- Security audit
- Build verification

## Architecture

### Tech Stack
- **Framework**: SolidJS with TypeScript
- **Styling**: TailwindCSS
- **Build Tool**: Vite
- **Testing**: Vitest + Playwright
- **State Management**: SolidJS signals + custom stores
- **Routing**: SolidJS Router

### Key Components
- `App.tsx` - Main application with routing
- `Dashboard.tsx` - Main dashboard with work creation form
- `AiSessionsList.tsx` - Work management interface
- `AiSessionDetail.tsx` - Individual work detail view
- `WebSocketProvider.tsx` - Real-time communication

### API Integration
Communicates with Manager daemon via:
- HTTP REST API for CRUD operations
- WebSocket for real-time updates
- TypeScript types generated from Rust backend

## Contributing

1. Follow the development workflow in `CLAUDE.md`
2. Add tests for new features
3. Run formatters and linters before committing
4. Ensure E2E tests pass locally