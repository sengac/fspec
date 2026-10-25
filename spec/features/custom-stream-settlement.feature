@done
@BUG-266
@providers
@provider-abstraction
Feature: Custom stream settlement
  """
  Script stop is a content boundary, not a transport success heuristic.
  Preserve usage through authoritative [DONE] or clean EOF after stop.
  Keep BUG-227 typed post-stop failure behavior; no timeout settles a request.
  The real rig adapter must settle usage before releasing executable calls.
  Synthetic local HTTP fixtures exercise the real script and request path.
  The approved mechanical stream_types extraction preserves public paths.
  """

  Background: User Story
    As a user of a scripted streaming provider
    I want trailing usage preserved at an honest completion boundary
    So that completion never hides physical usage or transport failure

  Scenario: DONE settles trailing usage without waiting for EOF
    Given a local custom stream emits stop then usage then DONE and stays open
    When the real scripted model consumes the response
    Then it settles promptly with exact input and output usage

  Scenario: Clean EOF after stop retains trailing usage
    Given a local custom stream emits stop then usage and clean EOF
    When the real scripted model consumes the response
    Then it settles with exact input and output usage

  Scenario: Post-stop failure cannot release executable tools
    Given a local custom stream queues a tool then stop and a transport failure
    When the real scripted model consumes the response
    Then one error survives without successful final or executable calls

  Scenario: Missing terminal evidence remains a failure
    Given a local custom stream ends with text but no stop or DONE
    When the real scripted model consumes the response
    Then one error survives without successful final or executable calls

  Scenario: Stop alone does not invent transport success
    Given a local custom stream emits text and stop then stays open
    When the consumer waits for settlement and then cancels
    Then no successful final appears and cancellation closes the connection

  Scenario: Post-stop content cannot extend the completed response
    Given a local custom stream emits stop then extra text tools and usage before DONE
    When the real scripted model consumes the response
    Then trailing usage is preserved without extra text or executable calls
