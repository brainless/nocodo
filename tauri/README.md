# Nocodo Tauri App

Desktop shell for `admin-gui` with `nocodo-backend` managed as a sidecar process.

## Development

```bash
npm --prefix tauri install
NOCODO_BACKEND_PATH="$(pwd)/target/debug/nocodo-backend" npm --prefix tauri run dev
```

This will:
- build `nocodo-backend`
- run `admin-gui` dev server on `http://127.0.0.1:${ADMIN_GUI_PORT}` from `project.conf` (current repo value: `6626`)
- launch the Tauri desktop window

## Build

```bash
cd tauri
npm run build
```

For bundle builds, put a sidecar binary at:

`tauri/bin/nocodo-backend`

Or override sidecar path at runtime with:

`NOCODO_BACKEND_PATH=/absolute/path/to/nocodo-backend`

Compatibility alias:

`DWATA_API_PATH=/absolute/path/to/nocodo-backend`
