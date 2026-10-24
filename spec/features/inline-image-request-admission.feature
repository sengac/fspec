@testing
@providers
@context-management
@phase-1
Feature: Inline images participate in request admission
  """
  Responses input_image parts with inline image data use the upstream Codex
  conservative reservation of 10000 estimated tokens per image. Their encoded
  payload is not text occupancy. All framing, instructions, schemas and output
  reservations remain in the budget, and the selected wire bytes stay unchanged.
  Selected preparation and every physical Codex send share this accounting.
  Unsupported media and remote images without known occupancy still fail closed.
  """

  Scenario: Inline image payload size does not become text occupancy
    Given a Responses request containing an inline image and fixed context
    When the encoded payload grows without changing the request framing
    Then estimated occupancy stays unchanged and includes the image reservation

  Scenario: Inline image reservations can exhaust the input budget
    Given a Responses request containing two inline images
    When only one image reservation fits alongside text and reserved output
    Then admission rejects the request without changing its bytes

  Scenario: Unknown media remains unavailable for admission
    Given requests containing remote images or unsupported media
    When admission inspects the complete requests
    Then each request returns the typed unavailable-media failure

  Scenario: Selected inline image preparation preserves schemas and generation
    Given a Codex request containing inline image data and a tool schema
    When the next generation is prepared within the complete request budget
    Then preparation retains the exact payload and schema without changing the checkpoint

  Scenario: Physical Codex image sends use the selected image estimate
    Given a Codex request containing inline image data that fits its estimated budget
    When streaming and nonstreaming transports submit the request
    Then both sends preserve the inline image without an admission failure
