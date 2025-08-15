# nocodo.com - Vibe Coding Learning Platform

## Project Overview

A modern learning website focused on "vibe coding" - a structured approach to software engineering that emphasizes understanding fundamentals before leveraging AI coding agents.

## Tech Stack

- **Frontend Framework**: Astro
- **Styling**: Tailwind CSS
- **Content Management**: Markdown files
- **Markdown Processing**: Built-in Astro markdown support + remark/rehype plugins
- **Routing**: File-based routing (Astro)
- **Code Highlighting**: Shiki (built into Astro)
- **Deployment**: Vercel/Netlify

## Site Structure

### 1. Homepage (`/`)
- Hero section with "Vibe Coding" branding
- Brief introduction to the learning philosophy
- Navigation to main learning paths
- Modern, clean design with coding-themed visuals

### 2. Learning Modules

#### Module 1: Software Engineering Fundamentals (`/fundamentals`)
- **Git & Version Control**
  - Understanding repositories
  - Branch-based workflows
  - Merge strategies and conflict resolution
  - Best practices for commit messages

- **CI/CD Pipelines**
  - Continuous Integration concepts
  - Automated testing in pipelines
  - Deployment strategies
  - Popular CI/CD tools overview

- **Testing Fundamentals**
  - Unit, integration, and e2e testing
  - Test-driven development (TDD)
  - Testing frameworks and tools
  - Code coverage and quality metrics

- **Code Quality & Review**
  - Code review processes
  - Linting and formatting
  - Documentation standards
  - Refactoring techniques

#### Module 2: AI-Assisted Development (`/ai-development`)
- **Understanding AI Coding Agents**
  - Claude.ai capabilities and best practices
  - ChatGPT for development tasks
  - Google Gemini integration
  - Comparing different AI tools

- **From Idea to Documentation**
  - Writing effective product descriptions
  - Generating comprehensive documentation
  - Creating user stories and requirements
  - Documentation tools and formats

- **Documentation to Implementation**
  - Converting docs to GitHub issues/tickets
  - Creating actionable development tasks
  - Project management integration
  - Agile workflow with AI assistance

#### Module 3: AI Coding Agents (`/coding-agents`)
- **Claude Code Integration**
  - Setting up and configuring Claude Code
  - Command-line workflows
  - Best practices for task delegation
  - Troubleshooting common issues

- **Other Coding Agents**
  - GitHub Copilot
  - Cursor AI
  - Codeium
  - Comparative analysis

- **Agent Workflow Optimization**
  - Task breakdown strategies
  - Quality control and review
  - Iterative development with AI
  - Human-AI collaboration patterns

### 3. Resources Section (`/resources`)
- Curated tool lists
- Recommended reading
- Community links
- Templates and boilerplates

### 4. About (`/about`)
- Philosophy behind "vibe coding"
- Learning methodology
- Community guidelines

## File Structure

```
website/
├── public/
│   ├── favicon.svg
│   └── images/
├── src/
│   ├── components/
│   │   ├── Layout/
│   │   │   ├── Header.astro
│   │   │   ├── Footer.astro
│   │   │   └── Sidebar.astro
│   │   ├── UI/
│   │   │   ├── Button.astro
│   │   │   ├── Card.astro
│   │   │   └── CodeBlock.astro
│   │   └── Welcome.astro
│   ├── layouts/
│   │   └── Layout.astro
│   ├── pages/
│   │   ├── index.astro
│   │   ├── fundamentals/
│   │   ├── ai-development/
│   │   ├── coding-agents/
│   │   ├── resources.astro
│   │   └── about.astro
│   ├── content/
│   │   ├── fundamentals/
│   │   │   ├── git-basics.md
│   │   │   ├── cicd-intro.md
│   │   │   ├── testing-fundamentals.md
│   │   │   └── code-quality.md
│   │   ├── ai-development/
│   │   │   ├── ai-agents-overview.md
│   │   │   ├── idea-to-docs.md
│   │   │   └── docs-to-tickets.md
│   │   └── coding-agents/
│   │       ├── claude-code.md
│   │       ├── other-agents.md
│   │       └── workflow-optimization.md
│   ├── styles/
│   │   └── global.css
│   └── assets/
│       ├── astro.svg
│       └── background.svg
├── package.json
├── astro.config.mjs
├── tsconfig.json
└── README.md
```

## Key Features

### Learning Experience
- **Progressive Learning Path**: Structured modules building on each other
- **Interactive Code Examples**: Embedded code snippets with syntax highlighting
- **Practical Exercises**: Hands-on tasks for each module
- **Progress Tracking**: Visual indicators of completion

### Content Management
- **Markdown-First**: All content in easily editable Markdown files
- **Dynamic Routing**: Automatic page generation from content structure
- **Search Functionality**: Find specific topics quickly
- **Tagging System**: Categorize and filter content

### Modern UI/UX
- **Responsive Design**: Mobile-first approach with Tailwind
- **Dark/Light Mode**: Theme switching capability
- **Smooth Animations**: Subtle transitions and micro-interactions
- **Accessibility**: WCAG compliant design

## Development Phases

### Phase 1: Foundation
1. Set up Astro + Tailwind
2. Create basic layout components
3. Implement markdown rendering
4. Build navigation structure

### Phase 2: Content Framework
1. Create placeholder content for all modules
2. Implement dynamic routing
3. Add code syntax highlighting
4. Design responsive layouts

### Phase 3: Enhanced Features
1. Add search functionality
2. Implement progress tracking
3. Create interactive elements
4. Add theme switching

### Phase 4: Content Population
1. Create detailed learning materials
2. Add code examples and exercises
3. Implement feedback systems
4. Community features

## Design Philosophy

The website embodies the "vibe coding" approach:
- **Clean and Minimal**: Focus on content without distractions
- **Developer-Friendly**: Familiar patterns and intuitive navigation
- **Learning-Focused**: Clear progression and achievement markers
- **AI-Forward**: Embraces modern development practices

## Technical Considerations

- **Performance**: Lazy loading for content, optimized images
- **SEO**: Meta tags, structured data, sitemap generation
- **Analytics**: Track learning progress and popular content
- **Deployment**: Automated CI/CD pipeline for content updates
