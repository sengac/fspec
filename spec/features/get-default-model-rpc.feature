@BUG-582
@RPC-440
@rpc
@model-selection
Feature: Read fspec's default model over the RPC
  """
  Read-side counterpart of set_default_model so clients (fspec-paseo FSPEC-046) can follow fspec's default model
  instead of overriding it. FspecService gains `async fn get_default_model() -> Option<String>` appended at the END of
  the trait (after the RPC-449 workspace methods, variant index 127) and recorded in
  rust/rpc/tests/fixtures/wire_order_frozen.txt. SessionManagerHandle::get_default_model delegates to
  SessionManager::get_default_model (restored from default-model.json / tui.lastUsedModel); the service impl and the
  embedded and websocket FspecBackend transports forward it. Research: spec/attachments/RPC-440/get-default-model-rpc.md.
  """

  Background: User Story
    As an fspec client such as the Paseo plugin
    I want to read the daemon's default model
    So that I follow fspec's default instead of guessing and overriding it

  Scenario: The default model round-trips over the RPC
    Given a daemon whose default model was set to "anthropic/claude-opus-4-5"
    When a client calls get_default_model
    Then it receives "anthropic/claude-opus-4-5"

  Scenario: No default model returns none
    Given a daemon with no default model set
    When a client calls get_default_model
    Then it receives no model

  Scenario: get_default_model is appended after every existing method
    Given the FspecService methods in declaration order
    When the index of get_default_model is read
    Then get_default_model keeps index 127, after every method declared before it
