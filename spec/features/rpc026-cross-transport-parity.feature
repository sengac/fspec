@done
@multi-session
@rpc
@history-search
@session-resume
@agent-view
@tui
@RPC-026
Feature: Cross-transport parity for persistence_delete_session (RPC-021c)
  """
  Lifting delete_session: codelet/core/src/persistence/sessions.rs is a new file < 100 LoC that imports the SAME `~/.fspec/sessions.jsonl` reader/writer logic the existing NAPI module uses. It re-exports `delete_session(uuid)`, `list_sessions()` (for the FspecServiceImpl::list_sessions to also use), and any helper needed. codelet/napi/src/persistence/mod.rs::delete_session becomes a one-line delegate to codelet_core::persistence::sessions::delete_session — preserving the byte-identical TS surface. NO #[napi] export signatures change.
  Both mode views MUST work against EmbeddedFspecBackend AND WebSocketFspecBackend (RPC-002 invariant). Cross-transport-parity tests live in tests/rpc026-*.rs and drive the SAME scripted scenarios against both transports.
  """

  # See spec/features/rpc026-* for the broader RPC-026 example-mapping context.
  # This file covers the lifted codelet_core::persistence::delete_session and the
  # cross-transport round-trip parity assertions.
  Background: User Story
    As a developer using the Rust ratatui TUI
    I want to press /resume or /search (and Ctrl+R) to open full-screen mode views that mirror the TypeScript Ink TUI — listing resumable sessions or filtering submitted-input history with delete confirmation — rather than small floating popups
    So that the Rust frontend's `/resume` and `/search` UX matches the existing TypeScript frontend pixel-for-pixel and feature-for-feature, so habits and integration tests carry across implementations unchanged

  @core
  @persistence
  @lift
  Scenario: codelet_core::persistence::delete_session lifts the NAPI implementation
    Given a fresh ~/.fspec/sessions.jsonl with sessions ["s-1", "s-2", "s-3"]
    When codelet_core::persistence::delete_session(Uuid("s-2")) is called
    Then the on-disk sessions.jsonl no longer lists "s-2"
    And codelet_core::persistence::sessions::list() returns ["s-1", "s-3"]
    And the NAPI export persistence_delete_session from codelet/napi/src/persistence/napi_bindings.rs is a one-line delegate to codelet_core::persistence::delete_session

  @rpc
  @cross-transport
  Scenario: persistence_delete_session round-trips identically across both transports
    Given a SharedFspecService with sessions ["s-1", "s-2", "s-3"]
    When the test calls EmbeddedFspecBackend.persistence_delete_session("s-2")
    And then calls EmbeddedFspecBackend.list_sessions()
    Then the result equals ["s-1", "s-3"]
    Given another SharedFspecService with sessions ["s-1", "s-2", "s-3"]
    When the test calls WebSocketFspecBackend.persistence_delete_session("s-2")
    And then calls WebSocketFspecBackend.list_sessions()
    Then the result equals ["s-1", "s-3"]
    And both transports produced byte-identical SessionInfo lists
