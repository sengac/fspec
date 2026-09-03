@done
@read
@multimodal
@high
@tools
@TOOLS-002
@BUG-168
Feature: Add PDF Reading Support to Read Tool
  """
  VISUAL MODE: Uses hayro (pure Rust). Renders pages at 150 DPI to PNG, encodes as base64. Returns {path, total_pages, pages:[{page_number, data, media_type}], notice} objects.

  TEXT MODE: Uses lopdf (pure Rust). Loads via Document::load_mem(), extracts text with extract_text(&[page_num]). Returns paginated string with '--- Page N ---' separators.

  IMAGES MODE: Uses lopdf. Iterates PDF objects looking for XObject streams with /Subtype /Image. Extracts raw image bytes, determines format from /Filter. Returns {path, total_pages, image_count, images:[{index, data, media_type, width, height}], notice}.

  PAGINATION (BUG-168): All three modes honor offset (1-based page, or embedded-image index in images mode) and limit (max pages / max images). When limit is absent, the configurable default cap CODELET_MAX_PDF_PAGES (default 20, env-var mirroring CODELET_MAX_FILE_TOKENS) applies. When fewer items are returned than remain, a truncation notice is included carrying the total count and the exact next offset so the LLM can paginate. An explicit limit always wins over the default cap.

  ENCRYPTION DETECTION: Check raw bytes for /Encrypt dictionary marker BEFORE lopdf parsing. Also check Document::is_encrypted() after load. Return PdfError::Encrypted early to avoid parsing errors on encrypted content streams.

  API DESIGN: ReadArgs gains optional pdf_mode: Option<String> field. Values: 'visual' (default), 'text', 'images'. Invalid values fall back to visual. Output varies by mode: visual/images return JSON objects with image arrays, text returns paginated string.

  DEPENDENCY: hayro (pure Rust, no native Pdfium C library).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Read tool MUST support three PDF modes via optional pdf_mode parameter: 'visual' (render pages as images), 'text' (extract text), 'images' (extract embedded images)
  #   2. Default mode MUST be 'visual' for vision-capable models because multimodal LLMs can see rendered pages, preserving ALL information (text, layout, diagrams, charts) that text extraction loses
  #   3. Visual mode MUST render each PDF page as a PNG image at 150 DPI and return as base64-encoded data array with page numbers
  #   4. Text mode MUST extract text page-by-page with clear page number labels (--- Page N ---), preserving reading order
  #   5. Images mode MUST iterate PDF XObject streams to extract embedded images, returning base64 array with media types and dimensions
  #   6. Password-protected PDFs MUST be detected via /Encrypt dictionary marker in raw bytes BEFORE parsing, returning clear error: 'Cannot read password-protected PDF'
  #   7. All PDF modes MUST be exempt from text token limits since they return structured content (base64 images or paginated text)
  #   8. Visual mode SHOULD include page count in output so LLM knows total context being consumed
  #   9. (BUG-168) offset/limit MUST be honored in ALL three PDF modes: visual/text limit the pages processed (offset is 1-based), images mode limits the embedded images (offset skips N images)
  #   10. (BUG-168) When no explicit limit is given, a configurable default page cap (env var CODELET_MAX_PDF_PAGES, default 20) MUST bound how many pages/images are returned
  #   11. (BUG-168) When fewer items are returned than remain in the document, a truncation notice MUST state the total count and the exact offset to continue with (pagination)
  #   12. (BUG-168) When the active session model lacks vision capability and pdf_mode is NOT explicitly set, the Read tool MUST default to text mode with a one-line notice (an explicit pdf_mode always wins)
  #
  # EXAMPLES:
  #   1. LLM reads architecture-diagram.pdf (no mode specified) -> visual mode renders pages as images -> LLM describes system components and their connections
  #   2. LLM reads api-spec.pdf with pdf_mode=text to find endpoints -> text mode extracts searchable text -> LLM locates POST /users endpoint definition
  #   3. LLM reads flowchart.pdf to understand a business process -> visual mode (default) shows the diagram -> LLM traces decision paths and explains the flow
  #   4. LLM reads product-catalog.pdf with pdf_mode=images -> images mode extracts 15 embedded product photos -> LLM can analyze or describe each product image
  #   5. LLM reads contract.pdf with pdf_mode=text for legal analysis -> all 30 pages extracted as text with page numbers -> LLM searches for liability clauses
  #   6. LLM tries to read encrypted.pdf -> encryption detected before parsing -> returns error 'Cannot read password-protected PDF: encrypted.pdf'
  #   7. LLM reads presentation.pdf with slides and charts -> visual mode (default) renders each slide -> LLM can describe slide content including graphs and diagrams
  #   8. LLM reads scanned-document.pdf (image-based PDF) -> visual mode renders the scanned image -> LLM can read text from the rendered image (text mode would return empty)
  #   9. LLM reads UML-class-diagram.pdf -> visual mode renders diagram -> LLM identifies classes, inheritance relationships, and method signatures from the visual
  #   10. (BUG-168) LLM reads 67-page requirements.pdf with limit=4 -> only 4 pages returned + notice 'Rendered 4 of 67 pages (limit). Continue with offset=5.'
  #   11. (BUG-168) LLM reads 67-page requirements.pdf (no limit, default cap 20) -> 20 pages returned + notice to continue with offset=21
  #   12. (BUG-168) LLM with a non-vision model (pdf_mode unset) reads a PDF -> text mode used automatically with a notice, no images burned into context
  #
  # ========================================
  Background: User Story
    As a developer using codelet for code exploration
    I want to read PDF files in different modes - visual rendering, text extraction, or embedded image extraction
    So that I can explore PDF documentation visually for diagrams, extract text for searching, or pull out embedded images as needed

  Scenario: Read PDF with default visual mode renders pages as images
    Given a PDF file "architecture-diagram.pdf" containing diagrams
    When the read tool is called with no pdf_mode specified
    Then each page should be rendered as a PNG image at 150 DPI
    And the response should include page count and base64-encoded image data
    And each image should have page_number, data, and media_type fields

  Scenario: Read PDF with explicit text mode extracts searchable text
    Given a PDF file "api-spec.pdf" with multiple pages of text content
    When the read tool is called with pdf_mode="text"
    Then text should be extracted from each page
    And each page should be labeled with "--- Page N ---" separator
    And the reading order should be preserved

  Scenario: Read PDF with images mode extracts embedded images
    Given a PDF file "product-catalog.pdf" with embedded product photos
    When the read tool is called with pdf_mode="images"
    Then all embedded XObject images should be extracted from the PDF
    And each image should be returned with base64 data and media type
    And image dimensions (width, height) should be included

  Scenario: Reject password-protected PDF with clear error before parsing
    Given a password-protected PDF file "encrypted.pdf"
    When the read tool is called with any pdf_mode
    Then encryption should be detected via /Encrypt marker in raw bytes
    And an error should be returned before parsing attempts
    And the error message should be "Cannot read password-protected PDF: encrypted.pdf"

  Scenario: Visual mode includes page count for context awareness
    Given a PDF file "report.pdf" with 3 pages of content
    When the read tool is called with pdf_mode="visual" and no offset or limit
    Then the response should include the total page count (3)
    And all 3 pages should be rendered as base64-encoded PNG images
    And no truncation notice should be present because the whole document fit

  Scenario: Text mode handles scanned PDFs gracefully
    Given a scanned PDF file "scanned-document.pdf" with no extractable text
    When the read tool is called with pdf_mode="text"
    Then empty or minimal text should be returned
    And the output should still include page separators

  Scenario: All PDF modes are exempt from text token limits
    Given a large PDF file that would exceed the text token limit
    When the read tool is called with any pdf_mode
    Then the PDF should be processed successfully without token limit error
    And the appropriate content should be returned based on the mode

  Scenario: Invalid pdf_mode falls back to visual mode
    Given a PDF file "document.pdf" with mixed content
    When the read tool is called with pdf_mode="invalid_mode"
    Then the PDF should be processed using visual mode as fallback
    And pages should be rendered as PNG images

  # ====================================================================
  # BUG-168: PDF pagination, configurable page cap, vision awareness
  # ====================================================================

  @bug-168
  @truncation
  Scenario: Visual mode honors offset and limit for PDF pages
    Given a PDF file "requirements.pdf" with 67 pages
    When the read tool is called with pdf_mode="visual", offset=1, limit=4
    Then exactly 4 rendered pages should be returned covering pages 1 through 4
    And the response should include the total page count (67)
    And a truncation notice should tell the reader the document has more pages
    And the truncation notice should name the next offset (5) to continue reading with

  @bug-168
  @truncation
  Scenario: Visual mode honors an offset that starts mid-document
    Given a PDF file "report.pdf" with 10 pages
    When the read tool is called with pdf_mode="visual", offset=3, limit=2
    Then exactly 2 rendered pages should be returned covering pages 3 and 4
    And the truncation notice should name the next offset (5) to continue reading with

  @bug-168
  @truncation
  Scenario: Text mode honors offset and limit for PDF pages
    Given a PDF file "spec.pdf" with 10 pages of text
    When the read tool is called with pdf_mode="text", offset=2, limit=3
    Then text should be extracted only from pages 2 through 4
    And the output should include the total page count (10)
    And a truncation notice should name the next offset (5) to continue reading with

  @bug-168
  @truncation
  Scenario: Images mode honors offset and limit for embedded images
    Given a PDF file "catalog.pdf" with 6 embedded images
    When the read tool is called with pdf_mode="images", offset=2, limit=3
    Then exactly 3 embedded images should be returned (the 2nd, 3rd and 4th)
    And the response should include the total image count (6)
    And a truncation notice should name the next offset (5) to continue with

  @bug-168
  @truncation
  Scenario: Default page cap bounds an unbounded PDF read
    Given a PDF file "huge.pdf" with 25 pages
    And CODELET_MAX_PDF_PAGES is at its default value
    When the read tool is called with pdf_mode="visual" and no offset or limit
    Then at most the default cap (20) pages should be returned
    And the response should include the total page count (25)
    And a truncation notice should name the next offset (21) to continue reading with

  @bug-168
  @truncation
  Scenario: Configurable page cap via environment variable
    Given a PDF file "configurable.pdf" with 25 pages
    And CODELET_MAX_PDF_PAGES is set to 5
    When the read tool is called with pdf_mode="visual" and no offset or limit
    Then exactly 5 pages should be returned
    And a truncation notice should name the next offset (6) to continue reading with

  @bug-168
  @truncation
  Scenario: Explicit limit wins over the default page cap
    Given a PDF file "limited.pdf" with 25 pages
    And CODELET_MAX_PDF_PAGES is set to 10
    When the read tool is called with pdf_mode="visual", offset=1, limit=3
    Then exactly 3 pages should be returned
    And a truncation notice should name the next offset (4) to continue reading with

  @bug-168
  @truncation
  Scenario: A PDF that fits entirely is returned without a truncation notice
    Given a PDF file "short.pdf" with 4 pages
    And CODELET_MAX_PDF_PAGES is at its default value
    When the read tool is called with pdf_mode="visual" and no offset or limit
    Then all 4 pages should be returned
    And no truncation notice should be present

  @bug-168
  @truncation
  Scenario: An offset beyond the document still reports the total
    Given a PDF file "tail.pdf" with 5 pages
    When the read tool is called with pdf_mode="visual", offset=10, limit=4
    Then zero rendered pages should be returned
    And the response should still include the total page count (5)
    And a truncation-free notice should make clear there are no pages at that offset

  @bug-168
  Scenario: Non-vision session model defaults PDF reads to text mode
    Given a PDF file "doc.pdf" with 3 pages
    And the session model is registered in the tool layer as lacking vision capability
    When the read tool is called with no pdf_mode specified
    Then text mode should be used automatically
    And the output should include a one-line notice that visual mode is unavailable for this model
    And no page images should be returned

  @bug-168
  Scenario: Explicit pdf_mode wins even when the session model lacks vision
    Given a PDF file "doc.pdf" with 3 pages
    And the session model is registered in the tool layer as lacking vision capability
    When the read tool is called with pdf_mode="visual" explicitly
    Then visual mode should be honored and pages rendered as images
    And no vision-unavailable notice should be present

  @bug-168
  Scenario: Unregistered session keeps the historical visual default
    Given a PDF file "doc.pdf" with 3 pages
    And the session model capability is NOT registered in the tool layer
    When the read tool is called with no pdf_mode specified
    Then visual mode should be used (historical default preserved)
    And no vision-unavailable notice should be present
