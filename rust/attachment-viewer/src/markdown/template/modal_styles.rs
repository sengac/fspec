//! Modal + mermaid-wrapper CSS for the fullscreen diagram viewer — port of the
//! modal section of `src/server/templates/viewer-styles.ts`.
//!
//! Appended to the page `<style>` block by [`super::viewer_template`]. Uses the
//! same CSS variables defined in [`super::styles::STYLES`] so it follows the
//! active light/dark theme.

/// Mermaid wrapper, hover-button, fullscreen-modal and zoom-control styles.
pub const MODAL_STYLES: &str = r#"
    .mermaid-wrapper {
      position: relative;
      display: block;
      margin: 1.5rem 0;
      text-align: center;
      width: 100%;
    }
    .mermaid-wrapper > pre { display: block; width: 100%; }

    .mermaid-overlay {
      position: absolute;
      top: 0;
      right: 0;
      display: flex;
      gap: 0.5rem;
      padding: 0.5rem;
      opacity: 0;
      transition: opacity 0.2s ease;
      pointer-events: none;
    }
    .mermaid-wrapper:hover .mermaid-overlay { opacity: 1; pointer-events: auto; }

    .mermaid-fullscreen-btn,
    .mermaid-download-btn {
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      backdrop-filter: blur(4px);
      color: var(--button-text-color);
      padding: 0.5rem;
      border-radius: 0.375rem;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }
    .mermaid-fullscreen-btn:hover,
    .mermaid-download-btn:hover {
      background-color: var(--control-hover-bg);
      transform: scale(1.05);
    }
    .mermaid-fullscreen-btn svg,
    .mermaid-download-btn svg { width: 20px; height: 20px; }

    .modal-backdrop {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background-color: rgba(0, 0, 0, 0.85);
      backdrop-filter: blur(4px);
      z-index: 9999;
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 2rem;
      opacity: 0;
      transition: opacity 0.25s ease;
    }
    .modal-backdrop.modal-visible { opacity: 1; }

    .modal-container {
      width: 100%;
      height: 100%;
      max-width: 100%;
      max-height: 100%;
      background-color: var(--bg-color);
      border-radius: 0.75rem;
      box-shadow: 0 25px 50px -12px rgba(0, 0, 0, 0.5);
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }

    .modal-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 1rem 1.5rem;
      border-bottom: 1px solid var(--control-border);
      background-color: var(--code-bg);
    }
    .modal-title { margin: 0; font-size: 1.25rem; font-weight: 600; color: var(--text-color); }
    .modal-controls { display: flex; gap: 0.5rem; }

    .modal-button {
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      color: var(--button-text-color);
      padding: 0.5rem;
      border-radius: 0.375rem;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all 0.2s ease;
    }
    .modal-button:hover { background-color: var(--control-hover-bg); transform: scale(1.05); }
    .modal-button svg { width: 20px; height: 20px; }

    .modal-body { flex: 1; overflow: hidden; position: relative; background-color: var(--bg-color); }

    .diagram-container {
      width: 100%;
      height: 100%;
      display: flex;
      align-items: center;
      justify-content: center;
      overflow: hidden;
      cursor: grab;
    }
    .diagram-container:active { cursor: grabbing; }
    .diagram-container.pan-mode { cursor: move; }
    .diagram-container svg { max-width: none; max-height: none; }

    .zoom-controls {
      position: absolute;
      bottom: 1rem;
      right: 1rem;
      display: flex;
      gap: 0.5rem;
      align-items: center;
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      backdrop-filter: blur(4px);
      padding: 0.5rem;
      border-radius: 0.5rem;
    }
    .zoom-btn {
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      color: var(--button-text-color);
      width: 2rem;
      height: 2rem;
      border-radius: 0.375rem;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 1.25rem;
      font-weight: 600;
      transition: all 0.2s ease;
    }
    .zoom-btn:hover { background-color: var(--control-hover-bg); transform: scale(1.05); }
    .zoom-level { min-width: 3rem; text-align: center; font-size: 0.875rem; font-weight: 500; color: var(--text-color); }

    .mode-indicator {
      position: absolute;
      bottom: 1rem;
      left: 1rem;
      background-color: var(--control-bg);
      border: 1px solid var(--control-border);
      backdrop-filter: blur(4px);
      color: var(--button-text-color);
      padding: 0.5rem 1rem;
      border-radius: 0.5rem;
      font-size: 0.875rem;
      font-weight: 500;
      opacity: 0.5;
      transition: opacity 0.2s ease;
      pointer-events: none;
    }
    .mode-indicator.active { opacity: 1; }
"#;
