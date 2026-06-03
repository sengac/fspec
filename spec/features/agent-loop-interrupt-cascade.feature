@done
@session-management
@RPC-088
@rust
@agent-loop
@rpc
@interrupt
Feature: Agent loop honours Esc and emits StreamChunk::Interrupted
  """
  RPC-088 (child of RPC-072 family). Interrupt cascade must consult
  session.is_interrupted and select against
  session.interrupt_notify.notified() inside the stream loop so Esc
  aborts the active provider call and emits StreamChunk::Interrupted.

  After the RPC-072/RPC-080/RPC-081/RPC-082 ports the implementation
  already lives in the NAPI-free `codelet-agent-loop` crate and
  `codelet/cli/src/interactive/stream_loop.rs`. RPC-088 pins the
  contract via structural source-string assertions plus a small
  integration check against a real BackgroundSession, mirroring the
  RPC-082/083/084/086 coverage pattern.
  """

  Background: User Story
    As a fspec user
    I want pressing Esc during an LLM call to actually cancel the call
    So that I can recover from a runaway response without restarting the session

  Scenario: BackgroundSession owns the AtomicBool + Notify interrupt handles
    Given the source of `codelet/sessions/src/background_session.rs`
    When I inspect the `BackgroundSession` struct
    Then it declares a `pub is_interrupted: Arc<AtomicBool>` field
    And it declares a `pub interrupt_notify: Arc<Notify>` field
    And the `new` constructor initialises both fields with `Arc::new(AtomicBool::new(false))` and `Arc::new(Notify::new())`

  Scenario: BackgroundSession::interrupt() flips the flag and wakes notifier
    Given a `codelet_sessions::background_session::BackgroundSession` constructed via test helpers
    When I call `session.interrupt()`
    Then `session.is_interrupted.load(Ordering::Acquire)` returns `true`
    And a tokio task awaiting `session.interrupt_notify.notified()` (registered BEFORE interrupt was called) is woken within 100ms
    And calling `session.reset_interrupt()` flips `session.is_interrupted.load(Ordering::Acquire)` back to `false`

  Scenario: Agent loop calls reset_interrupt() at the start of each turn
    Given the source of `codelet/agent-loop/src/agent_loop.rs`
    When I locate the pre-turn setup block
    Then the body contains `session.reset_interrupt();` immediately after `session.set_status(SessionStatus::Running);`

  Scenario: run_with_provider! macro forwards both interrupt handles to run_agent_stream_with_images
    Given the source of `codelet/agent-loop/src/dispatch.rs`
    When I locate the `run_with_provider!` macro body
    Then the body's call to `codelet_cli::interactive::run_agent_stream_with_images` passes `$session.is_interrupted.clone()` as positional arg 5
    And the call passes `$session.interrupt_notify.clone()` as positional arg 7

  Scenario: OpenAI inlined arm and custom-provider fallthrough forward both interrupt handles
    Given the source of `codelet/agent-loop/src/agent_loop.rs`
    When I locate the inlined `"openai" =>` match arm
    Then the body's `run_agent_stream_with_images` call passes `session.is_interrupted.clone()` and `session.interrupt_notify.clone()`
    When I locate the `_ =>` custom-provider fallthrough arm
    Then the body's `run_agent_stream_with_images` call also passes `session.is_interrupted.clone()` and `session.interrupt_notify.clone()`

  Scenario: BackgroundOutput translates StreamEvent::Interrupted into StreamChunk::interrupted
    Given the source of `codelet/agent-loop/src/background_output.rs`
    When I locate the `StreamEvent::Interrupted(queued)` arm of `BackgroundOutput::emit`
    Then the arm body calls `self.persist_assistant_message()`
    And the arm body returns `StreamChunk::interrupted(queued)`

  Scenario: codelet_rpc_types::StreamChunk declares Interrupted variant + constructor
    Given the source of `codelet/rpc-types/src/lib.rs`
    When I inspect the `StreamChunk` enum
    Then the enum declares a `Interrupted { queued_inputs: Vec<String> }` variant
    And the impl block defines a `pub fn interrupted(queued_inputs: Vec<String>) -> Self` constructor returning `Self::Interrupted { queued_inputs }`
