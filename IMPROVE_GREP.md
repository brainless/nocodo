# Improve Grep Tool: Migration to Ripgrep

## Current State
- Custom implementation using `walkdir` + `regex`
- Manual exclusion patterns (no .gitignore support)
- Basic binary file filtering
- Response size limiting implemented

## Available Ripgrep Dependencies
```
grep v0.4.1
├── grep-cli v0.1.12
├── grep-matcher v0.1.8
├── grep-printer v0.3.1
├── grep-regex v0.1.14
└── grep-searcher v0.1.16
```

## Migration Plan

### Phase 1: Core Integration
1. Replace `walkdir` with `grep-searcher::Searcher`
2. Use `grep-regex` for pattern matching
3. Leverage `grep-searcher`'s built-in .gitignore support

### Phase 2: Enhanced Features
1. Automatic .gitignore parsing and respect
2. Better binary file detection via `grep-searcher`
3. Improved performance with parallel search capabilities

### Phase 3: API Compatibility
1. Maintain existing `GrepRequest`/`GrepResponse` models
2. Preserve current parameter semantics
3. Add new optional parameters for ripgrep-specific features

## Implementation Steps
1. Create new `RipgrepExecutor` struct
2. Implement search using `grep-searcher::SearcherBuilder`
3. Add .gitignore support via `grep-searcher::sinks::UTF8`
4. Migrate existing exclusion logic to ripgrep patterns
5. Update tests and benchmarks
6. Replace old implementation

## Benefits
- Automatic .gitignore respect
- Better performance (parallel search)
- Mature, battle-tested search logic
- Reduced maintenance burden
