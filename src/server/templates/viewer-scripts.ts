/**
 * Viewer Scripts - Client-side JavaScript for markdown viewer
 *
 * Coverage:
 * - TUI-020: Local web server for rendering markdown/mermaid attachments
 */

/**
 * Generates mermaid initialization script.
 *
 * @returns Mermaid script tag with initialization
 */
export function getMermaidScript(): string {
  return `
  <!-- Mermaid library -->
  <script type="module">
    import mermaid from 'https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs';

    // Detect theme preference
    const prefersDark = window.matchMedia && window.matchMedia('(prefers-color-scheme: dark').matches;
    const savedTheme = localStorage.getItem('fspec-theme');
    const isDark = savedTheme ? savedTheme === 'dark' : prefersDark;

    // Initialize mermaid with detected theme
    mermaid.initialize({
      startOnLoad: true,
      theme: isDark ? 'dark' : 'default',
      securityLevel: 'loose',
      fontFamily: 'monospace',
      flowchart: {
        useMaxWidth: true,
        htmlLabels: true,
        curve: 'basis'
      }
    });

    // Apply theme class
    if (isDark) {
      document.documentElement.classList.add('dark-theme');
    } else {
      document.documentElement.classList.add('light-theme');
    }

    // Re-render diagrams after DOM is ready
    window.addEventListener('DOMContentLoaded', () => {
      mermaid.run();
    });
  </script>
  `;
}

/**
 * Generates Prism syntax highlighting scripts.
 *
 * @returns Prism script tags
 */
export function getPrismScripts(): string {
  return `
  <!-- Prism JS for syntax highlighting -->
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/prism-core.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/plugins/autoloader/prism-autoloader.min.js"></script>
  `;
}

/**
 * Generates client-side interaction script for code blocks and theme toggle.
 *
 * @returns Client-side script for UI interactions
 */
export function getInteractionScript(): string {
  return `
  <script>
    // Configure Prism autoloader
    Prism.plugins.autoloader.languages_path = 'https://cdnjs.cloudflare.com/ajax/libs/prism/1.29.0/components/';

    // Language mapping for aliases
    const languageMap = {
      sh: 'bash', shell: 'bash', console: 'bash',
      js: 'javascript', ts: 'typescript', py: 'python',
      rb: 'ruby', yml: 'yaml'
    };

    function getSupportedLanguage(language) {
      if (!language || language === 'text') return 'plaintext';
      return languageMap[language.toLowerCase()] || language;
    }

    // Apply syntax highlighting and add UI features
    window.addEventListener('DOMContentLoaded', () => {
      document.querySelectorAll('pre.code-block').forEach((pre) => {
        const code = pre.querySelector('code');
        const rawLanguage = pre.getAttribute('data-language') || 'text';
        const language = getSupportedLanguage(rawLanguage);

        // Set Prism language class
        code.className = \`language-\${language}\`;

        // Add copy button
        const copyButton = document.createElement('button');
        copyButton.className = 'copy-button';
        copyButton.textContent = 'Copy';
        copyButton.onclick = () => {
          navigator.clipboard.writeText(code.textContent || '');
          copyButton.textContent = 'Copied!';
          setTimeout(() => { copyButton.textContent = 'Copy'; }, 2000);
        };
        pre.appendChild(copyButton);

        // Add language badge
        const badge = document.createElement('div');
        badge.className = 'language-badge';
        badge.textContent = rawLanguage;
        pre.appendChild(badge);
      });

      // Trigger Prism highlighting
      Prism.highlightAll();

      // Theme toggle functionality
      const toggleButton = document.getElementById('theme-toggle');
      const themeIcon = document.getElementById('theme-icon');
      const isDark = document.documentElement.classList.contains('dark-theme');

      themeIcon.textContent = isDark ? '🌙' : '☀️';

      toggleButton.onclick = () => {
        const currentlyDark = document.documentElement.classList.contains('dark-theme');

        if (currentlyDark) {
          document.documentElement.classList.remove('dark-theme');
          document.documentElement.classList.add('light-theme');
          themeIcon.textContent = '☀️';
          localStorage.setItem('fspec-theme', 'light');
        } else {
          document.documentElement.classList.remove('light-theme');
          document.documentElement.classList.add('dark-theme');
          themeIcon.textContent = '🌙';
          localStorage.setItem('fspec-theme', 'dark');
        }
      };

      // Font size controls functionality
      const MIN_FONT_SIZE = 10;
      const MAX_FONT_SIZE = 24;
      const FONT_SIZE_STEP = 2;
      const DEFAULT_FONT_SIZE = 16;

      const fontSizeIncreaseBtn = document.getElementById('font-size-increase');
      const fontSizeDecreaseBtn = document.getElementById('font-size-decrease');
      const fontSizeDisplay = document.getElementById('font-size-display');

      // Load saved font size from localStorage or use default
      const savedFontSize = localStorage.getItem('fspec-base-font-size');
      let currentFontSize = savedFontSize ? parseInt(savedFontSize, 10) : DEFAULT_FONT_SIZE;

      // Apply initial font size
      document.documentElement.style.setProperty('--base-font-size', currentFontSize + 'px');
      document.documentElement.style.setProperty('--font-scale', (currentFontSize / 16).toString());
      fontSizeDisplay.textContent = currentFontSize + 'px';

      // Update button states based on current font size
      function updateButtonStates() {
        fontSizeDecreaseBtn.disabled = currentFontSize <= MIN_FONT_SIZE;
        fontSizeIncreaseBtn.disabled = currentFontSize >= MAX_FONT_SIZE;
      }

      updateButtonStates();

      // Increase font size
      fontSizeIncreaseBtn.onclick = () => {
        const newSize = Math.min(currentFontSize + FONT_SIZE_STEP, MAX_FONT_SIZE);
        currentFontSize = newSize;
        document.documentElement.style.setProperty('--base-font-size', newSize + 'px');
        document.documentElement.style.setProperty('--font-scale', (newSize / 16).toString());
        fontSizeDisplay.textContent = newSize + 'px';
        localStorage.setItem('fspec-base-font-size', newSize.toString());
        updateButtonStates();
      };

      // Decrease font size
      fontSizeDecreaseBtn.onclick = () => {
        const newSize = Math.max(currentFontSize - FONT_SIZE_STEP, MIN_FONT_SIZE);
        currentFontSize = newSize;
        document.documentElement.style.setProperty('--base-font-size', newSize + 'px');
        document.documentElement.style.setProperty('--font-scale', (newSize / 16).toString());
        fontSizeDisplay.textContent = newSize + 'px';
        localStorage.setItem('fspec-base-font-size', newSize.toString());
        updateButtonStates();
      };

      // ========================================
      // FULLSCREEN MERMAID DIAGRAM VIEWER
      // ========================================

      let panzoomInstance = null;
      let currentModalDiagram = null;
      let isPanMode = false;
      let isMouseOverModal = false;
      let modeIndicatorTimeout = null;

      // OUR OWN pan/scale state tracking (don't trust panzoom's internal state)
      // This fixes the zoom drift bug by maintaining a single source of truth
      let ownPanX = 0;
      let ownPanY = 0;
      let ownScale = 1;

      // Add fullscreen buttons to mermaid diagrams
      function addFullscreenButtons() {
        const diagrams = document.querySelectorAll('pre.mermaid');

        diagrams.forEach((diagram, index) => {
          // Skip if already has wrapper
          if (diagram.parentElement?.classList.contains('mermaid-wrapper')) return;

          // Wrap diagram in container
          const wrapper = document.createElement('div');
          wrapper.className = 'mermaid-wrapper';
          diagram.parentNode.insertBefore(wrapper, diagram);
          wrapper.appendChild(diagram);

          // Add button overlay
          const overlay = document.createElement('div');
          overlay.className = 'mermaid-overlay';
          overlay.innerHTML = \`
            <button class="mermaid-fullscreen-btn" data-index="\${index}" title="Fullscreen">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3"/>
              </svg>
            </button>
            <button class="mermaid-download-btn" data-index="\${index}" title="Download SVG">
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" y1="15" x2="12" y2="3"/>
              </svg>
            </button>
          \`;
          wrapper.appendChild(overlay);
        });

        // Attach event listeners for fullscreen buttons
        document.querySelectorAll('.mermaid-fullscreen-btn').forEach(btn => {
          btn.addEventListener('click', (e) => {
            const index = parseInt(e.currentTarget.getAttribute('data-index'));
            openMermaidModal(index);
          });
        });

        // Attach event listeners for download buttons
        document.querySelectorAll('.mermaid-download-btn').forEach(btn => {
          btn.addEventListener('click', (e) => {
            const index = parseInt(e.currentTarget.getAttribute('data-index'));
            downloadDiagram(index);
          });
        });
      }

      // Open fullscreen modal
      function openMermaidModal(index) {
        const diagrams = document.querySelectorAll('pre.mermaid');
        const diagram = diagrams[index];
        if (!diagram) return;

        // Clone diagram content to modal
        const modal = document.getElementById('mermaid-modal');
        const container = document.getElementById('modal-diagram-container');
        container.innerHTML = diagram.innerHTML;

        // Store reference for download and panzoom
        currentModalDiagram = container.firstElementChild;

        // Show modal with animation
        modal.style.display = 'flex';
        requestAnimationFrame(() => {
          modal.classList.add('modal-visible');
        });

        // Lock body scroll
        document.body.style.overflow = 'hidden';

        // Initialize panzoom on diagram
        if (currentModalDiagram && typeof Panzoom !== 'undefined') {
          panzoomInstance = Panzoom(currentModalDiagram, {
            maxScale: 5,
            minScale: 0.5,
            startScale: 1,
            cursor: 'default',
            canvas: true,
            // Don't set origin - let panzoom handle it with zoomToPoint
          });

          // Initialize our own state tracking (single source of truth)
          ownPanX = 0;
          ownPanY = 0;
          ownScale = 1;

          // Attach custom wheel handler
          const modalBody = document.querySelector('.modal-body');
          modalBody.addEventListener('wheel', handleModalWheel, { passive: false });

          // Mouse tracking for scope
          modalBody.addEventListener('mouseenter', () => {
            isMouseOverModal = true;
          });

          modalBody.addEventListener('mouseleave', () => {
            isMouseOverModal = false;
            isPanMode = false;
            updateModeIndicator();
          });

          // Update zoom level display
          updateZoomLevel();
        }
      }

      // Close modal
      function closeMermaidModal() {
        const modal = document.getElementById('mermaid-modal');
        modal.classList.remove('modal-visible');

        // Wait for fade animation, then hide
        setTimeout(() => {
          modal.style.display = 'none';
          document.body.style.overflow = '';
          currentModalDiagram = null;

          // Cleanup panzoom
          if (panzoomInstance) {
            panzoomInstance.destroy();
            panzoomInstance = null;
          }
        }, 250);
      }

      // Zoom session state - locks the zoom point for entire gesture
      let lockedZoomPointX = null;
      let lockedZoomPointY = null;
      let zoomSessionTimeout = null;
      const ZOOM_SESSION_TIMEOUT_MS = 150;

      // Custom wheel handler for zoom/pan
      function handleModalWheel(event) {
        if (!panzoomInstance) return;

        event.preventDefault();

        const deltaX = event.deltaX;
        const deltaY = event.deltaY;
        const deltaMode = event.deltaMode;

        // Check if pan modifier is held (Space only)
        const isPanModifierHeld = isPanMode;

        // Use OUR tracked state (single source of truth - don't trust panzoom)
        const currentPan = { x: ownPanX, y: ownPanY };
        const currentScale = ownScale;

        // DEBUG: Log wheel event (JSON for easy copy/paste)
        // Heuristic to guess input device
        let likelyDevice = 'unknown';
        if (deltaMode === 1) {
          likelyDevice = 'mouse-wheel';
        } else if (deltaMode === 0) {
          // Small fractional values suggest trackpad
          const hasFractional = (Math.abs(deltaY) > 0 && Math.abs(deltaY) < 10) ||
                                (Math.abs(deltaX) > 0 && Math.abs(deltaX) < 10);
          const hasBothAxes = Math.abs(deltaX) > 0 && Math.abs(deltaY) > 0;
          if (hasFractional || hasBothAxes) {
            likelyDevice = 'trackpad';
          } else {
            likelyDevice = 'mouse-wheel-pixel-mode';
          }
        }

        console.log('WHEEL_EVENT: ' + JSON.stringify({
          likelyDevice: likelyDevice,
          clientX: event.clientX,
          clientY: event.clientY,
          deltaX: deltaX,
          deltaY: deltaY,
          deltaMode: deltaMode,
          isPanMode: isPanModifierHeld,
          currentPan: currentPan,
          currentScale: currentScale,
          lockedX: lockedZoomPointX,
          lockedY: lockedZoomPointY
        }));

        // DEBUG: Log container bounds
        const container = document.getElementById('modal-diagram-container');
        if (container) {
          const cRect = container.getBoundingClientRect();
          console.log('CONTAINER: ' + JSON.stringify({
            left: cRect.left,
            top: cRect.top,
            right: cRect.right,
            bottom: cRect.bottom,
            width: cRect.width,
            height: cRect.height
          }));
        }

        // DEBUG: Log SVG bounds
        if (currentModalDiagram) {
          const sRect = currentModalDiagram.getBoundingClientRect();
          console.log('SVG: ' + JSON.stringify({
            left: sRect.left,
            top: sRect.top,
            right: sRect.right,
            bottom: sRect.bottom,
            width: sRect.width,
            height: sRect.height
          }));
        }

        // DEBUG: Log parent bounds
        const parent = currentModalDiagram?.parentElement;
        if (parent) {
          const pRect = parent.getBoundingClientRect();
          console.log('PARENT: ' + JSON.stringify({
            left: pRect.left,
            top: pRect.top,
            right: pRect.right,
            bottom: pRect.bottom,
            width: pRect.width,
            height: pRect.height
          }));
        }

        if (isPanModifierHeld) {
          // Pan mode: Apply BOTH vertical and horizontal pan together (enables diagonal panning)
          let newX = currentPan.x;
          let newY = currentPan.y;

          if (Math.abs(deltaX) > 0) {
            newX = currentPan.x - deltaX / currentScale;
          }
          if (Math.abs(deltaY) > 0) {
            newY = currentPan.y - deltaY / currentScale;
          }

          // Apply transform directly and update our state
          if (Math.abs(deltaX) > 0 || Math.abs(deltaY) > 0) {
            const element = currentModalDiagram;
            if (element) {
              element.style.transform = 'translate(' + newX + 'px, ' + newY + 'px) scale(' + currentScale + ')';
              element.style.transformOrigin = '0 0';

              // Update our state
              ownPanX = newX;
              ownPanY = newY;

              // DEBUG: Log after pan
              console.log('AFTER_PAN: ' + JSON.stringify({
                newX: newX,
                newY: newY,
                ownPan: { x: ownPanX, y: ownPanY }
              }));
            }
          }
        } else {
          // Zoom mode: vertical scroll zooms, horizontal scroll pans

          // VERTICAL SCROLL = ZOOM
          if (Math.abs(deltaY) > 0) {
            // Lock zoom point on FIRST event in gesture
            if (lockedZoomPointX === null || lockedZoomPointY === null) {
              // Store SCREEN coordinates (not container-relative)
              lockedZoomPointX = event.clientX;
              lockedZoomPointY = event.clientY;

              // DEBUG: Log when we lock coordinates
              console.log('LOCK_ZOOM_POINT: ' + JSON.stringify({
                screenX: event.clientX,
                screenY: event.clientY,
                lockedX: lockedZoomPointX,
                lockedY: lockedZoomPointY
              }));
            }

            const zoomDelta = -deltaY * (deltaMode === 1 ? 0.05 : deltaMode ? 1 : 0.002);
            let newScale = currentScale * Math.pow(2, zoomDelta);

            // Clamp zoom to panzoom's min/max (matching initialization values)
            newScale = Math.max(0.5, Math.min(5, newScale));

            // Get parent container position
            const parentRect = currentModalDiagram.parentElement.getBoundingClientRect();

            // Get SVG's current bounding box (where it is rendered on screen RIGHT NOW)
            const svgRect = currentModalDiagram.getBoundingClientRect();

            // DEBUG: Log raw values
            console.log('ZOOM_DEBUG_1_RAW: ' + JSON.stringify({
              currentScale: currentScale,
              newScale: newScale,
              currentPan: { x: currentPan.x, y: currentPan.y },
              mouseScreen: { x: lockedZoomPointX, y: lockedZoomPointY },
              parentRect: { left: parentRect.left, top: parentRect.top, width: parentRect.width, height: parentRect.height },
              svgRect: { left: svgRect.left, top: svgRect.top, width: svgRect.width, height: svgRect.height }
            }));

            // Calculate mouse position relative to SVG's current top-left corner
            const mouseRelativeToSvgX = lockedZoomPointX - svgRect.left;
            const mouseRelativeToSvgY = lockedZoomPointY - svgRect.top;

            // DEBUG: Log mouse relative to SVG
            console.log('ZOOM_DEBUG_2_MOUSE_REL_SVG: ' + JSON.stringify({
              mouseRelativeToSvg: { x: mouseRelativeToSvgX, y: mouseRelativeToSvgY }
            }));

            // Calculate as PERCENTAGE of current SVG rendered size (0.0 to 1.0)
            const percentageX = mouseRelativeToSvgX / svgRect.width;
            const percentageY = mouseRelativeToSvgY / svgRect.height;

            // DEBUG: Log percentage
            console.log('ZOOM_DEBUG_3_PERCENTAGE: ' + JSON.stringify({
              percentage: { x: percentageX, y: percentageY }
            }));

            // Calculate new SVG rendered size based on scale change
            const scaleRatio = newScale / currentScale;
            const newRenderedWidth = svgRect.width * scaleRatio;
            const newRenderedHeight = svgRect.height * scaleRatio;

            // DEBUG: Log new size
            console.log('ZOOM_DEBUG_4_NEW_SIZE: ' + JSON.stringify({
              scaleRatio: scaleRatio,
              newRenderedSize: { width: newRenderedWidth, height: newRenderedHeight }
            }));

            // Calculate where the percentage point will be in the new zoomed SVG
            // (offset from SVG's top-left corner)
            const percentagePointX = percentageX * newRenderedWidth;
            const percentagePointY = percentageY * newRenderedHeight;

            // DEBUG: Log percentage point
            console.log('ZOOM_DEBUG_5_PERCENTAGE_POINT: ' + JSON.stringify({
              percentagePoint: { x: percentagePointX, y: percentagePointY }
            }));

            // Calculate SVG's intrinsic offset (where it sits when pan=0,0)
            // This accounts for CSS positioning, margins, or SVG viewBox offset
            const intrinsicOffsetX = svgRect.left - parentRect.left - currentPan.x;
            const intrinsicOffsetY = svgRect.top - parentRect.top - currentPan.y;

            // Position SVG so that the percentage point is directly under the mouse
            // We need to subtract the intrinsic offset because transform: translate() is RELATIVE to natural position
            const newPanX = (lockedZoomPointX - parentRect.left) - percentagePointX - intrinsicOffsetX;
            const newPanY = (lockedZoomPointY - parentRect.top) - percentagePointY - intrinsicOffsetY;

            // DEBUG: Log intrinsic offset and new pan
            console.log('ZOOM_DEBUG_6_INTRINSIC_OFFSET: ' + JSON.stringify({
              intrinsicOffset: { x: intrinsicOffsetX, y: intrinsicOffsetY }
            }));

            console.log('ZOOM_DEBUG_7_NEW_PAN: ' + JSON.stringify({
              newPan: { x: newPanX, y: newPanY }
            }));

            // DEBUG: Log before zoom
            console.log('BEFORE_ZOOM: ' + JSON.stringify({
              oldScale: currentScale,
              newScale: newScale,
              scaleRatio: scaleRatio,
              mouseScreenX: lockedZoomPointX,
              mouseScreenY: lockedZoomPointY,
              svgRect: { left: svgRect.left, top: svgRect.top, width: svgRect.width, height: svgRect.height },
              mouseRelativeToSvg: { x: mouseRelativeToSvgX, y: mouseRelativeToSvgY },
              percentage: { x: percentageX, y: percentageY },
              newRenderedSize: { width: newRenderedWidth, height: newRenderedHeight },
              percentagePoint: { x: percentagePointX, y: percentagePointY },
              oldPanX: currentPan.x,
              oldPanY: currentPan.y,
              newPanX: newPanX,
              newPanY: newPanY
            }));

            // Apply transform directly to SVG element
            const element = currentModalDiagram;
            if (element) {
              // Apply the transform directly to the element
              element.style.transform = 'translate(' + newPanX + 'px, ' + newPanY + 'px) scale(' + newScale + ')';
              element.style.transformOrigin = '0 0';  // Ensure transform origin is at top-left

              // Update our state (single source of truth)
              ownPanX = newPanX;
              ownPanY = newPanY;
              ownScale = newScale;

              // DEBUG: Log after zoom - verify what actually happened
              const actualTransform = element.style.transform;
              console.log('AFTER_ZOOM: ' + JSON.stringify({
                expectedPan: { x: newPanX, y: newPanY },
                expectedScale: newScale,
                ownState: { x: ownPanX, y: ownPanY, scale: ownScale },
                actualTransform: actualTransform
              }));

              // VERIFICATION: Check where the percentage point actually ended up
              const newSvgRect = element.getBoundingClientRect();
              const percentagePointScreenX = newSvgRect.left + percentagePointX;
              const percentagePointScreenY = newSvgRect.top + percentagePointY;
              const driftX = percentagePointScreenX - lockedZoomPointX;
              const driftY = percentagePointScreenY - lockedZoomPointY;

              console.log('ZOOM_DEBUG_8_VERIFICATION: ' + JSON.stringify({
                newSvgRect: { left: newSvgRect.left, top: newSvgRect.top, width: newSvgRect.width, height: newSvgRect.height },
                percentagePointScreen: { x: percentagePointScreenX, y: percentagePointScreenY },
                mouseScreen: { x: lockedZoomPointX, y: lockedZoomPointY },
                drift: { x: driftX, y: driftY },
                driftMagnitude: Math.sqrt(driftX * driftX + driftY * driftY)
              }));
            }

            // Reset timeout - unlock zoom point after gesture ends
            if (zoomSessionTimeout) {
              clearTimeout(zoomSessionTimeout);
            }
            zoomSessionTimeout = setTimeout(() => {
              lockedZoomPointX = null;
              lockedZoomPointY = null;
              zoomSessionTimeout = null;
              console.log('UNLOCK_ZOOM_POINT');
            }, ZOOM_SESSION_TIMEOUT_MS);
          }
          // HORIZONTAL SCROLL = PAN (only when NOT zooming)
          // Use else-if to make this mutually exclusive with zoom
          else if (Math.abs(deltaX) > 0) {
            // Calculate new pan using our own state
            const newPanX = ownPanX - deltaX / ownScale;

            // Apply transform directly
            const element = currentModalDiagram;
            if (element) {
              element.style.transform = 'translate(' + newPanX + 'px, ' + ownPanY + 'px) scale(' + ownScale + ')';
              element.style.transformOrigin = '0 0';

              // Update our state
              ownPanX = newPanX;

              // DEBUG: Log after horizontal pan
              console.log('AFTER_HPAN: ' + JSON.stringify({
                deltaX: deltaX,
                newPanX: newPanX,
                ownState: { x: ownPanX, y: ownPanY, scale: ownScale }
              }));
            }
          }
        }

        // Update indicator opacity on scroll
        showModeIndicator();
        updateZoomLevel();
      }

      // Pan modifier key tracking (Space only)
      document.addEventListener('keydown', (e) => {
        if (!isMouseOverModal || !panzoomInstance) return;

        if (e.key === ' ') {
          e.preventDefault();
          isPanMode = true;
          updateModeIndicator();
        }
      });

      document.addEventListener('keyup', (e) => {
        if (e.key === ' ') {
          isPanMode = false;
          updateModeIndicator();
        }
      });

      // Update zoom level display
      function updateZoomLevel() {
        if (!panzoomInstance) return;
        const scale = ownScale;  // Use our own state
        const percentage = Math.round(scale * 100);
        document.getElementById('zoom-level').textContent = percentage + '%';
      }

      // Update mode indicator
      function updateModeIndicator() {
        const indicator = document.querySelector('.mode-indicator');
        if (!indicator) return;

        if (isPanMode) {
          indicator.textContent = 'Pan Mode';
          indicator.classList.add('active');
          indicator.style.opacity = '1';
        } else {
          indicator.textContent = 'Zoom Mode (hold Space for Pan)';
          indicator.classList.remove('active');
          indicator.style.opacity = '0.5';
        }

        showModeIndicator();
      }

      // Show mode indicator with fade
      function showModeIndicator() {
        const indicator = document.querySelector('.mode-indicator');
        if (!indicator) return;

        indicator.style.opacity = isPanMode ? '1' : '1';

        // Fade after 2 seconds of inactivity
        clearTimeout(modeIndicatorTimeout);
        modeIndicatorTimeout = setTimeout(() => {
          if (!isPanMode) {
            indicator.style.opacity = '0.5';
          }
        }, 2000);
      }

      // Download diagram as SVG
      function downloadDiagram(index) {
        const diagrams = document.querySelectorAll('pre.mermaid');
        const diagram = diagrams[index];
        if (!diagram) return;

        const svg = diagram.querySelector('svg');
        if (!svg) return;

        const svgData = svg.outerHTML;
        const blob = new Blob([svgData], { type: 'image/svg+xml' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = \`mermaid-diagram-\${Date.now()}.svg\`;
        a.click();

        URL.revokeObjectURL(url);
      }

      // Event listeners for modal controls
      document.getElementById('modal-close').addEventListener('click', closeMermaidModal);
      document.getElementById('mermaid-modal').addEventListener('click', (e) => {
        // Close on backdrop click
        if (e.target.id === 'mermaid-modal') {
          closeMermaidModal();
        }
      });

      // Download button in modal
      document.getElementById('modal-download').addEventListener('click', () => {
        if (!currentModalDiagram) return;

        const svg = currentModalDiagram.querySelector('svg') || currentModalDiagram;
        const svgData = svg.outerHTML;
        const blob = new Blob([svgData], { type: 'image/svg+xml' });
        const url = URL.createObjectURL(blob);

        const a = document.createElement('a');
        a.href = url;
        a.download = \`mermaid-diagram-\${Date.now()}.svg\`;
        a.click();

        URL.revokeObjectURL(url);
      });

      // ESC key handler
      document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && currentModalDiagram) {
          closeMermaidModal();
        }
      });

      // Zoom control buttons
      document.getElementById('zoom-in').addEventListener('click', () => {
        if (panzoomInstance && currentModalDiagram) {
          // Calculate new scale (0.2 is panzoom's default zoom step)
          const newScale = Math.min(5, ownScale * 1.2);

          // Get container center for zoom point
          const parent = currentModalDiagram.parentElement;
          const rect = parent.getBoundingClientRect();
          const centerX = rect.width / 2;
          const centerY = rect.height / 2;

          // Zoom to center using our formula
          const worldX = (centerX - ownPanX) / ownScale;
          const worldY = (centerY - ownPanY) / ownScale;
          const newPanX = centerX - worldX * newScale;
          const newPanY = centerY - worldY * newScale;

          // Apply transform and update state
          currentModalDiagram.style.transform = 'translate(' + newPanX + 'px, ' + newPanY + 'px) scale(' + newScale + ')';
          currentModalDiagram.style.transformOrigin = '0 0';
          ownPanX = newPanX;
          ownPanY = newPanY;
          ownScale = newScale;

          updateZoomLevel();
        }
      });

      document.getElementById('zoom-out').addEventListener('click', () => {
        if (panzoomInstance && currentModalDiagram) {
          // Calculate new scale
          const newScale = Math.max(0.5, ownScale / 1.2);

          // Get container center for zoom point
          const parent = currentModalDiagram.parentElement;
          const rect = parent.getBoundingClientRect();
          const centerX = rect.width / 2;
          const centerY = rect.height / 2;

          // Zoom to center using our formula
          const worldX = (centerX - ownPanX) / ownScale;
          const worldY = (centerY - ownPanY) / ownScale;
          const newPanX = centerX - worldX * newScale;
          const newPanY = centerY - worldY * newScale;

          // Apply transform and update state
          currentModalDiagram.style.transform = 'translate(' + newPanX + 'px, ' + newPanY + 'px) scale(' + newScale + ')';
          currentModalDiagram.style.transformOrigin = '0 0';
          ownPanX = newPanX;
          ownPanY = newPanY;
          ownScale = newScale;

          updateZoomLevel();
        }
      });

      document.getElementById('zoom-reset').addEventListener('click', () => {
        if (panzoomInstance && currentModalDiagram) {
          // Reset to initial state
          currentModalDiagram.style.transform = 'translate(0px, 0px) scale(1)';
          currentModalDiagram.style.transformOrigin = '0 0';
          ownPanX = 0;
          ownPanY = 0;
          ownScale = 1;

          updateZoomLevel();
        }
      });

      // Add fullscreen buttons after mermaid diagrams are rendered
      // Wait a bit to ensure mermaid has finished rendering
      setTimeout(addFullscreenButtons, 500);
    });
  </script>
  `;
}
