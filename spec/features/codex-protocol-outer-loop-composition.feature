@done
@PROV-162
@providers
@completion
Feature: Codex protocol outer loop composition
  """
  Compose the real Codex CompletionModel with the existing CLI stream driver,
  real Read and native armed Done. Internal protocol requests are not outer
  nudges or genuine user inputs. Preserve exact physical billing and IDs.
  Existing BUG-265 genuine-user and automatic-recovery controls remain required.
  """

  Background: User Story
    As a Codex user completing a tool assisted task
    I want bounded protocol continuation composed with normal tool and nudge behavior
    So that native tool effects and completion budgets are not replayed or reset

  Scenario: Typed protocol tools and outer nudges retain separate accounting
    Given a real CLI Codex session has typed protocol output a native Read and an armed Done
    When tool results and one outer continuation nudge complete the task
    Then native call result identities and owned reasoning remain paired across exact physical requests
    And one outer nudge and once only physical billing survive internal protocol follow ups

  Scenario: Refusal settles visibly without protocol or outer continuation
    Given a real CLI Codex session has outer continuation enabled and receives a completed refusal
    When the refusal carries an explicit nonfinal turn marker
    Then exactly one physical response settles visible refusal and its measured billing
    And no protocol follow up outer nudge or executable tool occurs
