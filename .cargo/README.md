# Cargo Configuration

## sccache Setup (Optional)

To enable build caching with sccache, set the `RUSTC_WRAPPER` environment variable:

### Linux/macOS
```bash
export RUSTC_WRAPPER=/usr/bin/sccache
```

Add to your `~/.bashrc` or `~/.zshrc` for persistence.

### Windows (PowerShell)
```powershell
$env:RUSTC_WRAPPER = "sccache"
```

Add to your PowerShell profile for persistence.

### Installation

If sccache is not installed:
```bash
cargo install sccache
```

This approach allows each developer to enable/disable sccache without affecting the shared repository configuration.
