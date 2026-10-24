@done
@CMPCT-071
@providers
@compaction
@context-management
@phase-1
Feature: Selected request admission distinguishes schemas from opaque media
  """
  Native tool definitions describe possible tool arguments, not media supplied
  to inference. Admission retains and counts every serialized schema byte while
  inspecting actual request content for media with unknown occupancy. A schema
  must neither create a false media rejection nor hide real request media.
  """

  Scenario: Media vocabulary in native tool schemas does not prevent text admission
    Given selected native adapters with complete tool schemas describing media fields
    When their text requests are prepared for the next committed generation
    Then every schema byte is retained and counted without transmitting a request

  Scenario: Native tool schemas cannot hide real media with unknown occupancy
    Given selected native adapters with tool schemas and actual image content
    When their complete requests are prepared for the next committed generation
    Then typed unknown media rejection leaves checkpoints and physical transport unchanged
