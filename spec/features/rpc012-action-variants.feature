@done
@RPC-012
@rust
@tui
@infrastructure
@rpc
Feature: RPC-012 Action enum — navigator slice variants
  """
  RPC-012 — The Action enum in codelet/fspec-tui/src/components/mod.rs
  is extended with the variants the navigator slice needs. Existing
  RPC-009 variants (Quit, Redraw, Custom, LoadWorkUnits, WorkUnitsLoaded,
  SessionCreated, ChunkReceived, InputSubmitted, Interrupt, FocusNext) and
  RPC-011 variants (Disconnected, Reconnecting, Reconnected,
  ManualReconnect) are preserved.
  """

  Background: User Story
    As a Rust fspec frontend developer
    I want the Action enum to carry the navigator-slice variants (EnterWorkUnit, OpenAgentView, BackToBoard, NavigationTargetSet, AttachSession)
    So that BoardView, AgentView, and App::dispatch share one fanout enum without re-inventing per-view enums

  Scenario: Action enum gains four new variants for the navigator slice
    Given the Action enum in codelet/fspec-tui/src/components/mod.rs
    Then it contains the variant EnterWorkUnit(String)
    And it contains the variant OpenAgentView(Option<SessionId>)
    And it contains the variant BackToBoard
    And it contains the variant NavigationTargetSet(Option<SessionId>)
    And it contains the variant AttachSession(String, SessionId)
