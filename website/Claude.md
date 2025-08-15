# Website - Vibe Coding Learning Platform

## Development

**Framework**: Astro v5.12.3
**Package Manager**: pnpm

```bash
# Development server
pnpm dev

# Build for production
pnpm build

# Preview production build
pnpm preview
```

## Project Structure

- `src/pages/` - Astro pages and routes
- `src/components/` - Reusable Astro components
- `src/layouts/` - Page layout templates
- `src/content/` - Markdown content files (auto-detected by Astro)
- `src/styles/` - Global CSS and Tailwind imports
- `src/assets/` - Static assets
- `public/` - Public static files

## Content Structure

**Learning Modules:**
- `src/content/fundamentals/` - Software engineering fundamentals
- `src/content/ai-development/` - AI-assisted development workflows  
- `src/content/coding-agents/` - AI coding agents and optimization

**Page Routes:**
- Module index pages: `/fundamentals`, `/ai-development`, `/coding-agents`
- Dynamic content pages: `/fundamentals/[slug]`, `/ai-development/[slug]`, `/coding-agents/[slug]`
- Static pages: `/resources`, `/about`

## Styling

**CSS Framework**: Tailwind CSS v4.1.11
- Global styles: `src/styles/global.css`
- Tailwind configured via `@tailwindcss/vite` plugin
- Imported in main layout: `src/layouts/Layout.astro`

## Key Features

- Learning modules for software engineering fundamentals
- AI-assisted development content
- Coding agents documentation
- Markdown-based content management
- Modern responsive design

## CI/CD

**GitHub Actions Workflow**: `.github/workflows/website-ci.yml`
- Triggers on PR and push to main when `website/` files change
- Runs build verification and type checking
- Uses pnpm with dependency caching for faster builds
- Uploads build artifacts for review
- Additional quality checks (vulnerability scanning, package validation)

**Requirements for CI to pass:**
- `pnpm install --frozen-lockfile` must succeed
- `pnpm astro check` (TypeScript validation) must pass
- `pnpm build` must complete without errors
- No high-severity package vulnerabilities

## Content Areas

1. **Fundamentals** - Git, CI/CD, Testing, Code Quality
2. **AI Development** - AI agents, documentation workflows
3. **Coding Agents** - Claude Code, workflow optimization
4. **Resources** - Tools, templates, community links