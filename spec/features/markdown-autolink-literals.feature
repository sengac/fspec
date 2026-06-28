@done
@viewer
@markdown
@rust
@markdown-formatting
@attachment-viewer
@RPC-379
Feature: Bare-URL and email autolink rendering parity in the Rust markdown viewer

  """
  Add a new module markdown/autolink.rs (kept under 300 lines) exposing a function that transforms a Vec<Event> (or iterates the event stream) and rewrites Event::Text nodes into Text + Start(Tag::Link) + Text + End(TagEnd::Link) sequences for detected URLs/emails. Apply it inside render.rs flush_passthrough BEFORE push_html so the autolinked events are serialized by pulldown's own HTML writer (keeping escaping identical to surrounding text).
  Skip-context tracking: the transform must NOT autolink Event::Text that appears inside a Tag::Link (track Start/End(Link) nesting depth) and must never touch Event::Code (inline code) which is a single non-Text event. Because render.rs buffers inline code as Event::Code in the passthrough vec, skipping non-Text events naturally excludes inline code. Heading inner text flows through HeadingState::render via push_html separately; decide and document whether autolinks apply inside headings (recommended: leave headings as-is for simplicity since slug text is plain).
  URL/email detection: match http:// and https:// schemes and bare emails (local-part@domain with a dot in the domain). Apply GFM-style trailing-punctuation trimming: strip trailing <>?!.,:;*_~ and trim an unmatched trailing ) when parens are unbalanced. Emit Tag::Link { link_type: LinkType::Inline or Autolink, dest_url: CowStr (mailto: prefix for emails), title: empty, id: empty }. Reference marked 16.4.2 / GFM autolink literal output for expected strings. Keep all files <300 lines; cargo test, cargo clippy -- -D warnings, and cargo fmt --check must pass. Preserve the fspec.pro axum architecture (no HTTP-layer changes).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A bare http:// or https:// URL in plain text renders as an anchor whose href equals the URL and whose visible text is the URL
  #   2. A bare email address in plain text renders as an anchor whose href is mailto:<email> and whose visible text is the email
  #   3. Autolinking applies only to literal text; URLs inside an existing markdown link or inside inline code spans are never autolinked
  #   4. Trailing sentence punctuation immediately after a URL (such as a period, comma, or closing paren) is excluded from the link, matching GFM autolink literal trimming
  #   5. Autolinked URL and email text is HTML-escaped consistently with the surrounding rendered text
  #   6. Existing rendering (headings with slug ids, mermaid blocks, language-tagged code blocks, soft-break handling) continues to work unchanged when autolinking is added
  #
  # EXAMPLES:
  #   1. Text 'See https://example.com for details' renders an anchor: <a href="https://example.com">https://example.com</a>
  #   2. Text 'Email a@b.com please' renders an anchor: <a href="mailto:a@b.com">a@b.com</a>
  #   3. An existing markdown link '[label](https://example.com)' renders a single anchor with text 'label'; the URL is not double-linked
  #   4. Text 'Visit https://example.com.' (trailing period) renders href='https://example.com' with the period left outside the anchor
  #   5. A URL written inside an inline code span like `https://x.com` is NOT autolinked and stays inside the code element
  #   6. Both 'http://example.com' and 'https://example.com' in plain text autolink to anchors with matching hrefs
  #
  # ========================================

  Background: User Story
    As a fspec user viewing markdown attachments in the browser viewer
    I want to have bare URLs and email addresses rendered as clickable links
    So that I can navigate to referenced links directly, matching the TypeScript viewer's behavior

  Scenario: Autolink a bare https URL in plain text
    Given markdown text "See https://example.com for details"
    When the markdown is rendered to HTML
    Then the output contains "<a href=\"https://example.com\">https://example.com</a>"

  Scenario: Autolink a bare email address as a mailto link
    Given markdown text "Email a@b.com please"
    When the markdown is rendered to HTML
    Then the output contains "<a href=\"mailto:a@b.com\">a@b.com</a>"

  Scenario: An existing markdown link is not double-linked
    Given markdown text "[label](https://example.com)"
    When the markdown is rendered to HTML
    Then the output contains a single anchor with visible text "label"
    And the output does not wrap the destination URL in a nested anchor

  Scenario: Trailing sentence punctuation is excluded from the autolink
    Given markdown text "Visit https://example.com."
    When the markdown is rendered to HTML
    Then the output contains an anchor with href "https://example.com"
    And the trailing period is left outside the anchor

  Scenario: A URL inside an inline code span is not autolinked
    Given markdown text "Run `https://x.com` now"
    When the markdown is rendered to HTML
    Then the URL stays inside a code element
    And the output does not contain an anchor for that URL

  Scenario: Both http and https schemes are autolinked
    Given markdown text "http://example.com and https://example.com"
    When the markdown is rendered to HTML
    Then the output contains an anchor with href "http://example.com"
    And the output contains an anchor with href "https://example.com"

  Scenario: Existing rendering is unaffected by autolinking
    Given markdown containing a mermaid block and a python code block and a heading
    When the markdown is rendered to HTML
    Then the mermaid block still renders as "<pre class=\"mermaid\">"
    And the python code block still renders with data-language "python"
    And the heading still renders a slug id

  Scenario: A URL preceded by non-ASCII text is autolinked without panicking
    Given markdown text "café https://example.com"
    When the markdown is rendered to HTML
    Then the output contains "<a href=\"https://example.com\">https://example.com</a>"
    And rendering does not panic
