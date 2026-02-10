# CONFIG-005: Rust Persistence Patterns Analysis

## Existing Persistence Architecture

The Rust persistence module follows consistent patterns that the credentials system should replicate.

---

## Pattern 1: Global Singleton with Lazy Initialization

**File: `codelet/napi/src/persistence/mod.rs`**

```rust
use std::sync::Mutex;

lazy_static::lazy_static! {
    static ref MESSAGE_STORE: Mutex<Option<MessageStore>> = Mutex::new(None);
    static ref SESSION_STORE: Mutex<Option<SessionStore>> = Mutex::new(None);
    static ref BLOB_STORE: Mutex<Option<BlobStore>> = Mutex::new(None);
    static ref HISTORY_STORE: Mutex<Option<HistoryStore>> = Mutex::new(None);
}

fn init_stores() -> Result<(), String> {
    let mut msg = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    if msg.is_none() {
        *msg = Some(MessageStore::new()?);
    }
    // ... repeat for other stores
    Ok(())
}
```

**For Credentials:**
```rust
lazy_static::lazy_static! {
    static ref CREDENTIAL_STORE: Mutex<Option<CredentialStore>> = Mutex::new(None);
}

fn init_credential_store() -> Result<(), String> {
    let mut store = CREDENTIAL_STORE.lock().map_err(|e| e.to_string())?;
    if store.is_none() {
        *store = Some(CredentialStore::new()?);
    }
    Ok(())
}
```

---

## Pattern 2: Single Source of Truth for Data Directory

**File: `codelet/common/src/data_dir.rs`**

```rust
static DATA_DIRECTORY: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn set_data_directory(dir: PathBuf) -> Result<(), String> {
    let mut guard = DATA_DIRECTORY.lock().map_err(|e| e.to_string())?;
    *guard = Some(dir);
    Ok(())
}

pub fn get_data_dir() -> Result<PathBuf, String> {
    let guard = DATA_DIRECTORY.lock().map_err(|e| e.to_string())?;
    guard.clone().ok_or_else(|| "Data directory not initialized".to_string())
}
```

**For Credentials:**
- Use same `get_data_dir()` to derive credentials path
- Path: `{data_dir}/credentials/credentials.json`
- Follows existing pattern for sessions, messages, blobs

---

## Pattern 3: Store Struct with In-Memory Cache

**File: `codelet/napi/src/persistence/storage.rs`**

```rust
pub struct SessionStore {
    sessions_dir: PathBuf,
    cache: HashMap<Uuid, SessionManifest>,
    last_session: HashMap<PathBuf, Uuid>,
}

impl SessionStore {
    pub fn new() -> Result<Self, String> {
        ensure_directories()?;
        let sessions_dir = get_data_dir()?.join("sessions");
        let mut store = Self {
            sessions_dir,
            cache: HashMap::new(),
            last_session: HashMap::new(),
        };
        store.load_all()?;
        Ok(store)
    }

    fn load_all(&mut self) -> Result<(), String> {
        // Load from disk into cache
    }

    pub fn get(&self, id: Uuid) -> Option<&SessionManifest> {
        self.cache.get(&id)
    }

    pub fn save(&mut self, session: &SessionManifest) -> Result<(), String> {
        // Write to disk AND update cache
    }
}
```

**For Credentials:**
```rust
pub struct CredentialStore {
    credentials_file: PathBuf,
    cache: CredentialsFile,
    last_mtime: Option<SystemTime>,
}

impl CredentialStore {
    pub fn new() -> Result<Self, String> {
        ensure_credentials_directory()?;
        let credentials_file = get_data_dir()?.join("credentials/credentials.json");
        let mut store = Self {
            credentials_file,
            cache: CredentialsFile::default(),
            last_mtime: None,
        };
        store.reload_if_changed()?;
        Ok(store)
    }

    /// Check file mtime and reload if changed
    pub fn reload_if_changed(&mut self) -> Result<bool, String> {
        let current_mtime = std::fs::metadata(&self.credentials_file)
            .ok()
            .and_then(|m| m.modified().ok());
        
        if current_mtime != self.last_mtime {
            self.load_from_disk()?;
            self.last_mtime = current_mtime;
            return Ok(true); // Changed
        }
        Ok(false) // No change
    }

    pub fn get_api_key(&mut self, provider_id: &str) -> Option<String> {
        // Auto-reload on access
        let _ = self.reload_if_changed();
        self.cache.providers.get(provider_id).map(|c| c.api_key.clone())
    }
}
```

---

## Pattern 4: High-Level API Functions with Init Check

**File: `codelet/napi/src/persistence/mod.rs`**

```rust
pub fn create_session(name: &str, project: &Path) -> Result<SessionManifest, String> {
    init_stores()?;  // <-- Always init first
    let mut store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store.as_mut().ok_or("Session store not initialized")?.create(name, project)
}

pub fn load_session(id: Uuid) -> Result<SessionManifest, String> {
    init_stores()?;
    let store = SESSION_STORE.lock().map_err(|e| e.to_string())?;
    store.as_ref().ok_or("Session store not initialized")?.load(id)
}
```

**For Credentials:**
```rust
/// Resolve API key for a provider (checks file, env vars, .env)
pub fn resolve_credential(provider_id: &str, project_dir: Option<&Path>) -> Result<Option<String>, String> {
    init_credential_store()?;
    
    let mut store = CREDENTIAL_STORE.lock().map_err(|e| e.to_string())?;
    let store_ref = store.as_mut().ok_or("Credential store not initialized")?;
    
    // 1. Check credentials file (auto-reloads if changed)
    if let Some(key) = store_ref.get_api_key(provider_id) {
        return Ok(Some(key));
    }
    
    // 2. Check environment variable
    if let Some(key) = get_env_api_key(provider_id) {
        return Ok(Some(key));
    }
    
    // 3. Check .env file in project directory
    if let Some(project) = project_dir {
        if let Some(key) = get_dotenv_api_key(provider_id, project) {
            return Ok(Some(key));
        }
    }
    
    Ok(None)
}

fn get_env_api_key(provider_id: &str) -> Option<String> {
    let env_var = match provider_id {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        // ... other providers
        _ => return None,
    };
    std::env::var(env_var).ok()
}
```

---

## Pattern 5: Separate NAPI Bindings Module

**File: `codelet/napi/src/persistence/napi_bindings.rs`**

```rust
#[napi]
pub fn persistence_create_session(name: String, project: String) -> Result<NapiSessionManifest> {
    create_session(&name, &PathBuf::from(project))
        .map(|s| s.into())
        .map_err(Error::from_reason)
}
```

**For Credentials:**
```rust
// codelet/napi/src/credentials/napi_bindings.rs

/// Resolve API key for a provider
/// Returns the key if found, null if not found
/// Checks: credentials file > env vars > .env file
#[napi]
pub fn credentials_resolve(provider_id: String, project: Option<String>) -> Result<Option<String>> {
    let project_path = project.map(PathBuf::from);
    resolve_credential(&provider_id, project_path.as_deref())
        .map_err(Error::from_reason)
}

/// Force reload credentials from disk
/// Call this after TypeScript updates the credentials file
#[napi]
pub fn credentials_reload() -> Result<bool> {
    init_credential_store()?;
    let mut store = CREDENTIAL_STORE.lock().map_err(|e| Error::from_reason(e.to_string()))?;
    store.as_mut()
        .ok_or_else(|| Error::from_reason("Credential store not initialized"))?
        .reload_if_changed()
        .map_err(Error::from_reason)
}
```

---

## Pattern 6: Module Structure

**Existing persistence structure:**
```
codelet/napi/src/persistence/
├── mod.rs              # Module exports, global stores, high-level API
├── storage.rs          # MessageStore, SessionStore structs
├── types.rs            # SessionManifest, StoredMessage, etc.
├── blob.rs             # BlobStore
├── blob_processing.rs  # Blob content extraction/rehydration
├── history.rs          # HistoryStore
├── message_envelope.rs # MessageEnvelope types
├── napi_bindings.rs    # NAPI exports
└── tests.rs            # Unit tests
```

**Proposed credentials structure:**
```
codelet/napi/src/credentials/
├── mod.rs              # Module exports, global store, high-level API
├── store.rs            # CredentialStore struct
├── types.rs            # CredentialsFile, ProviderCredential
├── resolver.rs         # Priority chain resolution logic
├── napi_bindings.rs    # NAPI exports
└── tests.rs            # Unit tests
```

---

## Type Definitions

**File: `codelet/napi/src/credentials/types.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Credential for a single provider
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCredential {
    pub api_key: String,
    pub last_updated: DateTime<Utc>,
}

/// Credentials file structure (matches TypeScript CredentialsFile)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsFile {
    pub version: u32,
    pub providers: HashMap<String, ProviderCredential>,
}

/// Source of a resolved credential
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialSource {
    File,      // ~/.fspec/credentials/credentials.json
    Env,       // Environment variable
    DotEnv,    // .env file in project
}

/// Result of credential resolution
#[derive(Debug, Clone)]
pub struct ResolvedCredential {
    pub api_key: String,
    pub source: CredentialSource,
}
```

---

## Integration with Session Manager

**Current (CONFIG-004 approach - TypeScript passes key):**
```rust
pub async fn create_session_with_id(
    session_id: String,
    model: String,
    project: String,
    name: String,
    api_key: Option<String>,  // <-- Passed from TypeScript
) -> Result<()> {
    // ...
    if let Some(key) = api_key {
        std::env::set_var(env_var_name, key);
    }
    // ...
}
```

**Proposed (CONFIG-005 approach - Rust resolves):**
```rust
pub async fn create_session_with_id(
    session_id: String,
    model: String,
    project: String,
    name: String,
) -> Result<()> {
    // Extract provider from model string
    let provider_id = model.split('/').next().unwrap_or("");
    
    // Resolve credential in Rust (checks file, env, .env)
    if let Some(resolved) = resolve_credential(provider_id, Some(&PathBuf::from(&project)))? {
        let env_var_name = get_env_var_name(provider_id);
        if !env_var_name.is_empty() {
            std::env::set_var(env_var_name, &resolved.api_key);
        }
    }
    
    // ... rest of session creation
}
```

**On Session Resume (also resolves fresh credentials):**
```rust
impl SessionManager {
    pub fn resume_session(&self, session_id: &str) -> Result<()> {
        let session = self.get_session(session_id)?;
        
        // Re-resolve credentials on resume (handles key changes)
        if let Some(ref provider) = session.provider_id {
            if let Some(resolved) = resolve_credential(provider, session.project.as_deref())? {
                let env_var_name = get_env_var_name(provider);
                if !env_var_name.is_empty() {
                    std::env::set_var(env_var_name, &resolved.api_key);
                }
            }
        }
        
        Ok(())
    }
}
```

---

## DRY/SOLID/COMPOSABLE Summary

| Principle | How It's Applied |
|-----------|------------------|
| **DRY** | Reuse `get_data_dir()`, same init pattern, same store trait approach |
| **Single Responsibility** | CredentialStore only handles credentials; resolver only handles priority chain |
| **Open/Closed** | New providers added via env var mapping, not code changes |
| **Interface Segregation** | Small focused NAPI functions (`credentials_resolve`, `credentials_reload`) |
| **Dependency Inversion** | SessionManager depends on abstract `resolve_credential()`, not concrete file I/O |
| **Composable** | Credential module is independent, can be tested in isolation |

---

## Migration Checklist

1. [ ] Create `codelet/napi/src/credentials/` module structure
2. [ ] Implement `CredentialStore` with mtime-based reload
3. [ ] Implement `resolve_credential()` with priority chain
4. [ ] Add NAPI bindings (`credentials_resolve`, `credentials_reload`)
5. [ ] Update `SessionManager::create_session_with_id()` to call Rust resolver
6. [ ] Update `SessionManager` resume logic to re-resolve credentials
7. [ ] Update TypeScript to call `credentials_reload()` after saving credentials
8. [ ] Remove `api_key` parameter from `sessionManagerCreateWithId` NAPI
9. [ ] Update TypeScript to not pass API keys
10. [ ] Add tests for credential resolution priority
11. [ ] Add tests for mtime-based reload
