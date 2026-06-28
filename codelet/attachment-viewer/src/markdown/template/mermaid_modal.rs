//! Fullscreen mermaid modal — port of the modal pieces of
//! `src/server/templates/viewer-scripts.ts` and `viewer-template.ts`.
//!
//! Three static server-emitted strings: the Panzoom v4.5.1 CDN `<script>` tag,
//! the modal markup added to the document body, and the interaction script that
//! wraps each `pre.mermaid` with Fullscreen + Download-SVG buttons and drives the
//! Panzoom-based zoom/pan modal (clamp 0.5x-5x, x1.2 factors, Space-to-pan, ESC /
//! backdrop close, live percentage readout, SVG Blob download). The cursor-
//! centered wheel-zoom + horizontal-pan + mode-indicator-fade JS lives in
//! [`super::mermaid_wheel`] and is spliced into the modal script by
//! [`modal_script`].

/// Panzoom v4.5.1 CDN `<script>` tag (loaded before the modal script).
pub const PANZOOM_CDN: &str =
    "  <script src=\"https://cdn.jsdelivr.net/npm/@panzoom/panzoom@4.5.1/dist/panzoom.min.js\"></script>";

/// Fullscreen modal markup injected into the document `<body>`.
pub const MODAL_MARKUP: &str = r##"
  <div id="mermaid-modal" class="modal-backdrop" style="display: none;">
    <div class="modal-container">
      <header class="modal-header">
        <h2 class="modal-title">Diagram Fullscreen View</h2>
        <div class="modal-controls">
          <button id="modal-download" class="modal-button" title="Download SVG">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
              <polyline points="7 10 12 15 17 10"/>
              <line x1="12" y1="15" x2="12" y2="3"/>
            </svg>
          </button>
          <button id="modal-close" class="modal-button" title="Close (ESC)">
            <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      </header>
      <div class="modal-body">
        <div id="modal-diagram-container" class="diagram-container"></div>
      </div>
      <div class="zoom-controls">
        <button id="zoom-in" class="zoom-btn" title="Zoom In">+</button>
        <button id="zoom-out" class="zoom-btn" title="Zoom Out">&#8722;</button>
        <button id="zoom-reset" class="zoom-btn" title="Reset Zoom">&#10226;</button>
        <span id="zoom-level" class="zoom-level">100%</span>
      </div>
      <div class="mode-indicator">Zoom Mode (hold Space for Pan)</div>
    </div>
  </div>
"##;

/// Interaction script driving the fullscreen mermaid modal.
///
/// Assembled from [`SCRIPT_HEAD`] (state, helpers, event wiring), the spliced-in
/// [`mermaid_wheel::WHEEL_JS`] (wheel zoom/pan + mode-indicator fade), and
/// [`SCRIPT_FOOT`] (deferred button setup + closing tags). Splitting the wheel
/// JS into a submodule keeps every template file under 300 lines.
pub fn modal_script() -> String {
    format!(
        "{SCRIPT_HEAD}{wheel}{SCRIPT_FOOT}",
        wheel = super::mermaid_wheel::WHEEL_JS
    )
}

const SCRIPT_HEAD: &str = r#"
  <script>
    window.addEventListener('DOMContentLoaded', () => {
      let panzoomInstance = null;
      let currentModalDiagram = null;
      let isPanMode = false;
      let isMouseOverModal = false;
      let ownPanX = 0, ownPanY = 0, ownScale = 1;
      let modeIndicatorTimeout = null;

      function addFullscreenButtons() {
        const diagrams = document.querySelectorAll('pre.mermaid');
        diagrams.forEach((diagram, index) => {
          if (diagram.parentElement && diagram.parentElement.classList.contains('mermaid-wrapper')) return;
          const wrapper = document.createElement('div');
          wrapper.className = 'mermaid-wrapper';
          diagram.parentNode.insertBefore(wrapper, diagram);
          wrapper.appendChild(diagram);
          const overlay = document.createElement('div');
          overlay.className = 'mermaid-overlay';
          overlay.innerHTML =
            '<button class="mermaid-fullscreen-btn" data-index="' + index + '" title="Fullscreen">' +
            '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">' +
            '<path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3"/></svg></button>' +
            '<button class="mermaid-download-btn" data-index="' + index + '" title="Download SVG">' +
            '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">' +
            '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/>' +
            '<line x1="12" y1="15" x2="12" y2="3"/></svg></button>';
          wrapper.appendChild(overlay);
        });
        document.querySelectorAll('.mermaid-fullscreen-btn').forEach((btn) => {
          btn.addEventListener('click', (e) => openMermaidModal(parseInt(e.currentTarget.getAttribute('data-index'), 10)));
        });
        document.querySelectorAll('.mermaid-download-btn').forEach((btn) => {
          btn.addEventListener('click', (e) => downloadDiagram(parseInt(e.currentTarget.getAttribute('data-index'), 10)));
        });
      }

      function applyTransform(x, y, scale) {
        const el = currentModalDiagram;
        if (!el) return;
        el.style.transform = 'translate(' + x + 'px, ' + y + 'px) scale(' + scale + ')';
        el.style.transformOrigin = '0 0';
        ownPanX = x; ownPanY = y; ownScale = scale;
      }

      function openMermaidModal(index) {
        const diagrams = document.querySelectorAll('pre.mermaid');
        const diagram = diagrams[index];
        if (!diagram) return;
        const modal = document.getElementById('mermaid-modal');
        const container = document.getElementById('modal-diagram-container');
        container.innerHTML = diagram.innerHTML;
        currentModalDiagram = container.firstElementChild;
        modal.style.display = 'flex';
        requestAnimationFrame(() => { modal.classList.add('modal-visible'); });
        document.body.style.overflow = 'hidden';
        ownPanX = 0; ownPanY = 0; ownScale = 1;
        if (currentModalDiagram && typeof Panzoom !== 'undefined') {
          panzoomInstance = Panzoom(currentModalDiagram, { maxScale: 5, minScale: 0.5, startScale: 1, canvas: true });
          const modalBody = document.querySelector('.modal-body');
          modalBody.addEventListener('wheel', handleModalWheel, { passive: false });
          modalBody.addEventListener('mouseenter', () => { isMouseOverModal = true; });
          modalBody.addEventListener('mouseleave', () => { isMouseOverModal = false; isPanMode = false; updateModeIndicator(); });
          updateZoomLevel();
        }
      }

      function closeMermaidModal() {
        const modal = document.getElementById('mermaid-modal');
        modal.classList.remove('modal-visible');
        setTimeout(() => {
          modal.style.display = 'none';
          document.body.style.overflow = '';
          currentModalDiagram = null;
          if (panzoomInstance) { panzoomInstance.destroy(); panzoomInstance = null; }
        }, 250);
      }

      function updateZoomLevel() {
        const el = document.getElementById('zoom-level');
        if (el) { const percentage = Math.round(ownScale * 100); el.textContent = percentage + '%'; }
      }

      function updateModeIndicator() {
        const container = document.getElementById('modal-diagram-container');
        if (container) {
          if (isPanMode) { container.classList.add('pan-mode'); }
          else { container.classList.remove('pan-mode'); }
        }
        const indicator = document.querySelector('.mode-indicator');
        if (!indicator) return;
        if (isPanMode) { indicator.textContent = 'Pan Mode'; indicator.classList.add('active'); indicator.style.opacity = '1'; }
        else { indicator.textContent = 'Zoom Mode (hold Space for Pan)'; indicator.classList.remove('active'); indicator.style.opacity = '0.5'; }
        showModeIndicator();
      }

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
        a.download = 'mermaid-diagram-' + Date.now() + '.svg';
        a.click();
        URL.revokeObjectURL(url);
      }

      function zoomToCenter(newScale) {
        if (!panzoomInstance || !currentModalDiagram) return;
        const rect = currentModalDiagram.parentElement.getBoundingClientRect();
        const centerX = rect.width / 2, centerY = rect.height / 2;
        const worldX = (centerX - ownPanX) / ownScale, worldY = (centerY - ownPanY) / ownScale;
        applyTransform(centerX - worldX * newScale, centerY - worldY * newScale, newScale);
        updateZoomLevel();
      }

      document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && currentModalDiagram) { closeMermaidModal(); return; }
        if (!isMouseOverModal || !panzoomInstance) return;
        if (e.key === ' ') { e.preventDefault(); isPanMode = true; updateModeIndicator(); }
      });
      document.addEventListener('keyup', (e) => {
        if (e.key === ' ') { isPanMode = false; updateModeIndicator(); }
      });

      const closeBtn = document.getElementById('modal-close');
      if (closeBtn) { closeBtn.addEventListener('click', closeMermaidModal); }
      const modalEl = document.getElementById('mermaid-modal');
      if (modalEl) {
        modalEl.addEventListener('click', (e) => { if (e.target.id === 'mermaid-modal') { closeMermaidModal(); } });
      }

      const modalDownload = document.getElementById('modal-download');
      if (modalDownload) {
        modalDownload.addEventListener('click', () => {
          if (!currentModalDiagram) return;
          const svg = currentModalDiagram.querySelector('svg') || currentModalDiagram;
          const svgData = svg.outerHTML;
          const blob = new Blob([svgData], { type: 'image/svg+xml' });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url;
          a.download = 'mermaid-diagram-' + Date.now() + '.svg';
          a.click();
          URL.revokeObjectURL(url);
        });
      }

      const zoomIn = document.getElementById('zoom-in');
      if (zoomIn) { zoomIn.addEventListener('click', () => { zoomToCenter(Math.max(0.5, Math.min(5, ownScale * 1.2))); }); }
      const zoomOut = document.getElementById('zoom-out');
      if (zoomOut) { zoomOut.addEventListener('click', () => { zoomToCenter(Math.max(0.5, Math.min(5, ownScale / 1.2))); }); }
      const zoomReset = document.getElementById('zoom-reset');
      if (zoomReset) { zoomReset.addEventListener('click', () => { applyTransform(0, 0, 1); updateZoomLevel(); }); }
"#;

const SCRIPT_FOOT: &str = r#"
      setTimeout(addFullscreenButtons, 500);
    });
  </script>
"#;
