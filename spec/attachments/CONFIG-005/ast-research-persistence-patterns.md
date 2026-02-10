# AST Research: Persistence Module Patterns for CONFIG-005

## Purpose
Analyze existing persistence module patterns to guide credential management implementation.

## Research Date
2026-02-10

## Pattern 1: Global Singleton Stores with lazy_static

**File:** `codelet/napi/src/persistence/mod.rs`

```rust
use std::sync::Mutex;

// Global singleton stores (thread-safe)
lazy_static::lazy_static! {
    static ref MESSAGE_STORE: Mutex<Option<MessageStore>> = Mutex::new(None);
    static ref SESSION_STORE: Mutex<Option<SessionStore>> = Mutex::new(None);
    static ref BLOB_STORE: Mutex<Option<BlobStore>> = Mutex::new(None);
    static ref HISTORY_STORE: Mutex<Option<HistoryStore>> = Mutex::new(None);
}
```

**Application for Credentials:**
```rust
lazy_static::lazy_static! {
    static ref CREDENTIAL_STORE: Mutex<Option<CredentialStore>> = Mutex::new(None);
}
```

## Pattern 2: Store Initialization

**File:** `codelet/napi/src/persistence/mod.rs`

```rust
pub fn set_data_directory(dir: PathBuf) -> Result<(), String> {
    codelet_common::set_data_directory(dir)?;

    // Reset stores so they reinitialize with the new directory
    let mut msg = MESSAGE_STORE.lock().map_err(|e| e.to_string())?;
    *msg = None;
    drop(msg);
    // ... repeat for other stores
    Ok(())
}

pub fn get_data_dir() -> Result<PathBuf, String> {
    codelet_common::get_data_dir()
}

pub fn ensure_directories() -> Result<(), String> {
    let base = get_data_dir()?;
    let dirs = [base.join("messages"), base.join("sessions"), base.join("blobs")];
    for dir in &dirs {
        std::fs::create_dir_all(dir)?;
    }
    Ok(())
}
```

## Pattern 3: Store Struct with Cache

**File:** `codelet/napi/src/persistence/storage.rs`

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
}
```

## Pattern 4: Session Manager create_session_with_id Signature

**File:** `codelet/napi/src/session_manager.rs:3550`

```rust
pub async fn create_session_with_id(
    &self, 
    id: &str, 
    model: &str, 
    project: &str, 
    name: &str, 
    api_key: Option<&str>  // <-- This parameter should be removed
) -> Result<()> {
    // ...
}
```

## AST Search Results

### Persistence Structs Found
```
codelet/napi/src/persistence/storage.rs:15   - MessageStore
codelet/napi/src/persistence/storage.rs:172  - SessionStore
codelet/napi/src/persistence/blob.rs:13      - BlobStore
codelet/napi/src/persistence/history.rs:13   - HistoryStore
codelet/napi/src/persistence/types.rs:*      - Various types
```

### lazy_static Usage Locations
```
codelet/napi/src/persistence/mod.rs    - Main store singletons
codelet/napi/src/test_support.rs       - Test utilities
codelet/napi/src/lib.rs                - Other global state
codelet/napi/src/work_units_watcher.rs - File watcher
```

## Recommended Implementation Structure

```
codelet/napi/src/credentials/
├── mod.rs              # Module exports, CREDENTIAL_STORE singleton
├── store.rs            # CredentialStore struct with mtime caching
├── types.rs            # CredentialsFile, ProviderCredential types
├── resolver.rs         # Priority chain resolution logic
├── napi_bindings.rs    # credentials_resolve, credentials_reload
└── tests.rs            # Unit tests
```

## Key Implementation Notes

1. **Path:** `{data_dir}/credentials/credentials.json` (matches TypeScript path)
2. **Mtime caching:** Use `std::fs::metadata().modified()` to detect changes
3. **Priority chain:** credentials file → env vars → project .env file
4. **Thread safety:** Use Mutex<Option<CredentialStore>> pattern
5. **NAPI exports:** Only export resolve/reload, never return actual keys
