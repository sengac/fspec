@done
@parity
@websocket
@tarpc
@rust
@infrastructure
@tui
@rpc
@RPC-008
Feature: App shell + run loop (RPC-008)

  The ratatui App struct + run loop. App::new(backend) pre-populates a
  Compositor with a Background-priority HelloComponent. App-level `?`
  pushes the HelpDialog onto the compositor; ESC removes it via the
  deferred-callback pattern; `q` flips should_quit and the
  tokio::select! loop unwinds. App-with-mock-backend snapshot
  captures HelpDialog visible after `?` and gone after ESC.

  Background: User Story
    As a fspec developer building the ratatui frontend
    I want App::new(backend) to pre-populate the compositor with a Background HelloComponent, intercept `?` to push HelpDialog at Critical, run the deferred-callback dismiss on ESC, and flip should_quit on `q`
    So that the placeholder app-shell built on top of the FspecBackend trait is observably wired end-to-end before RPC-009 swaps in the real list view + REPL

  Scenario: App::new pre-populates the compositor with a Background-priority HelloComponent
    Given a mock backend implementing `dyn FspecBackend`
    When `App::new(Arc::new(mock))` is constructed
    Then the App's compositor contains exactly one layer
    And that layer's id() returns "hello"
    And that layer's priority() returns Priority::Background

  Scenario: '?' at App level pushes the HelpDialog onto the compositor
    Given an App constructed against a MockBackend
    And the App's compositor contains exactly the HelloComponent
    When the App receives a synthetic Key('?') event
    Then the compositor contains exactly two layers
    And the topmost layer's priority() returns Priority::Critical
    And the topmost layer's id() returns "help-dialog"

  Scenario: ESC while the HelpDialog is on top removes the dialog via deferred callback
    Given an App with a HelpDialog pushed onto the compositor at Priority::Critical
    When the App receives a synthetic Key(Esc) event
    Then the dialog's handle_event returned `Consumed(Some(callback))`
    And the App ran the callback against the compositor after dispatch completed
    And the compositor now contains only the HelloComponent

  Scenario: 'q' at App level sets should_quit and the run loop exits
    Given an App with a MockBackend and the run loop driven by a TestBackend terminal
    When the App receives a synthetic Key('q') event
    Then the App's should_quit flag is true
    And the next iteration of the tokio::select! breaks the loop
    And `App::run().await` returns Ok(())

  Scenario: App-with-mock-backend snapshot captures HelpDialog visible after '?' and gone after ESC
    Given an App constructed against a MockBackend on an 80x24 TestBackend terminal
    When the App processes a synthetic Key('?') event and renders one frame
    Then the rendered buffer matches the insta snapshot "help_dialog_visible"
    When the App processes a synthetic Key(Esc) event and renders one frame
    Then the rendered buffer matches the insta snapshot "help_dialog_dismissed"
