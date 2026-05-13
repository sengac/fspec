@done
@infrastructure
@rust
@tui
@rpc
@RPC-008
Feature: Compositor priority dispatcher

  Layered priority dispatcher (~30 LoC core) per RPC-002 doc 09 §A.7 +
  §D.1. Stable priority sort with FIFO tiebreak (newer registrations
  win), short-circuit on Consumed, skip on is_active() == false,
  deferred callbacks for self-removal, bottom-up render order, and
  top-down Action fan-out.

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want the Compositor to handle priority dispatch, FIFO tiebreak, deferred callbacks, bottom-up render, and top-down action fan-out exactly as RPC-002 doc 09 specifies
    So that every Component layer (HelloComponent, HelpDialog, future widgets) inherits a single tested dispatch contract

  Scenario: Higher priority intercepts events first
    Given a Compositor with a Background-priority HelloComponent and a Critical-priority HelpDialog pushed
    When a synthetic key event is dispatched via compositor.handle_event(&event)
    Then the HelpDialog's handle_event was invoked
    And the HelloComponent's handle_event was NOT invoked

  Scenario: Ignored events propagate to the next handler
    Given a Compositor with two layers: a Critical-priority component returning Ignored(None) and a Background-priority component recording its invocation
    When a key event is dispatched
    Then both layers received the event in priority order
    And the dispatch returned Ignored(None) overall

  Scenario: is_active=false skips a handler without consuming
    Given a Compositor with a Critical-priority component whose is_active() returns false and a Background-priority component returning Consumed(None)
    When a key event is dispatched
    Then the inactive Critical-priority component's handle_event was NOT invoked
    And the Background-priority component's handle_event was invoked
    And the dispatch returned Consumed

  Scenario: FIFO tiebreak at equal priority — newer registrations win
    Given a Compositor with two Medium-priority components A and B pushed in that order
    When a key event is dispatched
    Then B's handle_event was invoked BEFORE A's handle_event
    And iteration short-circuited if B returned Consumed

  Scenario: Callback inside Consumed runs after event handling completes
    Given a Compositor with a layer that returns `Consumed(Some(Box::new(|c| { c.pop(); })))` on a key event
    When the App dispatches the key event and runs the returned callback against the compositor
    Then the layer was popped off the compositor stack
    And the dispatch path itself did not mutate the compositor before the callback ran

  Scenario: A Critical-priority modal pushed on top intercepts subsequent keystrokes
    Given a Compositor with a single Background-priority HelloComponent
    When a Critical-priority HelpDialog is pushed onto the compositor
    And a key event is dispatched
    Then the HelpDialog's handle_event was invoked first
    And iteration short-circuited at the HelpDialog because it returned Consumed

  Scenario: pop() removes the most recently pushed layer regardless of priority
    Given a Compositor with a Background-priority HelloComponent and then a Critical-priority HelpDialog pushed in that order
    When compositor.pop() is invoked
    Then the returned Option contains the HelpDialog
    And the compositor's remaining layer count is 1

  Scenario: remove(id) removes the layer with the matching id
    Given a Compositor with two layers identified as "hello" (Background) and "help" (Critical)
    When compositor.remove("help") is invoked
    Then the returned Option contains the HelpDialog component
    And only the "hello" layer remains in the compositor

  Scenario: Empty compositor returns Ignored from handle_event
    Given a freshly constructed Compositor with zero layers
    When a key event is dispatched
    Then the dispatch returned Ignored(None)
    And no panic or borrow-checker error occurred

  Scenario: All-inactive compositor returns Ignored
    Given a Compositor with three pushed layers, all of which return false from is_active()
    When a key event is dispatched
    Then no layer's handle_event was invoked
    And the dispatch returned Ignored(None)

  Scenario: Render order is bottom-up so highest priority paints last
    Given a Compositor with a Background-priority component drawing 'A' across the buffer and a Critical-priority component drawing 'B' across the buffer
    When compositor.render(area, &mut buf) is invoked
    Then every cell in the buffer contains 'B'
    And no cell contains 'A'

  Scenario: Action propagation in update fans out across all layers top-down
    Given a Compositor with three layers each recording the actions they observe in update()
    When compositor.update(Action::Quit) is invoked
    Then all three layers observed Action::Quit in registration order
    And the call returned None because no layer produced a follow-up Action
