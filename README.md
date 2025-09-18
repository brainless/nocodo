# nocodo 🤖

**A platform that takes you from idea to live full-stack MVP (no lock-in)**

🚀 [**nocodo.com →**](https://nocodo.com)

> ⚠️ **Under Active Development** - This product is actively being developed. Please ⭐ star and 👀 watch this repository for updates!

Transform your ideas into production-ready applications using AI coding agents, your own cloud infrastructure, and unlimited development iterations.

![nocodo AI Session Details](./website/src/assets/nocodo_AI_Session_Details_Redesigned_26_August_2025.png)

## ✨ What We're Solving

### 🤖 **Free AI Coding Agents**
Integrated AI-powered development tools with no vendor lock-in!

### ☁️ **Your Cloud Infrastructure**
Your development setup is managed by nocodo on your own cloud infrastructure. You own everything.

### 🔓 **Complete Ownership**
Keep your API keys for coding agents and cloud providers (DigitalOcean, Scaleway, CloudFlare). Zero lock-in!

### 🎯 **Idea to Live App**
Takes your idea (voice notes or written text) to live full-stack app running on your domain.

### 📱 **GitHub Integration**
Uses your GitHub account to setup projects, tickets, automation, and comprehensive testing workflows.

### ♾️ **Unlimited Changes**
Make endless change requests using your own API credits or subscriptions.

## 🏗️ Architecture

The nocodo MVP consists of two core components running locally on your Linux machine:

```
┌─────────────────────────────────────────────────────────┐
│                 Linux Laptop (Local)                    │
├─────────────────────────────────┬────────────────────┤
│          Manager Daemon         │   Manager Web      │
│       (Rust + Actix)           │   (SolidJS) ⚡     │
└─────────────────────────────────┴────────────────────┘
```

### 🎯 **Core Components**

- **🖥️ Manager Daemon**: Local orchestration service managing projects, APIs, and coordination
  - Project management and file system operations
  - AI agent session management
  - GitHub Actions workflow parsing and command execution
  - RESTful API server for web app communication
- **💻 Manager Web App**: Chat-based interface for AI interaction at `localhost:8081`

> ⚠️ **Note**: The CLI component has been removed as part of issue #80. The nocodo CLI is no longer included in this repository.

## 🚀 Quick Start

### 📋 Prerequisites
- 🐧 Linux laptop (tested on CachyOS Linux)
- 🦀 Rust toolchain
- 📦 Node.js and npm
- 🤖 AI tools (if using external integrations)

### 🔧 Installation
```bash
# Build Manager daemon
cargo build --release --bin nocodo-manager
sudo cp target/release/nocodo-manager /usr/local/bin/

# Build Web app
cd manager-web
npm install && npm run build

# Start Manager daemon
nocodo-manager --config ~/.config/nocodo/manager.toml
```

### 💡 Usage
```bash
# 🌐 Access web interface
# Navigate to http://localhost:8081

# Note: The nocodo CLI has been removed as part of issue #80
```

## 🧪 Testing

### API-Only End-to-End Tests

nocodo includes fast, reliable API-only end-to-end tests that focus on LLM agent tool call processing without loading UI components.

#### 🚀 Key Benefits
- **10-20x faster** than browser-based tests
- **More reliable** - no UI timing issues or DOM dependencies
- **Better coverage** - direct API endpoint testing
- **CI/CD friendly** - no headless browser requirements

#### 🏃 Running Tests

```bash
# Run all API E2E tests
cd manager-web
npm run test:api-e2e

# Run tests in watch mode during development
npm run test:api-e2e:watch

# Run tests with coverage reporting
npm run test:api-e2e:coverage

# Run tests for CI (with JSON output)
npm run test:api-e2e:ci
```

#### 📊 Test Coverage

The API-only tests cover:

- **Project Management**: CRUD operations, validation, workflows
- **File Operations**: Create, read, update, delete, listing, search
- **Work Sessions**: LLM agent session management, message handling
- **Tool Call Processing**: File operations, error handling, complex workflows
- **WebSocket Communication**: Real-time updates, connection management
- **State Management**: SolidJS store integration, reactive updates
- **Performance**: Load testing, concurrent operations, memory usage
- **Error Handling**: Edge cases, boundary conditions, recovery scenarios

#### 🏗️ Test Architecture

```
manager-web/src/__tests__/api-e2e/
├── setup/                    # Test infrastructure
│   ├── api-client.ts        # HTTP client for API calls
│   ├── test-server.ts       # Manager daemon lifecycle
│   ├── test-database.ts     # Database setup/cleanup
│   ├── test-data.ts         # Mock data generators
│   └── setup.test.ts        # Framework verification
├── workflows/               # Core workflow tests
│   ├── project-workflow.test.ts
│   ├── file-operations.test.ts
│   ├── work-session.test.ts
│   └── llm-agent.test.ts
├── integration/             # Complex integration tests
│   ├── end-to-end-workflow.test.ts
│   ├── websocket-communication.test.ts
│   ├── complex-workflows.test.ts
│   ├── error-handling.test.ts
│   ├── performance-testing.test.ts
│   └── solid-integration.test.ts
└── utils/                   # Test utilities
    ├── websocket-client.ts
    └── state-manager.ts
```

#### 🔄 CI/CD Integration

Tests run automatically on:
- Push to `main` or `develop` branches
- Pull requests affecting test files
- Scheduled runs for performance regression detection

Coverage reports are uploaded to Codecov, and test results are archived for 30 days.

## 📖 Vibe Coding Playbook

Learn our proven methodology for building MVP web applications using terminal-based coding tools and structured prompting flows. Master the art of being both Product Owner and Project Manager in your AI-assisted development workflow.

**[📚 Read the Complete Playbook →](https://nocodo.com/playbook)**

## 🎓 Vibe Coding Fundamentals

Master the essential fundamentals for AI-powered development:

- **📖 Learn**: Master fundamentals and AI tools through structured modules
- **🧪 Practice**: Apply concepts with hands-on projects and real-world scenarios
- **⚡ Optimize**: Fine-tune your AI-assisted development workflow
- **🤝 Share**: Contribute to the vibe coding community

## 🛣️ Roadmap

### 🎯 **Current MVP Focus**
- ✅ Local Linux laptop deployment
- ✅ Manager daemon with SQLite
- ✅ Web interface at localhost:8081
- ✅ CLI integration with AI tools
- 🔄 Active development and testing

### 🚀 **Future Features**
- ☁️ Cloud deployment automation
- 🌐 Public domain hosting (`*.nocodo.dev`)
- 🔧 Infrastructure as code
- 📊 Advanced monitoring and analytics
- 🔒 Enhanced security features

## 🤝 Contributing

We're preparing for launch with early adopters!

- 🐛 **Found a bug?** Open an issue
- 💡 **Have an idea?** Start a discussion
- 🔧 **Want to contribute?** Check our development workflow
- ⭐ **Support us** by starring this repository

## 📞 Stay Connected

- 🌐 **Website**: [nocodo.com](https://nocodo.com)
- 📖 **Documentation**: [docs.nocodo.com](https://nocodo.com/fundamentals)
- 📋 **Playbook**: [nocodo.com/playbook](https://nocodo.com/playbook)

---

**⚡ Ready to transform your development workflow?** [**Get Started →**](https://nocodo.com)

> 🤖 Built with AI • 🔓 No lock-in • ♾️ Unlimited possibilities
