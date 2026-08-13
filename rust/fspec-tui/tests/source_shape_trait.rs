//! Source-shape regression tests for the FspecBackend trait surface
//! and transport-agnostic consumer (RPC-008).
//!
//! Feature: spec/features/fspec-tui-trait-surface.feature
//!
//! Scenarios covered (trait-shape half):
//!   - "FspecBackend trait surface exposes 5 RPC methods + 3 broadcast
//!     subscriptions" — pure source inspection of
//!     `rust/fspec-tui/src/transport/mod.rs`.
//!   - "A transport-agnostic consumer compiles against either backend
//!     implementation" — compile-time assertion via two arms that both
//!     return `Arc<dyn FspecBackend>` and forward to a single
//!     `use_backend` consumer fn.
//!
//! These tests are Cargo-toml-free; the Cargo half lives in
//! `tests/source_shape_cargo.rs` so each file stays under the project's
//! 300-LoC ceiling.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

#[test]
fn fspec_backend_trait_surface_exposes_5_rpc_methods_and_3_broadcast_subscriptions() {
    // @step Given rust/fspec-tui/src/transport/mod.rs exists
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("transport")
        .join("mod.rs");
    let raw = common::read_to_string_or_panic(&path);
    let src = common::strip_rust_comments(&raw);

    // @step When I inspect the public trait declaration
    // @step Then a public async_trait FspecBackend with bounds Send + Sync is defined
    assert!(
        src.contains("#[async_trait]"),
        "transport/mod.rs must annotate FspecBackend with #[async_trait]"
    );
    assert!(
        src.contains("pub trait FspecBackend: Send + Sync"),
        "transport/mod.rs must declare `pub trait FspecBackend: Send + Sync`"
    );

    // @step And the trait declares async fn list_work_units returning Result<Vec<WorkUnitInfo>>
    assert!(
        src.contains("async fn list_work_units(&self) -> Result<Vec<WorkUnitInfo>>"),
        "FspecBackend must declare list_work_units returning Result<Vec<WorkUnitInfo>>"
    );

    // @step And the trait declares async fn list_sessions(project_path: String) returning Result<Vec<SessionInfo>>
    assert!(
        src.contains("async fn list_sessions(&self, project_path: String) -> Result<Vec<SessionInfo>>"),
        "FspecBackend must declare list_sessions(project_path: String) returning Result<Vec<SessionInfo>>"
    );

    // @step And the trait declares async fn create_session(role: Option<String>) returning Result<SessionId>
    assert!(
        src.contains("async fn create_session(&self, role: Option<String>) -> Result<SessionId>"),
        "FspecBackend must declare create_session(role: Option<String>) returning Result<SessionId>"
    );

    // @step And the trait declares async fn send_input(id: SessionId, text: String) returning Result<()>
    assert!(
        src.contains("async fn send_input(&self, id: SessionId, text: String) -> Result<()>"),
        "FspecBackend must declare send_input(id: SessionId, text: String) returning Result<()>"
    );

    // @step And the trait declares async fn interrupt(id: SessionId) returning Result<()>
    assert!(
        src.contains("async fn interrupt(&self, id: SessionId) -> Result<()>"),
        "FspecBackend must declare interrupt(id: SessionId) returning Result<()>"
    );

    // @step And the trait declares fn work_units_rx returning broadcast::Receiver<Vec<WorkUnitInfo>>
    assert!(
        src.contains("fn work_units_rx(&self) -> broadcast::Receiver<Vec<WorkUnitInfo>>"),
        "FspecBackend must declare work_units_rx returning broadcast::Receiver<Vec<WorkUnitInfo>>"
    );

    // @step And the trait declares fn chunks_rx returning broadcast::Receiver<(SessionId, StreamChunk)>
    assert!(
        src.contains("fn chunks_rx(&self) -> broadcast::Receiver<(SessionId, StreamChunk)>"),
        "FspecBackend must declare chunks_rx returning broadcast::Receiver<(SessionId, StreamChunk)>"
    );

    // @step And the trait declares fn logs_rx returning broadcast::Receiver<LogRecord>
    assert!(
        src.contains("fn logs_rx(&self) -> broadcast::Receiver<LogRecord>"),
        "FspecBackend must declare logs_rx returning broadcast::Receiver<LogRecord>"
    );
}

#[test]
fn transport_agnostic_consumer_compiles_against_either_backend_implementation() {
    use std::sync::Arc;

    use codelet_fspec_tui::{EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
    use codelet_rpc_types::WorkUnitInfo;

    // @step Given a Rust test consumer that asks for `Arc<dyn FspecBackend>`
    async fn use_backend(backend: Arc<dyn FspecBackend>) -> anyhow::Result<Vec<WorkUnitInfo>> {
        let units = backend.list_work_units().await?;
        let _ = backend.work_units_rx();
        Ok(units)
    }

    // @step When the consumer is constructed once with `Arc::new(EmbeddedFspecBackend::new(handle, service))`
    // @step And the consumer is constructed again with `Arc::new(WebSocketFspecBackend::connect(url).await?)`
    // (Type-check only — these inner async fns are never invoked at runtime.)
    #[allow(dead_code)]
    async fn embedded_arm(
        handle: tokio::runtime::Handle,
        service: Arc<codelet_rpc::SharedFspecService>,
    ) -> anyhow::Result<Vec<WorkUnitInfo>> {
        let backend: Arc<dyn FspecBackend> = Arc::new(EmbeddedFspecBackend::new(handle, service));
        use_backend(backend).await
    }

    #[allow(dead_code)]
    async fn websocket_arm(url: url::Url) -> anyhow::Result<Vec<WorkUnitInfo>> {
        let backend: Arc<dyn FspecBackend> = Arc::new(WebSocketFspecBackend::connect(url).await?);
        use_backend(backend).await
    }

    // @step Then the consumer body invoking `backend.list_work_units().await` and `backend.work_units_rx()` compiles unchanged in both arms
    // @step And the only difference between the two arms is the constructor expression
    // (Reaching this line proves the compiler accepted both arms; the
    // arms differ ONLY in `Arc::new(...)` constructor expression.)
}

/// Scenario: Priority enum has the exact #[repr(u32)] discriminants
/// from RPC-002 doc 09
#[test]
fn priority_enum_has_the_exact_repr_u32_discriminants_from_rpc_002_doc_09() {
    use codelet_fspec_tui::Priority;

    // @step Given the Priority enum is defined in rust/fspec-tui/src/components/mod.rs
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("components")
        .join("mod.rs");
    let src = common::strip_rust_comments(&common::read_to_string_or_panic(&path));
    assert!(
        src.contains("#[repr(u32)]"),
        "Priority enum must be #[repr(u32)] (RPC-002 doc 09 §A.1)"
    );

    // @step When I cast each variant to u32
    // @step Then Priority::Background as u32 equals 100
    assert_eq!(Priority::Background as u32, 100);

    // @step And Priority::Low as u32 equals 200
    assert_eq!(Priority::Low as u32, 200);

    // @step And Priority::Medium as u32 equals 500
    assert_eq!(Priority::Medium as u32, 500);

    // @step And Priority::High as u32 equals 800
    assert_eq!(Priority::High as u32, 800);

    // @step And Priority::Critical as u32 equals 1000
    assert_eq!(Priority::Critical as u32, 1000);
}

/// Scenario: Component trait surface exposes priority is_active id
/// handle_event update and render with documented defaults
#[test]
fn component_trait_surface_exposes_documented_methods_and_defaults() {
    use codelet_fspec_tui::{Action, Component, EventResult, Priority};
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    // @step Given rust/fspec-tui/src/components/mod.rs declares the Component trait
    let path = common::workspace_root()
        .join("fspec-tui")
        .join("src")
        .join("components")
        .join("mod.rs");
    let src = common::strip_rust_comments(&common::read_to_string_or_panic(&path));

    // @step When I inspect the trait declaration
    // @step Then the trait is bounded Send (no Sync)
    assert!(
        src.contains("pub trait Component: Send {"),
        "Component trait must be `Send` (no Sync). Source did not match."
    );
    assert!(
        !src.contains("pub trait Component: Send + Sync"),
        "Component trait MUST NOT bound Sync"
    );

    // The remaining defaults are runtime-asserted via a minimal stub
    // that overrides ONLY the no-default methods (id + render). The
    // trait's defaults must drive every other method's return value.
    struct Stub;
    impl Component for Stub {
        fn id(&self) -> &str {
            "stub"
        }
        fn render(&mut self, _area: Rect, _buf: &mut Buffer) {}
    }
    let mut stub = Stub;

    // @step And fn priority(&self) -> Priority has a default body returning Priority::Medium
    assert_eq!(stub.priority(), Priority::Medium);

    // @step And fn is_active(&self) -> bool has a default body returning true
    assert!(stub.is_active());

    // @step And fn id(&self) -> &str has no default and must be implemented
    assert_eq!(stub.id(), "stub");

    // @step And fn handle_event(&mut self, event: &Event) -> EventResult has a default body returning EventResult::Ignored(None)
    let key = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    let result = stub.handle_event(&key);
    assert!(matches!(result, EventResult::Ignored(None)));

    // @step And fn update(&mut self, action: Action) -> Option<Action> has a default body returning None
    assert!(stub.update(Action::Quit).is_none());

    // @step And fn render(&mut self, area: Rect, buf: &mut Buffer) has no default and must be implemented
    let area = Rect::new(0, 0, 1, 1);
    let mut buf = Buffer::empty(area);
    stub.render(area, &mut buf);
}
