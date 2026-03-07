@done
@browser-control
@LOCATE-009
Feature: Iframe-Aware DOM Scanning

  """
  Multi-phase scan: (1) getAllFrames discovers frames, (2) main frame scanPageDOM detects <iframe> elements and records their position/depth, (3) per-frame executeScript with frameIds runs scanPageDOM in each iframe, (4) service worker merges results — main frame tree + iframe subtrees spliced at the correct positions. Refs assigned per-frame with f{frameId}e{N} prefix for non-main frames. RefEntry gains frameId field so click/fill targets the right frame.
  getAllFrames() returns MORE fields than documented in research: per Chromium IDL (web_navigation.json), each frame object includes: errorOccurred (boolean), processId (integer), frameId (integer), parentFrameId (integer), url (string), documentId (string), parentDocumentId (string, optional), documentLifecycle (enum: prerender/active/cached/pending_deletion), frameType (enum: outermost_frame/fenced_frame/sub_frame). The documentId and frameType fields are particularly useful — documentId enables precise correlation with InjectionResult.documentId, and frameType distinguishes sub_frame from the main frame without relying on frameId===0.
  Chrome 133+ behavior change: Since Chrome 133, chrome.scripting.executeScript uses match_origin_as_fallback by default (PSA from Chrome Extensions DevRel, Jan 2025). This means executeScript now injects into MORE frames by default, including about:blank and sandboxed srcdoc frames that previously required explicit match_origin_as_fallback. This improves iframe scanning coverage automatically. Source: groups.google.com/a/chromium.org/g/chromium-extensions/c/D8DcJARVM90
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. manifest.json must include 'webNavigation' permission to enable chrome.webNavigation.getAllFrames() for frame discovery
  #   2. Frame discovery uses chrome.webNavigation.getAllFrames() to enumerate all frames in a tab — returns frameId, parentFrameId, url for each frame including nested iframes
  #   3. chrome.scripting.executeScript with frameIds targets specific frames — works for both same-origin AND cross-origin iframes because the extension has <all_urls> host permission
  #   4. Main frame refs remain backward-compatible (e1, e2, e3). Iframe refs use frame-prefixed format: f{frameId}e{N} (e.g. f5e1 = frame 5, element 1). The @ prefix works for both: @e1 and @f5e3
  #   5. RefEntry must gain a frameId field (0 for main frame) so click/fill can target the correct frame via chrome.scripting.executeScript({ target: { tabId, frameIds: [frameId] } })
  #   6. Iframe content appears nested under the iframe element in the accessibility tree output — the tree shows the iframe's src/name, then indented children from the frame scan
  #   7. A maxFrames parameter (default 10) limits the number of iframes scanned to prevent timeout on ad-heavy pages. Same-origin and larger iframes are prioritized over small cross-origin ones
  #   8. browser_diff_page must produce diffs on the merged multi-frame tree. Frame additions/removals (iframes dynamically created or destroyed) appear as tree-level changes
  #   9. CRITICAL CORRECTION (from Chromium source review): Sandboxed iframes WITHOUT allow-scripts CAN be scanned via chrome.scripting.executeScript in ISOLATED world (the default). The sandbox attribute only blocks MAIN world scripts. Extension content scripts in ISOLATED world bypass sandbox restrictions. This was confirmed by Chromium bug 355256366 (fixed Chrome 130) and the Chrome 133 PSA that made match_origin_as_fallback the default for executeScript. Rule [6] claim about sandbox-no-scripts skipping is WRONG for our use case.
  #   10. Non-scannable frames must be skipped gracefully: chrome-extension:// and chrome:// URLs. about:blank frames should be scanned if they have content (same-origin JS-populated). about:srcdoc frames (url='about:srcdoc') should always be scanned. Sandboxed iframes WITHOUT allow-scripts CAN still be scanned because executeScript runs in ISOLATED world which bypasses sandbox script restrictions (only MAIN world is blocked). The iframe element always appears in the tree regardless.
  #   11. Frame-to-DOM correlation uses a two-pass injection approach: (1) First pass: inject a marker into each frame via executeScript({ frameIds: [fid], func: (id) => { window.__fspec_frameId = id; }, args: [fid] }), setting a global variable with the frameId. (2) Second pass: parent frame scan detects all <iframe> elements. For same-origin iframes, reads iframe.contentWindow.__fspec_frameId for exact frameId→DOM mapping. For cross-origin iframes (where contentWindow access is blocked by SOP), falls back to matching iframe.src against getAllFrames().url — cross-origin duplicates with identical URLs are rare and use document order as tiebreaker.
  #
  # EXAMPLES:
  #   1. Page with Stripe payment iframe: scan returns main frame elements (e1, e2) plus iframe content as nested children (f5e1 'Card Number', f5e2 'Expiry', f5e3 'CVC', f5e4 'Pay'). AI fills card fields using @f5e1, @f5e2, @f5e3 and clicks @f5e4.
  #   2. Page with no iframes: scan returns only main frame elements with simple refs (e1, e2, e3) — 100% backward compatible, no behavioral change
  #   3. Click on iframe element using @f5e4: resolveRefSelector parses 'f5e4' → frameId=5, elementRef='e4', looks up CSS selector, then executeScript targets frameIds: [5] to click within the iframe
  #   4. Ad-heavy page with 25 iframes: maxFrames=10 limits scanning to the 10 largest/most relevant frames, remaining iframes show as 'iframe [skipped]' in the tree
  #   5. Nested iframes (iframe inside iframe): getAllFrames returns all frames at all depths with parentFrameId chain. Tree shows nested indentation correctly. Refs use the direct frame's ID (f12e1), not the parent chain
  #   6. Sandboxed iframe with sandbox='allow-same-origin' (no allow-scripts): executeScript in ISOLATED world still succeeds — scanPageDOM runs and returns elements. The iframe appears in tree with its content. The sandbox attribute only blocks MAIN world scripts, not extension content scripts.
  #
  # ========================================

  Background: User Story
    As a AI agent
    I want to scan and interact with elements inside iframes
    So that I can fill payment forms, complete OAuth flows, and interact with any embedded content without being blind to iframe boundaries

  Scenario: Scan a page with a cross-origin payment iframe and receive nested tree with frame-prefixed refs
    Given a page with a heading, an email input, and a cross-origin Stripe payment iframe containing card number, expiry, CVC, and pay button fields
    When I call browser_scan_page with default parameters
    Then the result should contain main frame elements with simple refs e1 and e2
    And the result should contain an iframe element showing the iframe's src URL
    And the iframe's children should appear nested under the iframe element with refs f5e1, f5e2, f5e3, f5e4
    And the tree should show the iframe content indented one level deeper than the iframe element itself

  Scenario: Page with no iframes returns backward-compatible simple refs
    Given a page with a heading, a text input, and a button but no iframes
    When I call browser_scan_page
    Then the result should contain only main frame elements with simple refs e1, e2, e3
    And no frame-prefixed refs should appear in the output
    And the behavior should be identical to pre-iframe-support scanning

  Scenario: Click an element inside an iframe using frame-prefixed ref
    Given a previous scan stored refs including f5e4 with frameId 5 and CSS selector for a pay button
    When I call browser_click_element with selector "@f5e4"
    Then resolveRefSelector should parse "f5e4" into frameId 5 and elementRef "e4"
    And executeScript should target frameIds [5] to click the element within the iframe
    And the result should confirm the click succeeded

  Scenario: Fill a form field inside an iframe using frame-prefixed ref
    Given a previous scan stored refs including f5e1 with frameId 5 and CSS selector for a card number input
    When I call browser_fill_form with selector "@f5e1" and value "4242424242424242"
    Then executeScript should target frameIds [5] to fill the input within the iframe
    And the result should confirm the value was set

  Scenario: Ad-heavy page with excess iframes respects maxFrames limit
    Given a page with 25 iframes including both same-origin content iframes and small cross-origin ad iframes
    When I call browser_scan_page with maxFrames set to 10
    Then at most 10 iframes should be scanned for content
    And the remaining iframes should appear in the tree as "iframe [skipped]"
    And same-origin and larger iframes should be prioritized over small cross-origin ones

  Scenario: Nested iframes are scanned at all depths with correct refs
    Given a page with an iframe containing another iframe inside it
    And getAllFrames returns frames at all nesting levels with parentFrameId chain
    When I call browser_scan_page
    Then the tree should show nested indentation matching the iframe nesting depth
    And refs should use the direct frame's ID not the parent chain
    And a deeply nested element should use ref format f12e1 for frame 12 element 1

  Scenario: Sandboxed iframe without allow-scripts is still scanned via ISOLATED world
    Given a page with a sandboxed iframe having sandbox attribute "allow-same-origin" but not "allow-scripts"
    When I call browser_scan_page
    Then the iframe should be scanned successfully because executeScript runs in ISOLATED world
    And the iframe's interactive elements should appear in the tree with frame-prefixed refs
    And the sandbox attribute should not prevent scanning

  Scenario: Non-scannable chrome URLs are skipped gracefully
    Given a page with iframes pointing to chrome-extension:// and chrome:// URLs
    When I call browser_scan_page
    Then the chrome-extension and chrome URL iframes should be skipped without error
    And the iframe elements should still appear in the tree without nested children
    And all other scannable frames should be scanned normally

  Scenario: about:blank iframes with content are scanned
    Given a page with an about:blank iframe that has been populated with content via JavaScript
    When I call browser_scan_page
    Then the about:blank iframe should be scanned because it may have same-origin JS-populated content
    And its interactive elements should appear nested under the iframe element

  Scenario: about:srcdoc iframes are always scanned
    Given a page with an iframe using srcdoc attribute containing inline HTML with form fields
    When I call browser_scan_page
    Then the srcdoc iframe should be scanned and its elements should appear in the tree
    And the elements should have frame-prefixed refs

  Scenario: Frame-to-DOM correlation maps frameIds to iframe elements via two-pass injection
    Given a page with multiple iframes including both same-origin and cross-origin iframes
    When the scan runs the two-pass injection
    Then first pass should inject a __fspec_frameId marker into each frame
    And second pass should correlate same-origin iframes by reading contentWindow.__fspec_frameId
    And cross-origin iframes should fall back to matching iframe.src against getAllFrames URL data

  Scenario: RefEntry includes frameId field for frame-aware click and fill
    Given a completed scan of a page with main frame and iframe elements
    When I inspect the stored RefEntry for a main frame element ref "e1"
    Then the RefEntry should have frameId 0
    When I inspect the stored RefEntry for an iframe element ref "f5e3"
    Then the RefEntry should have frameId 5

  Scenario: browser_diff_page produces diffs on merged multi-frame tree
    Given a previous scan of a page with an iframe containing a card number field
    And the iframe content has changed to show a success message
    When I call browser_diff_page
    Then the diff should show removals of iframe's old elements and additions of new elements
    And the diff should operate on the merged multi-frame tree

  Scenario: Dynamically added iframes are discovered on re-scan
    Given a page that initially has no iframes
    And a payment modal iframe is dynamically added after initial scan
    When I call browser_scan_page again
    Then the newly added iframe should appear in the scan results
    And its interactive elements should have frame-prefixed refs

  Scenario: manifest.json includes webNavigation permission
    Given the extension manifest.json
    Then the permissions array should include "webNavigation"
    And this permission should enable chrome.webNavigation.getAllFrames for frame discovery
