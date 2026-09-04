@done
@PROV-144
@rust
@tools
@session
@multimodal
@high
Feature: Per-profile Max Images Read tool budget enforcement
  """
  The Read tool (rust/tools/src/read.rs, via the new read_image_budget module)
  enforces the per-session image budget at call time — the single source of
  truth consulted by BOTH front doors because FileToolFacadeWrapper delegates
  to ReadTool::call. The budget is resolved from the session capability
  registry (codelet_tools::model_capabilities, sibling max-images registry):
  registry entry absent (non-profile / unregistered session) => default 4;
  budget 0 (no-vision profile) => any read that would return an image fails
  with the no-vision message; budget n >= 1 => a single image file always
  passes, a PDF read that would return more pages/embedded images than n
  FAILS the tool call with a message naming the limit, the requested count,
  and how to read fewer (offset/limit) or raise the profile's Max Images
  setting. Clamping is not acceptable (explicit PO ruling).
  """

  Background: User Story
    As a user configuring an OpenAI API profile
    I want the Read tool to enforce my profile's Max Images limit
    So that no-vision profiles fail image reads clearly and image reads never exceed the budget

  # ========================================
  # Read tool enforcement: no-vision (0)
  # ========================================

  Scenario: A no-vision profile fails image-file reads with a clear message
    Given a session against a profile with maxImages 0
    When the Read tool is called on a PNG image file
    Then the tool call fails with a message stating image reading is disabled because the profile is configured with Max Images 0 (no vision)
    And the message points at text-based alternatives
    When the Read tool is called on a text file
    Then the call succeeds and returns the text content

  Scenario: A no-vision profile forces PDF reads to text mode
    Given a session against a profile with maxImages 0
    When the Read tool is called on a PDF with no pdf_mode specified
    Then text mode is used automatically
    And no page images are returned

  # ========================================
  # Read tool enforcement: over-budget (n >= 1)
  # ========================================

  Scenario: A PDF read exceeding the image budget fails with the limit message
    Given a session against a profile with maxImages 2
    And a PDF file with 10 pages
    When the Read tool is called on the PDF in visual mode with no limit
    Then the tool call fails with a message naming the limit 2, the requested page count, and how to read fewer with offset/limit
    When the Read tool is retried on the PDF with limit 2
    Then the call succeeds and returns exactly 2 page images

  Scenario: A PDF read within the image budget succeeds
    Given a session against a profile with maxImages 4
    And a PDF file with 3 pages
    When the Read tool is called on the PDF in visual mode
    Then the call succeeds and returns 3 page images
    When the Read tool is called on a single PNG image file
    Then the call succeeds and returns 1 image

  Scenario: A PDF images-mode read exceeding the budget fails
    Given a session against a profile with maxImages 1
    And a PDF file with 5 embedded images
    When the Read tool is called on the PDF with pdf_mode images and no limit
    Then the tool call fails with a message naming the limit 1 and how to read fewer

  # ========================================
  # Session lifecycle: budget follows the model
  # ========================================

  Scenario: A mid-session model switch updates the image budget
    Given a session against a profile with maxImages 8
    When the session switches mid-session to a profile with maxImages 0
    Then the Read tool fails subsequent image-file reads with the no-vision message
    And the session was not recreated

  Scenario: A non-profile session resolves the default budget of 4
    Given a session created with a cloud model that has no profile behind it
    When the Read tool is called on a PDF with 5 pages in visual mode
    Then the tool call fails with an over-budget message naming the limit 4
    When the Read tool is called on a single JPEG image file
    Then the call succeeds
