@watcher
@codelet
@WATCH-003
Feature: Broadcast Channel for Parent Stream Observation

  """
  Add watcher_broadcast: broadcast::Sender<StreamChunk> field to BackgroundSession struct
  Initialize broadcast channel in BackgroundSession::new() with capacity 256
  Add subscribe_to_stream() method returning broadcast::Receiver<StreamChunk>
  Modify handle_output() to call watcher_broadcast.send(chunk.clone()) after buffering
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. BackgroundSession has a broadcast::Sender<StreamChunk> field that enables multiple receivers
  #   2. handle_output() broadcasts chunks to all subscribed watchers in addition to buffering and UI callback
  #   3. Broadcast channel capacity is bounded (e.g., 256 chunks) - slow watchers may miss chunks via lagging
  #   4. subscribe_to_stream() returns a broadcast::Receiver<StreamChunk> for watchers to receive live updates
  #   5. Receivers can be created at any time - late subscribers start receiving from current stream position
  #   6. StreamChunk must implement Clone to be broadcast (already does)
  #
  # EXAMPLES:
  #   1. Parent session sends TextDelta chunk → broadcast sends to 0 watchers (no receivers subscribed) → chunk still buffered normally
  #   2. Watcher calls subscribe_to_stream(parent_id) → receives broadcast::Receiver → parent sends chunk → watcher receives same chunk via receiver
  #   3. Two watchers subscribe to same parent → parent sends chunk → both watchers receive identical chunk independently
  #   4. Watcher processes slowly and falls 256+ chunks behind → receives RecvError::Lagged(n) indicating n missed chunks
  #   5. Watcher drops receiver (disconnects) → broadcast sender continues working for other watchers → no impact on parent session
  #   6. Session with no broadcast subscribers → handle_output() still buffers and calls UI callback normally (broadcast is fire-and-forget)
  #
  # ========================================

  Background: User Story
    As a watcher session
    I want to receive real-time stream chunks from my parent session via a broadcast channel
    So that I can observe and analyze the parent's conversation as it happens

  @unit
  Scenario: Broadcast with no subscribers still buffers normally
    Given a BackgroundSession with broadcast channel initialized
    And no watchers have subscribed to the stream
    When handle_output is called with a TextDelta chunk
    Then the chunk should be added to the output buffer
    And no error should occur from the broadcast

  @unit
  Scenario: Single watcher receives chunks via broadcast
    Given a BackgroundSession with broadcast channel initialized
    And a watcher has called subscribe_to_stream to get a receiver
    When handle_output is called with a TextDelta chunk
    Then the watcher should receive the same chunk via its receiver
    And the chunk should also be buffered normally

  @unit
  Scenario: Multiple watchers receive chunks independently
    Given a BackgroundSession with broadcast channel initialized
    And watcher A has subscribed to the stream
    And watcher B has subscribed to the stream
    When handle_output is called with a TextDelta chunk
    Then watcher A should receive the chunk via its receiver
    And watcher B should receive the chunk via its receiver
    And both received chunks should be identical

  @unit
  Scenario: Slow watcher receives lagged error when falling behind
    Given a BackgroundSession with broadcast channel capacity of 256
    And a watcher has subscribed to the stream
    And the watcher has not consumed any chunks
    When handle_output is called 300 times with chunks
    Then the watcher should receive RecvError::Lagged when trying to receive

  @unit
  Scenario: Dropped receiver does not affect other watchers
    Given a BackgroundSession with broadcast channel initialized
    And watcher A has subscribed to the stream
    And watcher B has subscribed to the stream
    When watcher A drops its receiver
    And handle_output is called with a TextDelta chunk
    Then watcher B should still receive the chunk normally
    And the parent session should continue operating normally

  @unit
  Scenario: Late subscriber starts receiving from current position
    Given a BackgroundSession with broadcast channel initialized
    And handle_output has been called 10 times with chunks
    When a new watcher subscribes to the stream
    And handle_output is called with a new chunk
    Then the new watcher should receive only the new chunk
    And the new watcher should not receive the previous 10 chunks
