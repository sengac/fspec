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

        // Get current panzoom state
        const currentPan = panzoomInstance.getPan();
        const currentScale = panzoomInstance.getScale();

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

          // Single pan call for both axes (allows north-west, south-east, etc.)
          if (Math.abs(deltaX) > 0 || Math.abs(deltaY) > 0) {
            panzoomInstance.pan(newX, newY, { animate: false });
          }
        } else {
          // Zoom mode: vertical scroll zooms, horizontal scroll pans

          // VERTICAL SCROLL = ZOOM
          if (Math.abs(deltaY) > 0) {
            // Lock zoom point on FIRST event in gesture
            if (lockedZoomPointX === null || lockedZoomPointY === null) {
              lockedZoomPointX = event.clientX;
              lockedZoomPointY = event.clientY;
            }

            const zoomDelta = -deltaY * (deltaMode === 1 ? 0.05 : deltaMode ? 1 : 0.002);
            let newScale = currentScale * Math.pow(2, zoomDelta);

            // Clamp zoom to panzoom's min/max (matching initialization values)
            newScale = Math.max(0.5, Math.min(5, newScale));

            // Use LOCKED zoom point for entire gesture (handles trackpad momentum)
            panzoomInstance.zoomToPoint(newScale, { clientX: lockedZoomPointX, clientY: lockedZoomPointY }, { animate: false });

            // Reset timeout - unlock zoom point after gesture ends
            if (zoomSessionTimeout) {
              clearTimeout(zoomSessionTimeout);
            }
            zoomSessionTimeout = setTimeout(() => {
              lockedZoomPointX = null;
              lockedZoomPointY = null;
              zoomSessionTimeout = null;
            }, ZOOM_SESSION_TIMEOUT_MS);
          }

          // HORIZONTAL SCROLL = PAN (get fresh pan values in case zoom changed them)
          if (Math.abs(deltaX) > 0) {
            const updatedPan = panzoomInstance.getPan();
            panzoomInstance.pan(updatedPan.x - deltaX / currentScale, updatedPan.y, { animate: false });
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
        const scale = panzoomInstance.getScale();
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
        if (panzoomInstance) {
          panzoomInstance.zoomIn();
          updateZoomLevel();
        }
      });

      document.getElementById('zoom-out').addEventListener('click', () => {
        if (panzoomInstance) {
          panzoomInstance.zoomOut();
          updateZoomLevel();
        }
      });

      document.getElementById('zoom-reset').addEventListener('click', () => {
        if (panzoomInstance) {
          panzoomInstance.reset();
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
