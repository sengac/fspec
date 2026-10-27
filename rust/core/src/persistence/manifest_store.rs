/// Session store — handles storing and retrieving session manifests.
///
/// Internally caches every manifest in memory plus a per-project "last
/// active session" pointer used by [`SessionStore::resume_last`]. Files
/// are persisted as pretty-printed JSON at
/// `{data_dir}/sessions/{uuid}.json`.
pub struct SessionStore {
    pub(super) sessions_dir: PathBuf,
    pub(super) leases: HashMap<Uuid, std::sync::Arc<super::manifest_commit::WriterLease>>,
    /// In-memory cache of sessions by ID.
    pub(super) cache: HashMap<Uuid, SessionManifest>,
    /// Track the last active session per project.
    pub(super) last_session: HashMap<PathBuf, Uuid>,
}

impl SessionStore {
    /// Create a new session store.
    ///
    /// Resolves the data directory via [`codelet_common::get_data_dir`]
    /// and ensures `{data_dir}/sessions/` exists.
    pub fn new() -> Result<Self, String> {
        let data_dir = codelet_common::get_data_dir()?;
        let sessions_dir = data_dir.join("sessions");
        fs::create_dir_all(&sessions_dir)
            .map_err(|e| format!("Failed to create sessions dir {sessions_dir:?}: {e}"))?;
        let mut store = Self {
            sessions_dir,
            leases: HashMap::new(),
            cache: HashMap::new(),
            last_session: HashMap::new(),
        };
        store.load_all()?;
        Ok(store)
    }

    /// Load all sessions from disk.
    fn load_all(&mut self) -> Result<(), String> {
        if !self.sessions_dir.exists() {
            debug!(sessions_dir = %self.sessions_dir.display(), "SessionStore::load_all: sessions directory does not exist, nothing to load");
            return Ok(());
        }
        let (sessions, issues) = super::manifest_discovery::discover(&self.sessions_dir)?;
        for issue in issues {
            tracing::warn!(path = %issue.path.display(), category = %issue.category,
                reason = %issue.reason, "Unreadable historical manifest preserved; diagnostics available");
        }
        for session in sessions {
            self.last_session
                .insert(session.project.clone(), session.id);
            self.cache.insert(session.id, session);
        }
        info!(sessions_dir = %self.sessions_dir.display(), loaded_count = self.cache.len(), "SessionStore::load_all: finished loading sessions from disk");
        Ok(())
    }

    /// Create a new session.
    pub fn create(&mut self, name: &str, project: &Path) -> Result<SessionManifest, String> {
        let session = SessionManifest::new(name, project.to_path_buf());
        let session = self.commit(&session).map_err(|error| error.to_string())?;
        self.last_session.insert(project.to_path_buf(), session.id);
        self.cache.insert(session.id, session.clone());
        Ok(session)
    }

    /// Create a new session with provider.
    pub fn create_with_provider(
        &mut self,
        name: &str,
        project: &Path,
        provider: &str,
    ) -> Result<SessionManifest, String> {
        let session = SessionManifest::with_provider(name, project.to_path_buf(), provider);
        info!(session_id = %session.id, provider = %provider, name = %name, "SessionStore::create_with_provider: created new session manifest");
        let session = self.commit(&session).map_err(|error| error.to_string())?;
        self.last_session.insert(project.to_path_buf(), session.id);
        self.cache.insert(session.id, session.clone());
        Ok(session)
    }

    /// Save a session to disk.
    pub fn save(&mut self, session: &SessionManifest) -> Result<(), String> {
        self.commit(session)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    /// Get a session by ID.
    pub fn get(&self, id: Uuid) -> Option<&SessionManifest> {
        self.cache.get(&id)
    }

    /// Get a mutable reference to a session.
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut SessionManifest> {
        self.cache.get_mut(&id)
    }

    /// Load a session by ID (returns owned value).
    pub fn load(&self, id: Uuid) -> Result<SessionManifest, String> {
        let path = self.sessions_dir.join(format!("{id}.json"));
        super::manifest_commit::read_manifest(&path)
            .map_err(|error| format!("Failed to load session manifest {path:?}: {error}"))
    }

    /// Get the last active session for a project.
    pub fn get_last_session(&self, project: &Path) -> Option<&SessionManifest> {
        self.last_session
            .get(project)
            .and_then(|id| self.cache.get(id))
    }

    /// Resume the last session for a project.
    pub fn resume_last(&self, project: &Path) -> Result<SessionManifest, String> {
        self.get_last_session(project)
            .cloned()
            .ok_or_else(|| format!("No session found for project {project:?}"))
    }

    /// List all sessions for a project.
    pub fn list_for_project(&self, project: &Path) -> Vec<&SessionManifest> {
        self.cache
            .values()
            .filter(|s| s.project == project)
            .collect()
    }

    /// List all sessions.
    pub fn list_all(&self) -> Vec<&SessionManifest> {
        self.cache.values().collect()
    }

    /// Delete a session.
    pub fn delete(&mut self, id: Uuid) -> Result<(), String> {
        self.delete_owned(id).map_err(|error| error.to_string())
    }

    /// Rename a session.
    pub fn rename(&mut self, id: Uuid, new_name: &str) -> Result<(), String> {
        self.update(id, |session| {
            session.name = new_name.to_string();
            Ok(())
        })
        .map(|_| ())
        .map_err(|error| {
            format!(
                "Failed to update session manifest {:?}: {error}",
                self.sessions_dir.join(format!("{id}.json"))
            )
        })
    }

    /// Fork a session at a specific message index.
    pub fn fork(
        &mut self,
        source: &SessionManifest,
        at_index: usize,
        name: &str,
    ) -> Result<SessionManifest, String> {
        if at_index >= source.messages.len() {
            return Err(format!(
                "Fork index {} is out of range (session has {} messages)",
                at_index,
                source.messages.len()
            ));
        }
        if let Some(ref compaction) = source.compaction {
            if at_index < compaction.compacted_before_index {
                return Err(format!(
                    "Cannot fork at index {} which is before compaction boundary {}. \
                     Compacted messages cannot be individually accessed. \
                     Fork at index {} or later.",
                    at_index, compaction.compacted_before_index, compaction.compacted_before_index
                ));
            }
        }
        let mut new_session = if source.provider.is_empty() {
            SessionManifest::new(name, source.project.clone())
        } else {
            SessionManifest::with_provider(name, source.project.clone(), &source.provider)
        };
        new_session.forked_from = Some(ForkPoint {
            source_session_id: source.id,
            fork_after_index: at_index,
            forked_at: Utc::now(),
        });
        for (i, msg_ref) in source.messages.iter().enumerate() {
            if i > at_index {
                break;
            }
            new_session.messages.push(MessageRef {
                message_id: msg_ref.message_id,
                source: MessageSource::Forked {
                    from_session: source.id,
                },
            });
        }
        if let Some(ref compaction) = source.compaction {
            if at_index >= compaction.compacted_before_index {
                new_session.compaction = Some(compaction.clone());
            }
        }
        new_session.provider_state = super::provider_state::fork::publish(source, &new_session)
            .map_err(|error| error.to_string())?;
        let new_session = self
            .commit(&new_session)
            .map_err(|error| error.to_string())?;
        self.cache.insert(new_session.id, new_session.clone());
        Ok(new_session)
    }
}
