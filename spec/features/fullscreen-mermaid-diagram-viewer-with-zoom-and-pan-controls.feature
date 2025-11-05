@tui
@done
@zoom-pan
@fullscreen
@mermaid
@ui-enhancement
@viewer
@high
@VIEW-001
Feature: Fullscreen Mermaid Diagram Viewer with Zoom and Pan Controls
  """
  Uses Panzoom.js library (11KB, no dependencies) for zoom and pan functionality. Vanilla JavaScript implementation in viewer (not React). Modal uses createPortal pattern translated to vanilla DOM manipulation. Zoom algorithm: cursor-centered zoom using panzoom.zoomToPoint(scale, {clientX, clientY}). Pan modifier: Space key only (hardcoded, no configuration). Body scroll locked when modal open (document.body.style.overflow = 'hidden'). Zoom range: 0.5x - 5.0x. Files modified: viewer-template.ts (modal HTML), viewer-scripts.ts (panzoom initialization, event handlers), viewer-styles.ts (modal and control CSS).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Fullscreen button must appear on hover over mermaid diagram
  #   2. Fullscreen modal must cover entire viewport with semi-transparent backdrop
  #   3. Modal must close on ESC key press or backdrop click
  #   4. Vertical scroll wheel must zoom in/out centered on cursor position
  #   5. Horizontal scroll wheel must pan diagram left/right
  #   6. Holding modifier key (Space/Shift/Alt) must switch vertical scroll from zoom to pan
  #   7. Visual indicator must show current mode (Zoom Mode or Pan Mode)
  #   8. Zoom range must be limited between 0.5x and 5.0x
  #   9. Download button must export diagram as SVG file
  #
  # EXAMPLES:
  #   1. User hovers over mermaid diagram, fullscreen button fades in at top-right corner
  #   2. User clicks fullscreen button, modal opens with smooth scale/fade animation covering entire viewport
  #   3. User presses ESC key while modal is open, modal closes with fade-out animation
  #   4. User clicks dark backdrop outside diagram, modal closes
  #   5. User scrolls mouse wheel up over diagram, diagram zooms in centered on cursor position
  #   6. User scrolls mouse wheel down, diagram zooms out centered on cursor position
  #   7. User scrolls horizontally (trackpad two-finger swipe), diagram pans left or right
  #   8. User holds Space key and scrolls vertically, diagram pans up/down instead of zooming
  #   9. Mode indicator shows 'Zoom Mode (hold Space for Pan)' at bottom-left, fades to 50% opacity after 2 seconds of inactivity
  #   10. User holds Space key, mode indicator changes to 'Pan Mode' with 100% opacity and accent color background
  #   11. User clicks zoom in button (+), diagram zooms in by fixed increment
  #   12. User clicks reset button, diagram returns to 100% zoom (1.0x) centered in modal
  #   13. User clicks download button, SVG file downloads with filename 'mermaid-diagram-{timestamp}.svg'
  #   14. User tries to zoom beyond 5.0x, zoom stops at maximum (5.0x)
  #   15. User tries to zoom below 0.5x, zoom stops at minimum (0.5x)
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the pan modifier key be configurable in the UI (like mindstrike) or hardcoded to Space?
  #   A: true
  #
  #   Q: Should zoom level be persisted across sessions (localStorage) or reset to 1.0x each time?
  #   A: true
  #
  #   Q: Should there be keyboard shortcuts for zoom (+ - 0 keys) in addition to mouse wheel?
  #   A: true
  #
  #   Q: Should body scroll be locked when modal is open to prevent background scrolling?
  #   A: true
  #
  #   Q: Should the fullscreen button also include a download button (two buttons on hover) or keep download only in modal header?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer or user viewing mermaid diagrams in fspec attachment viewer
    I want to view mermaid diagrams in fullscreen with zoom and pan controls
    So that I can examine complex diagrams in detail without leaving the viewer

  Scenario: Display fullscreen and download buttons on diagram hover
    Given I am viewing a markdown file with a mermaid diagram in the attachment viewer
    When I hover my mouse over the mermaid diagram
    Then the fullscreen button should fade in at the top-right corner
    And the download button should also be visible

  Scenario: Open diagram in fullscreen modal
    Given I am viewing a mermaid diagram
    When I click the fullscreen button
    Then a fullscreen modal should open with smooth scale and fade animation
    And the modal should cover the entire viewport
    And a semi-transparent backdrop should be visible
    And the diagram should be centered in the modal

  Scenario: Close modal with ESC key
    Given the fullscreen modal is open
    When I press the ESC key
    Then the modal should close with a fade-out animation
    And the body scroll should be restored

  Scenario: Close modal with backdrop click
    Given the fullscreen modal is open
    When I click the dark backdrop outside the diagram
    Then the modal should close

  Scenario: Zoom in with mouse wheel
    Given the fullscreen modal is open showing a diagram
    When I scroll the mouse wheel up
    Then the diagram should zoom in
    And the zoom should be centered on the cursor position

  Scenario: Zoom out with mouse wheel
    Given the fullscreen modal is open showing a diagram
    When I scroll the mouse wheel down
    Then the diagram should zoom out
    And the zoom should be centered on the cursor position

  Scenario: Pan diagram with horizontal scroll
    Given the fullscreen modal is open showing a diagram
    When I scroll horizontally with a trackpad two-finger swipe
    Then the diagram should pan left or right

  Scenario: Switch to pan mode with Space key
    Given the fullscreen modal is open showing a diagram
    When I hold the Space key and scroll vertically
    Then the diagram should pan up or down instead of zooming

  Scenario: Display mode indicator in zoom mode
    Given the fullscreen modal is open
    When I am in zoom mode
    Then the mode indicator should show 'Zoom Mode (hold Space for Pan)' at the bottom-left
    And the indicator should fade to 50% opacity after 2 seconds of inactivity

  Scenario: Display mode indicator in pan mode
    Given the fullscreen modal is open
    When I hold the Space key
    Then the mode indicator should change to 'Pan Mode'
    And the indicator should have 100% opacity
    And the indicator background should have an accent color

  Scenario: Zoom in with zoom button
    Given the fullscreen modal is open showing a diagram
    When I click the zoom in button (+)
    Then the diagram should zoom in by a fixed increment

  Scenario: Reset zoom to default
    Given the fullscreen modal is open with a zoomed diagram
    When I click the reset button
    Then the diagram should return to 100% zoom (1.0x)
    And the diagram should be centered in the modal

  Scenario: Download diagram as SVG
    Given the fullscreen modal is open showing a diagram
    When I click the download button
    Then an SVG file should be downloaded
    And the filename should match the pattern 'mermaid-diagram-{timestamp}.svg'

  Scenario: Enforce maximum zoom limit
    Given the fullscreen modal is open with a diagram at 5.0x zoom
    When I try to zoom in further
    Then the zoom should remain at 5.0x maximum

  Scenario: Enforce minimum zoom limit
    Given the fullscreen modal is open with a diagram at 0.5x zoom
    When I try to zoom out further
    Then the zoom should remain at 0.5x minimum
