@done
@RPC-423
Feature: Session persistence broken by duplicate manifest creation in FspecAgentHooks

  """
  Remove the duplicate manifest creation block (lines 44-92) from FspecAgentHooks::spawn_agent_loop.
  The manifest is already created by SessionManager::create_session_with_id at line 571 before
  spawn_agent_loop is called at line 770. The duplicate save overwrites the correct manifest
  with one containing only the provider_id (e.g., "anthropic") instead of the full provider/model
  string (e.g., "anthropic/claude-sonnet-4"), breaking session resume.
  """

  Background: User Story
    As a Rust TUI user
    I want to create sessions that persist correctly to disk
    So that I can resume them later with full message history and correct model

  Scenario: Session manifest preserves full provider string after creation
    Given a SessionManager with FspecAgentHooks installed
    When I create a session with model "anthropic/claude-sonnet-4"
    Then the persisted manifest must have provider field set to "anthropic/claude-sonnet-4"
    And the manifest must NOT have provider field set to just "anthropic"

  Scenario: FspecAgentHooks does not overwrite the session manifest
    Given a SessionManager with FspecAgentHooks installed
    When I create a session with model "anthropic/claude-sonnet-4"
    Then FspecAgentHooks::spawn_agent_loop must not call save_session
    And the manifest created by SessionManager::create_session_with_id must remain unchanged

  Scenario: Session resume restores the correct model from persisted manifest
    Given a persisted session manifest with provider "anthropic/claude-sonnet-4"
    When I resume that session via SessionManagerHandle::resume_session
    Then the resumed BackgroundSession must have provider_id "anthropic" and model_id "claude-sonnet-4"

  Scenario: Removing duplicate manifest creation does not break agent loop persistence
    Given a SessionManager with FspecAgentHooks installed without duplicate manifest creation
    When I create a session and send a user message through the agent loop
    Then the message must be persisted to the session manifest on disk
    And the manifest must contain the message in its messages list
