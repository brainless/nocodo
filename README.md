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
