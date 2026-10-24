@done
@agent-core
@session-management
@persistence
@BUG-251
Feature: Faithful multimodal input archives and cold replay
  """
  Accepted input is Unicode text followed by ordered base64 images. The original
  envelope, not a provider checkpoint, owns these bytes through the existing
  content-addressed blob store and BUG-246 recoverable writer. Stored PDF and
  URL media are replay compatibility, not a new document-send UI. URL identity
  is retained without promising immutable remote content. Archival retention
  remains independent of model image limits. See BUG-251/api-proposal.md.
  """

  Background: User Story
    """
    As a user attaching images to a conversation
    I want original attachments retained and restored faithfully
    So that a resumed conversation can use the same supported media
    """

  Scenario: Accepted images survive originals and cold next request
    Given a real session accepting Unicode text and two ordered images
    When the input is sent through the real agent loop
    Then bounded originals preserve the original text image order types and bytes
    And a fresh process sends those images on the next physical provider request

  Scenario: Stored documents and remote media retain supported source identities
    Given ordered stored PDF documents and remote image sources
    When the structured user content is archived and cold restored
    Then the next physical provider request preserves supported media source identities
    And originals retain document metadata without claiming captured remote bytes

  Scenario: Invalid or unsupported restored media fails visibly
    Given stored media with unsupported formats or invalid source metadata
    When cold restoration decodes the media
    Then restoration returns a compatibility error instead of text substitution

  Scenario: Referenced media survives cleanup and missing blobs fail visibly
    Given archived media using the existing content addressed blob store
    When unrelated blobs are cleaned and the session is reopened
    Then referenced media bytes remain available in bounded originals
    And removing a referenced blob returns a typed missing source error

  Scenario: Legacy Unicode text remains compatible
    Given a legacy text only conversation containing Unicode
    When the session is cold restored
    Then the next physical provider request retains the original text

  Scenario: Tool image budgets do not discard original attachments
    Given an archived input with two images and a tool image budget of one
    When the session resumes and the tool image budget is queried
    Then the tool image budget remains one and originals still retain both images
