@done
@parity
@infrastructure
@rust
@tui
@rpc
@RPC-008
Feature: FspecBackend trait surface + transport-agnostic consumer

  Source-shape regressions for the public FspecBackend trait declaration
  in codelet/fspec-tui/src/transport/mod.rs plus the compile-time
  guarantee that a single Arc<dyn FspecBackend> consumer compiles
  against both backend implementations. Includes the Priority enum
  discriminant + Component trait default-method invariants.

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want the FspecBackend trait + Component trait + Priority enum to expose exactly the documented surface (5 RPC methods, 3 broadcast subscriptions, repr(u32) discriminants, Send-only Component bound)
    So that RPC-009/RPC-010 consumers can hold Arc<dyn FspecBackend> safely and the cross-crate type contract is enforced at compile + source-shape time

  Scenario: FspecBackend trait surface exposes 5 RPC methods + 3 broadcast subscriptions
    Given codelet/fspec-tui/src/transport/mod.rs exists
    When I inspect the public trait declaration
    Then a public async_trait FspecBackend with bounds Send + Sync is defined
    And the trait declares async fn list_work_units returning Result<Vec<WorkUnitInfo>>
    And the trait declares async fn list_sessions returning Result<Vec<SessionInfo>>
    And the trait declares async fn create_session(role: Option<String>) returning Result<SessionId>
    And the trait declares async fn send_input(id: SessionId, text: String) returning Result<()>
    And the trait declares async fn interrupt(id: SessionId) returning Result<()>
    And the trait declares fn work_units_rx returning broadcast::Receiver<Vec<WorkUnitInfo>>
    And the trait declares fn chunks_rx returning broadcast::Receiver<(SessionId, StreamChunk)>
    And the trait declares fn logs_rx returning broadcast::Receiver<LogRecord>

  Scenario: A transport-agnostic consumer compiles against either backend implementation
    Given a Rust test consumer that asks for `Arc<dyn FspecBackend>`
    When the consumer is constructed once with `Arc::new(EmbeddedFspecBackend::new(handle, service))`
    And the consumer is constructed again with `Arc::new(WebSocketFspecBackend::connect(url).await?)`
    Then the consumer body invoking `backend.list_work_units().await` and `backend.work_units_rx()` compiles unchanged in both arms
    And the only difference between the two arms is the constructor expression

  Scenario: Priority enum has the exact #[repr(u32)] discriminants from RPC-002 doc 09
    Given the Priority enum is defined in codelet/fspec-tui/src/components/mod.rs
    When I cast each variant to u32
    Then Priority::Background as u32 equals 100
    And Priority::Low as u32 equals 200
    And Priority::Medium as u32 equals 500
    And Priority::High as u32 equals 800
    And Priority::Critical as u32 equals 1000

  Scenario: Component trait surface exposes priority is_active id handle_event update and render with documented defaults
    Given codelet/fspec-tui/src/components/mod.rs declares the Component trait
    When I inspect the trait declaration
    Then the trait is bounded Send (no Sync)
    And fn priority(&self) -> Priority has a default body returning Priority::Medium
    And fn is_active(&self) -> bool has a default body returning true
    And fn id(&self) -> &str has no default and must be implemented
    And fn handle_event(&mut self, event: &Event) -> EventResult has a default body returning EventResult::Ignored(None)
    And fn update(&mut self, action: Action) -> Option<Action> has a default body returning None
    And fn render(&mut self, area: Rect, buf: &mut Buffer) has no default and must be implemented
