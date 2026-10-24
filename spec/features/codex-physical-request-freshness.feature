@done
@CMPCT-078
@CMPCT-080
@providers
@provider-abstraction
@compaction
@phase-1
Feature: Codex physical requests retain their prepared generation authority
  """
  Creating a lazy stream is not permission to send it after checkpoint failure
  or durable generation replacement. The prepared body remains bound to its
  original generation until physical send and checkpoint publication. Stale
  streams fail explicitly without transport fallback or completion publication.
  Nonstreaming completions retain the same authority while awaiting a response;
  they cannot relabel an old response with a newer committed generation.
  """

  Scenario Outline: Invalidated lazy streams cannot send a previously prepared request
    Given a Codex stream prepared against the committed checkpoint but not yet polled
    When checkpoint authority changes through <invalidation> before the first poll
    Then the prepared stream returns a typed checkpoint failure without a physical request or completion
    And the authoritative checkpoint remains unchanged until an authentic fresh request succeeds

    Examples:
      | invalidation           |
      | failed restore         |
      | generation replacement |

  Scenario: Replaced generations cannot receive an old physical response checkpoint
    Given a Codex physical response has started for the committed generation
    When a newer durable generation is installed before the response completes
    Then the old stream fails with typed lineage error without publishing completion
    And its response cannot overwrite the new generation checkpoint
