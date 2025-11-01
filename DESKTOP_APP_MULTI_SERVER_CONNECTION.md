# Desktop App Multi-Server Connection & Auth State Management

This document outlines the phased approach to implementing proper per-server authentication state management and multi-server support in the nocodo desktop application.

## Background

The desktop app needs to support:
1. **Multiple server connections** - Users may connect to different nocodo manager instances (personal, work, client servers)
2. **Per-server authentication** - Each server has independent user accounts and JWT tokens
3. **Auth state tracking** - When JWT expires or user is kicked, the app should detect and show login dialog
4. **Shared ApiClient state** - JWT token updates must propagate to all parts of the app immediately

## Implementation Phases

---

## ✅ Phase 1: Shared ApiClient with Arc (COMPLETED)

**Status:** Implemented and tested

**Problem Solved:**
- JWT token updates from login weren't propagating to cloned ApiClient instances
- Different parts of the app had stale ApiClient instances without JWT tokens

**Changes Made:**
- Changed `ConnectionManager.api_client` from `Arc<RwLock<Option<ApiClient>>>` to `Arc<RwLock<Option<Arc<RwLock<ApiClient>>>>>`
- Updated `get_api_client()` to return `Option<Arc<RwLock<ApiClient>>>` instead of `Option<ApiClient>`
- Updated all call sites to use `api_client_arc.read().await` to access the shared instance
- JWT token updates in `login()` now immediately affect all consumers

**Files Modified:**
- `desktop-app/src/connection_manager.rs`
- `desktop-app/src/services/api.rs`
- `desktop-app/src/components/connection_dialog.rs`
- `desktop-app/src/pages/project_detail.rs`

**Result:**
- All parts of the app now share the same ApiClient instance
- JWT token updates propagate immediately
- No more 401 errors after successful login

---

## 📋 Phase 2: Per-Server Auth State

**Goal:** Track authentication state separately for each server connection

### Data Structures

```rust
// In desktop-app/src/state/connection.rs or new auth.rs

/// Authentication state for a specific server connection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerAuthState {
    /// Unique identifier for this server (e.g., "user@host:port")
    pub server_id: String,

    /// Server hostname/IP
    pub server_host: String,

    /// SSH username used for connection
    pub server_user: String,

    /// JWT token for this server
    pub jwt_token: Option<String>,

    /// User ID on this server
    pub user_id: Option<i64>,

    /// Username on this server (may differ from SSH user)
    pub username: Option<String>,

    /// Whether currently authenticated
    pub is_authenticated: bool,

    /// Last authentication error (expired token, kicked out, etc.)
    pub auth_error: Option<String>,

    /// Timestamp of last successful authentication
    pub last_auth_time: Option<i64>,

    /// Whether this is the first user (super admin) on this server
    pub is_first_user: bool,
}

impl ServerAuthState {
    /// Create a new unauthenticated state for a server
    pub fn new(server_host: String, server_user: String) -> Self {
        Self {
            server_id: format!("{}@{}", server_user, server_host),
            server_host,
            server_user,
            jwt_token: None,
            user_id: None,
            username: None,
            is_authenticated: false,
            auth_error: None,
            last_auth_time: None,
            is_first_user: false,
        }
    }

    /// Mark as authenticated after successful login
    pub fn mark_authenticated(&mut self, token: String, user_id: i64, username: String) {
        self.jwt_token = Some(token);
        self.user_id = Some(user_id);
        self.username = Some(username);
        self.is_authenticated = true;
        self.auth_error = None;
        self.last_auth_time = Some(chrono::Utc::now().timestamp());
    }

    /// Mark as unauthenticated (token expired, kicked, etc.)
    pub fn mark_unauthenticated(&mut self, reason: String) {
        self.is_authenticated = false;
        self.auth_error = Some(reason);
        // Keep jwt_token for display purposes, but mark as invalid
    }

    /// Clear all auth data (logout)
    pub fn clear(&mut self) {
        self.jwt_token = None;
        self.user_id = None;
        self.username = None;
        self.is_authenticated = false;
        self.auth_error = None;
        self.last_auth_time = None;
    }
}
```

### AppState Changes

```rust
// In desktop-app/src/state/mod.rs

pub struct AppState {
    // ... existing fields ...

    /// Authentication states for all known servers
    /// Key: server_id (e.g., "user@host:port")
    #[serde(skip)]
    pub server_auth_states: Arc<RwLock<HashMap<String, ServerAuthState>>>,

    /// Currently active server connection
    #[serde(skip)]
    pub active_server_id: Arc<RwLock<Option<String>>>,
}

impl AppState {
    /// Get auth state for the currently active server
    pub async fn get_active_auth_state(&self) -> Option<ServerAuthState> {
        let server_id = self.active_server_id.read().await.clone()?;
        let states = self.server_auth_states.read().await;
        states.get(&server_id).cloned()
    }

    /// Check if authenticated on the current server
    pub async fn is_authenticated(&self) -> bool {
        self.get_active_auth_state()
            .await
            .map(|s| s.is_authenticated)
            .unwrap_or(false)
    }

    /// Update auth state for a specific server
    pub async fn update_server_auth_state<F>(&self, server_id: &str, update_fn: F)
    where
        F: FnOnce(&mut ServerAuthState),
    {
        let mut states = self.server_auth_states.write().await;
        if let Some(state) = states.get_mut(server_id) {
            update_fn(state);
        }
    }
}
```

### ConnectionManager Changes

```rust
// In desktop-app/src/connection_manager.rs

impl ConnectionManager {
    /// Get the server ID for the current connection
    pub async fn get_server_id(&self) -> Option<String> {
        match self.connection_type.read().await.as_ref()? {
            ConnectionType::Ssh { server, username, port, .. } => {
                Some(format!("{}@{}:{}", username, server, port))
            }
            ConnectionType::Local { .. } => {
                Some("local@localhost:8081".to_string())
            }
        }
    }

    /// Login and update per-server auth state
    pub async fn login_with_state(
        &self,
        username: &str,
        password: &str,
        ssh_fingerprint: &str,
        server_auth_states: Arc<RwLock<HashMap<String, ServerAuthState>>>,
    ) -> Result<manager_models::LoginResponse, ConnectionError> {
        let response = self.login(username, password, ssh_fingerprint).await?;

        // Update per-server auth state
        if let Some(server_id) = self.get_server_id().await {
            let mut states = server_auth_states.write().await;
            if let Some(state) = states.get_mut(&server_id) {
                state.mark_authenticated(
                    response.token.clone(),
                    response.user.id,
                    response.user.username.clone(),
                );
            }
        }

        Ok(response)
    }
}
```

### Implementation Tasks

1. **Create `ServerAuthState` struct** in `state/connection.rs`
2. **Add `server_auth_states` to `AppState`**
3. **Update `ConnectionManager.login()` to update auth state**
4. **Initialize auth state when connecting to a server**
5. **Persist auth states to local database** (optional, for remembering servers)

### Migration Notes

- Existing `auth_state: AuthState` in `AppState` can be deprecated
- `jwt_token` is duplicated in both `ConnectionManager` and `ServerAuthState` initially
  - Later phases can consolidate to single source of truth

---

## 📋 Phase 3: Reactive Auth State Updates

**Goal:** Automatically detect and respond to authentication failures across the app

### API Call Interceptor Pattern

```rust
// In desktop-app/src/api_client.rs or new api_interceptor.rs

/// Wrapper around ApiClient that intercepts auth errors
pub struct AuthAwareApiClient {
    client: Arc<RwLock<ApiClient>>,
    server_id: String,
    auth_states: Arc<RwLock<HashMap<String, ServerAuthState>>>,
}

impl AuthAwareApiClient {
    /// Execute an API call with automatic auth error handling
    pub async fn execute<T, F, Fut>(
        &self,
        request_fn: F,
    ) -> Result<T, ApiError>
    where
        F: FnOnce(ApiClient) -> Fut,
        Fut: Future<Output = Result<T, ApiError>>,
    {
        let client = self.client.read().await.clone();
        let result = request_fn(client).await;

        // Check for auth errors
        if let Err(ref e) = result {
            if e.is_unauthorized() || e.is_forbidden() {
                tracing::warn!(
                    "API call failed with auth error on server {}: {}",
                    self.server_id,
                    e
                );

                // Update server auth state
                let mut states = self.auth_states.write().await;
                if let Some(state) = states.get_mut(&self.server_id) {
                    state.mark_unauthenticated(format!("API returned auth error: {}", e));
                }
            }
        }

        result
    }
}
```

### Background Auth State Monitor

```rust
// In desktop-app/src/services/auth_monitor.rs

/// Monitors auth state changes and triggers UI updates
pub struct AuthStateMonitor {
    auth_states: Arc<RwLock<HashMap<String, ServerAuthState>>>,
    active_server_id: Arc<RwLock<Option<String>>>,
    auth_dialog_trigger: Arc<std::sync::Mutex<bool>>,
}

impl AuthStateMonitor {
    /// Start monitoring auth state changes
    pub fn start(&self) {
        let auth_states = Arc::clone(&self.auth_states);
        let active_server_id = Arc::clone(&self.active_server_id);
        let auth_dialog_trigger = Arc::clone(&self.auth_dialog_trigger);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                // Check if active server is authenticated
                if let Some(server_id) = active_server_id.read().await.as_ref() {
                    let states = auth_states.read().await;
                    if let Some(state) = states.get(server_id) {
                        if !state.is_authenticated && state.auth_error.is_some() {
                            // Trigger auth dialog
                            if let Ok(mut trigger) = auth_dialog_trigger.lock() {
                                *trigger = true;
                            }
                        }
                    }
                }
            }
        });
    }
}
```

### Implementation Tasks

1. **Create `AuthAwareApiClient` wrapper** for automatic error detection
2. **Update all API calls** to use the wrapper (or add interceptor to existing `ApiClient`)
3. **Create `AuthStateMonitor`** background task
4. **Update UI** to react to `auth_dialog_trigger` changes
5. **Add logging** for auth state transitions

### Alternative: Event-Based Approach

Instead of polling, use channels for immediate notification:

```rust
pub struct AuthStateEvents {
    auth_required_tx: tokio::sync::broadcast::Sender<String>, // server_id
}

// When 401 detected:
auth_required_tx.send(server_id).ok();

// In UI update loop:
if let Ok(server_id) = auth_required_rx.try_recv() {
    show_login_dialog_for_server(server_id);
}
```

---

## 📋 Phase 4: Multi-Server UI & Connection Management

**Goal:** Support multiple simultaneous server connections with UI to switch between them

### Multiple ConnectionManagers

```rust
// In desktop-app/src/state/mod.rs

pub struct AppState {
    /// Connection managers for each server
    /// Key: server_id (e.g., "user@host:port")
    #[serde(skip)]
    pub connection_managers: Arc<RwLock<HashMap<String, Arc<ConnectionManager>>>>,

    /// Currently active server
    #[serde(skip)]
    pub active_server_id: Arc<RwLock<Option<String>>>,
}

impl AppState {
    /// Get the active connection manager
    pub async fn get_active_connection_manager(&self) -> Option<Arc<ConnectionManager>> {
        let server_id = self.active_server_id.read().await.clone()?;
        let managers = self.connection_managers.read().await;
        managers.get(&server_id).cloned()
    }

    /// Switch to a different server
    pub async fn switch_to_server(&mut self, server_id: String) -> Result<(), String> {
        let managers = self.connection_managers.read().await;

        if !managers.contains_key(&server_id) {
            return Err(format!("Server {} not found", server_id));
        }

        *self.active_server_id.write().await = Some(server_id.clone());

        // Refresh data for the new server
        // (projects, works, settings, etc.)

        Ok(())
    }

    /// Add a new server connection
    pub async fn add_server_connection(
        &mut self,
        server_id: String,
        connection_manager: Arc<ConnectionManager>,
    ) {
        let mut managers = self.connection_managers.write().await;
        managers.insert(server_id.clone(), connection_manager);

        // Initialize auth state for this server
        let mut auth_states = self.server_auth_states.write().await;
        if !auth_states.contains_key(&server_id) {
            // Parse server_id to extract host/user
            let parts: Vec<&str> = server_id.split('@').collect();
            if parts.len() == 2 {
                let user = parts[0].to_string();
                let host = parts[1].to_string();
                auth_states.insert(
                    server_id.clone(),
                    ServerAuthState::new(host, user),
                );
            }
        }
    }
}
```

### UI: Server Switcher

```rust
// In desktop-app/src/components/server_switcher.rs

pub struct ServerSwitcher;

impl ServerSwitcher {
    pub fn ui(&self, ui: &mut egui::Ui, state: &mut AppState) {
        ui.horizontal(|ui| {
            ui.label("Server:");

            // Dropdown to select active server
            let active_server_id = state.active_server_id.blocking_read().clone();
            let servers: Vec<String> = state
                .connection_managers
                .blocking_read()
                .keys()
                .cloned()
                .collect();

            egui::ComboBox::from_id_salt("server_switcher")
                .selected_text(active_server_id.clone().unwrap_or_else(|| "None".to_string()))
                .show_ui(ui, |ui| {
                    for server_id in servers {
                        let auth_state = state
                            .server_auth_states
                            .blocking_read()
                            .get(&server_id)
                            .cloned();

                        let label = if let Some(auth) = auth_state {
                            if auth.is_authenticated {
                                format!("✓ {} ({})", server_id, auth.username.unwrap_or_default())
                            } else {
                                format!("⚠ {} (not authenticated)", server_id)
                            }
                        } else {
                            server_id.clone()
                        };

                        if ui.selectable_label(
                            active_server_id.as_ref() == Some(&server_id),
                            label
                        ).clicked() {
                            // Switch to this server
                            tokio::spawn({
                                let mut state = state.clone();
                                let server_id = server_id.clone();
                                async move {
                                    state.switch_to_server(server_id).await.ok();
                                }
                            });
                        }
                    }
                });

            // Button to add new server
            if ui.button("+").clicked() {
                state.ui_state.show_connection_dialog = true;
            }
        });
    }
}
```

### Data Isolation Per Server

```rust
// Each server should have its own data cache
pub struct ServerDataCache {
    pub projects: Vec<manager_models::Project>,
    pub works: Vec<manager_models::Work>,
    pub settings: Option<manager_models::SettingsResponse>,
    pub supported_models: Vec<manager_models::SupportedModel>,
}

// In AppState:
pub server_data_caches: Arc<RwLock<HashMap<String, ServerDataCache>>>,
```

### Implementation Tasks

1. **Support multiple `ConnectionManager` instances** in `AppState`
2. **Create `ServerSwitcher` component** for UI
3. **Isolate data per server** (projects, works, etc.)
4. **Update all API calls** to use active connection manager
5. **Add "Add Server" flow** to connection dialog
6. **Persist server list** to local database
7. **Handle concurrent connections** (background refresh for all servers)

### Future Enhancements

- **Server groups** (Personal, Work, Clients)
- **Favorite servers** with quick-switch
- **Connection health indicators** in UI
- **Notifications** for events on background servers
- **Workspace concept** - different window layouts per server

---

## Migration Path

### From Current State → Phase 2

1. Keep existing `ConnectionManager` singleton
2. Add `server_auth_states` to `AppState`
3. Populate single entry in `server_auth_states` for current connection
4. Update login flow to populate auth state
5. Test auth state updates work correctly

### From Phase 2 → Phase 3

1. Add `AuthAwareApiClient` wrapper (optional, can modify `ApiClient` directly)
2. Update API calls to detect 401/403 and update auth state
3. Add `AuthStateMonitor` background task
4. Connect auth state changes to login dialog trigger
5. Test auth expiration and re-authentication flow

### From Phase 3 → Phase 4

1. Change `connection_manager` from `Arc<ConnectionManager>` to `HashMap<String, Arc<ConnectionManager>>`
2. Add server switcher UI component
3. Update data structures to be per-server
4. Update all API call sites to use active connection manager
5. Add server persistence to local database
6. Test multi-server switching

---

## Testing Checklist

### Phase 2 Testing
- [ ] Login updates `ServerAuthState` correctly
- [ ] Auth state persists across app restarts (if persistence added)
- [ ] Logout clears auth state
- [ ] Multiple server entries can coexist in `server_auth_states`

### Phase 3 Testing
- [ ] 401 error automatically marks server as unauthenticated
- [ ] Login dialog appears when auth required
- [ ] Re-authentication updates shared `ApiClient` JWT token
- [ ] All pending API calls retry after successful re-auth
- [ ] Background tasks detect auth state changes

### Phase 4 Testing
- [ ] Can connect to multiple servers simultaneously
- [ ] Switching servers updates UI data correctly
- [ ] Each server maintains separate auth state
- [ ] Adding/removing servers works correctly
- [ ] Server list persists across app restarts
- [ ] Background data refresh for inactive servers

---

## Security Considerations

1. **JWT Token Storage**
   - Phase 2: Store in memory only (current approach)
   - Future: Optionally encrypt and store in local DB for auto-login
   - Use platform keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)

2. **Token Expiration**
   - Parse JWT `exp` claim and proactively show login dialog before expiration
   - Implement token refresh endpoint on manager side

3. **Multi-User Desktop**
   - Consider encrypting local database with user password
   - Clear sensitive data on app close (optional setting)

4. **Audit Logging**
   - Log all auth state changes (login, logout, expiration, kick)
   - Store in local database for debugging

---

## Performance Considerations

1. **Connection Pooling**
   - Reuse SSH tunnels when possible
   - Implement connection pooling for multiple servers

2. **Background Refresh**
   - Limit concurrent API calls per server
   - Implement exponential backoff for failed auth attempts

3. **Data Caching**
   - Cache API responses per server
   - Implement cache invalidation on data changes

4. **Memory Usage**
   - Limit number of simultaneous connections
   - Unload data for inactive servers after timeout

---

## Open Questions

1. **Should we support offline mode?**
   - Cache data locally and allow read-only access when disconnected?

2. **How to handle server version mismatches?**
   - Different manager versions may have different API schemas
   - Show warning in UI? Block connection?

3. **Server discovery?**
   - Auto-discover nocodo managers on local network?
   - Import server list from config file?

4. **Cross-server operations?**
   - Copy/move projects between servers?
   - Unified search across all servers?

---

*Document Version: 1.0*
*Last Updated: 2025-11-01*
*Author: Claude (Anthropic)*
