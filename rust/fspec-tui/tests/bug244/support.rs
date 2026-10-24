use codelet_core::{persistence, work_units::WorkUnitsWatcher};
use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::SessionId;
use codelet_sessions::SessionManager;
use std::sync::Arc;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub manager: Arc<SessionManager>,
    pub id: SessionId,
    previous_key: Option<std::ffi::OsString>,
    previous_user: Option<std::ffi::OsString>,
}

impl Fixture {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let previous_key = std::env::var_os("ANTHROPIC_API_KEY");
        let previous_user = std::env::var_os("FSPEC_USER_DIR");
        std::env::set_var("ANTHROPIC_API_KEY", "test-transcript");
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        codelet_common::set_data_directory(directory.path().into())?;
        persistence::reset_stores_for_tests();
        std::fs::create_dir_all(directory.path().join("cache"))?;
        std::fs::write(
            directory.path().join("cache/models.json"),
            include_str!("../../../sessions/tests/fixtures/prov123_models.json"),
        )?;
        std::fs::create_dir_all(directory.path().join("spec"))?;
        std::fs::write(
            directory.path().join("spec/work-units.json"),
            "{\"workUnits\":{}}",
        )?;
        let manager = Arc::new(SessionManager::new());
        let id = SessionId::new(
            manager
                .create_session(
                    "anthropic/claude-sonnet-4",
                    &directory.path().to_string_lossy(),
                )
                .await?,
        );
        Ok(Self {
            directory,
            manager,
            id,
            previous_key,
            previous_user,
        })
    }

    pub async fn backends(
        &self,
    ) -> Result<
        (
            Arc<dyn FspecBackend>,
            Arc<dyn FspecBackend>,
            tokio::task::JoinHandle<()>,
        ),
        Box<dyn std::error::Error>,
    > {
        let watcher = Arc::new(WorkUnitsWatcher::new(self.directory.path())?);
        let service = Arc::new(
            SharedFspecService::with_session_manager(watcher, self.manager.clone())
                .with_cwd(self.directory.path().into()),
        );
        let embedded = Arc::new(EmbeddedFspecBackend::new(
            tokio::runtime::Handle::current(),
            service.clone(),
        ));
        let (address, _, server) =
            codelet_rpc_server::bind_and_serve("127.0.0.1:0", service).await?;
        let websocket = Arc::new(
            WebSocketFspecBackend::connect(url::Url::parse(&format!("ws://{address}/"))?).await?,
        );
        Ok((embedded, websocket, server))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for (name, previous) in [
            ("ANTHROPIC_API_KEY", &self.previous_key),
            ("FSPEC_USER_DIR", &self.previous_user),
        ] {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        persistence::reset_stores_for_tests();
    }
}
