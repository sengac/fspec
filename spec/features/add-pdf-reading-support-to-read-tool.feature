@done
@read
@multimodal
@high
@tools
@TOOLS-002
Feature: Add PDF Reading Support to Read Tool

  """
  VISUAL MODE: Uses pdfium-render crate (Rust wrapper for Google Pdfium C++ library). Renders pages via PdfPage::render_with_config() at 150 DPI to PdfBitmap, converts to image::DynamicImage, encodes as PNG base64. Returns array of {page_number, data, media_type} objects.

  TEXT MODE: Uses lopdf crate (pure Rust). Loads via Document::load_mem(), iterates pages with get_pages(), extracts text with extract_text(&[page_num]). Returns paginated string with '--- Page N ---' separators. Already implemented in pdf.rs.

  IMAGES MODE: Uses lopdf crate. Iterates PDF objects looking for XObject streams with /Subtype /Image. Extracts raw image bytes, determines format from /Filter (DCTDecode=JPEG, FlateDecode+decode=PNG). Returns array of {data, media_type, width, height} objects.

  ENCRYPTION DETECTION: Check raw bytes for /Encrypt dictionary marker BEFORE lopdf parsing. Also check Document::is_encrypted() after load. Return PdfError::Encrypted early to avoid parsing errors on encrypted content streams.

  API DESIGN: ReadArgs gains optional pdf_mode: Option<String> field. Values: 'visual' (default), 'text', 'images'. Invalid values fall back to visual. Output varies by mode: visual/images return JSON array of image objects, text returns paginated string.

  DEPENDENCY: pdfium-render requires Pdfium C library. Use pdfium-render with 'static' feature to bundle Pdfium, or 'dynamic' to link at runtime. Consider platform-specific builds (pdfium-render supports Linux, macOS, Windows, WASM).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Read tool MUST support three PDF modes via optional pdf_mode parameter: 'visual' (render pages as images), 'text' (extract text), 'images' (extract embedded images)
  #   2. Default mode MUST be 'visual' because multimodal LLMs can see rendered pages, preserving ALL information (text, layout, diagrams, charts) that text extraction loses
  #   3. Visual mode MUST render each PDF page as a PNG image at 150 DPI and return as base64-encoded data array with page numbers
  #   4. Text mode MUST extract text page-by-page with clear page number labels (--- Page N ---), preserving reading order
  #   5. Images mode MUST iterate PDF XObject streams to extract all embedded images, returning base64 array with media types and dimensions
  #   6. Password-protected PDFs MUST be detected via /Encrypt dictionary marker in raw bytes BEFORE parsing, returning clear error: 'Cannot read password-protected PDF'
  #   7. All PDF modes MUST be exempt from text token limits since they return structured content (base64 images or paginated text)
  #   8. Visual mode SHOULD include page count in output so LLM knows total context being consumed
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
    Given a PDF file "report.pdf" with 25 pages of content
    When the read tool is called with pdf_mode="visual"
    Then the response should include the total page count (25)
    And all 25 pages should be rendered as base64-encoded PNG images


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
