# Self-contained nocodo-manager Binary

This document describes the implementation of a self-contained nocodo-manager binary with embedded web assets and auto-launch functionality as specified in GitHub issue #95.

## Overview

The self-contained binary embeds the complete manager-web application assets and automatically launches the default browser on startup, providing a seamless single-file distribution for testing and deployment.

## Architecture

### Embedded Web Assets
- **rust-embed**: Static asset embedding at compile time
- **mime_guess**: Proper MIME type detection for web assets  
- **Fallback Support**: Filesystem serving for development mode
- **Size Optimization**: Source maps excluded, assets compressed

### Browser Auto-launch
- **Cross-platform**: Native browser launching for Linux, macOS, Windows
- **Health Checks**: Wait for server readiness before launching
- **CLI Control**: `--no-browser` flag to disable auto-launch
- **Error Handling**: Graceful fallback with manual instructions

### Build Process
- **GitHub Actions**: Automated multi-platform builds
- **Asset Integration**: manager-web built before Rust compilation
- **Release Automation**: Version tagging with `issue-[x]-[commit-count]` format
- **Artifact Generation**: Cross-platform binaries for distribution

## Implementation Details

### Core Components

1. **embedded_web.rs** - Web asset embedding and serving
   - `WebAssets` - rust-embed derived struct for asset access
   - `handle_embedded_file()` - Actix-web handler for embedded assets
   - `configure_embedded_routes()` - Route configuration for SPA support
   - Asset validation and size calculation utilities

2. **browser_launcher.rs** - Cross-platform browser launching
   - `BrowserConfig` - Configuration for launch behavior
   - `launch_browser()` - Async browser launching with retries
   - `wait_for_server()` - Server readiness health checks
   - Platform-specific launch implementations

3. **build.rs** - Build-time asset processing
   - Automatic manager-web building in CI/release mode
   - Asset validation and size calculation
   - Development mode warnings and fallbacks
   - Environment variable configuration

### File Structure
```
manager/
├── src/
│   ├── embedded_web.rs      # Web asset embedding
│   ├── browser_launcher.rs  # Browser auto-launch
│   ├── main.rs              # Updated with CLI and browser support
│   └── lib.rs               # Module exports
├── build.rs                 # Build-time web asset processing
└── Cargo.toml              # Dependencies: rust-embed, webbrowser, mime_guess

.github/workflows/
└── release.yml             # Multi-platform build automation

manager-web/
└── dist/                   # Built assets (embedded in binary)
```

## Usage

### Command Line Interface
```bash
# Default: auto-launch browser
./nocodo-manager

# Disable browser auto-launch
./nocodo-manager --no-browser

# Custom config file
./nocodo-manager -c /path/to/config.toml

# Help
./nocodo-manager --help
```

### Development Workflow
```bash
# Build web assets
cd manager-web && npm run build

# Build manager binary with embedded assets  
cd manager && cargo build --release

# Test the complete solution
./test_embedded_build.sh
```

## Release Process

### Automated Builds
GitHub Actions workflow triggers on:
- Push to `issue-*` branches
- Pull requests to main
- Manual workflow dispatch

### Build Matrix
- **Linux x86_64**: `nocodo-manager-linux-x64`
- **macOS Intel**: `nocodo-manager-macos-x64`  
- **macOS Apple Silicon**: `nocodo-manager-macos-arm64`
- **Windows x86_64**: `nocodo-manager-windows-x64.exe`

### Version Tagging
Release tags follow the pattern: `issue-[number]-[commit-count]`

Example: `issue-95-3` = Issue #95, 3rd commit in the branch

### Release Artifacts
Each build generates:
- Cross-platform binaries with embedded web assets
- Release notes with build information
- Automated artifact uploads to GitHub releases

## Configuration

### Environment Variables
- `NOCODO_TERMINAL_RUNNER_ENABLED=1` - Enable PTY terminal runner (default: true)
- `NOCODO_RUNNER_ENABLED=1` - Enable AI session runner (default: false)
- `CI=1` - Detected in GitHub Actions for build behavior
- `SKIP_WEB_BUILD=1` - Skip web asset building (development)

### Build-time Variables
- `WEB_ASSETS_AVAILABLE` - Set to 1 when assets are embedded
- `WEB_ASSETS_SIZE` - Total embedded asset size in bytes

## Security Considerations

### Asset Integrity
- Assets embedded at compile time, cannot be modified at runtime
- Content-Type headers properly set based on file extensions
- Caching headers configured appropriately (static vs. HTML)

### Browser Launching
- No shell injection - direct process spawning only
- Platform-specific safe command execution
- Graceful fallback when browser launching fails

## Performance Characteristics

### Binary Size
- Embedded assets increase binary size (~1-5MB typical)
- Source maps excluded to minimize size impact
- Compression applied where possible

### Startup Time
- Asset validation runs on startup (~10ms)
- Browser launching delayed 2 seconds for server readiness
- Health checks with timeout protection

### Runtime Performance
- In-memory asset serving (faster than filesystem)
- ETag support for client-side caching
- No disk I/O for web requests

## Development vs. Release Modes

### Development Mode
- Web assets served from filesystem if available
- Build warnings when assets missing
- Hot reload support via Vite dev server proxy
- Graceful fallbacks for missing components

### Release Mode
- Web assets must be successfully embedded
- Build fails if assets cannot be generated
- All dependencies bundled in single executable
- No external file dependencies

## Testing Strategy

### Unit Tests
- Asset embedding validation
- Browser configuration testing  
- Platform-specific launch logic
- Error handling scenarios

### Integration Tests
- Complete build pipeline testing
- Multi-platform binary validation
- Browser launch functionality
- Server startup and asset serving

### Automated Testing
- GitHub Actions test matrix
- Asset embedding verification
- Binary smoke testing
- Cross-platform compatibility checks

## Troubleshooting

### Common Issues

1. **Web Assets Not Embedded**
   ```
   Solution: Ensure manager-web/dist exists and contains built assets
   Command: cd manager-web && npm run build
   ```

2. **Browser Won't Launch**
   ```
   Solution: Use --no-browser flag and open manually
   Check: Platform-specific browser availability
   ```

3. **Build Failures in CI**
   ```
   Solution: Verify Node.js and Rust toolchains in workflow
   Check: npm dependencies and build scripts
   ```

4. **Large Binary Size**
   ```
   Solution: Exclude unnecessary assets, check embedded file list
   Monitor: Binary size growth in releases
   ```

## Future Enhancements

- **Progressive Loading**: Lazy load non-critical assets
- **Compression**: Brotli/gzip compression for assets
- **Hot Updates**: Runtime asset updating capabilities
- **Themes**: Multiple UI theme embedding
- **Localization**: Multi-language asset support
- **Desktop Integration**: System tray and native notifications

## Dependencies

### Runtime Dependencies
- `rust-embed` - Static file embedding
- `webbrowser` - Cross-platform browser launching
- `mime_guess` - MIME type detection
- `clap` - Command line argument parsing

### Build Dependencies  
- Node.js 20+ and npm for manager-web building
- Rust 1.70+ for manager compilation
- Platform-specific build tools (handled by GitHub Actions)

### Development Dependencies
- `actix-rt`, `tempfile` for testing
- GitHub Actions runners (ubuntu, macos, windows)

This implementation provides a complete solution for distributing nocodo-manager as a single executable with embedded web interface and automated browser launching across all major platforms.