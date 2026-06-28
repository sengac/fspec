# RPC-378 — Fullscreen mermaid modal with Panzoom zoom/pan + SVG download

## Problem

The Rust viewer only emits `mermaid.initialize({ startOnLoad: true })`. The
TypeScript viewer ships a full interactive mermaid experience the Rust port lacks:
theme-aware init, per-diagram hover buttons (Fullscreen + Download SVG), and a
fullscreen modal with Panzoom-based zoom/pan, keyboard controls, a live zoom
percentage readout, and SVG download.

## Source of truth (TypeScript)

- `src/server/templates/viewer-scripts.ts`
  - `getMermaidScript()` (~13-50) — Mermaid ESM v11 from jsDelivr;
    `mermaid.initialize({ startOnLoad: true, securityLevel: 'loose',
    fontFamily: 'monospace', flowchart: { useMaxWidth, htmlLabels, curve: 'basis' } })`,
    theme `dark`|`default` from saved/preferred theme, `mermaid.run()` on
    `DOMContentLoaded`
  - `addFullscreenButtons()` (~208-256) — wraps each `pre.mermaid` in
    `.mermaid-wrapper`, injects Fullscreen + Download-SVG hover buttons
  - `getInteractionScript()` (~191-668) — the modal:
    - `openMermaidModal(index)` clones the rendered SVG into the modal (~259)
    - `closeMermaidModal()` (~318); close on backdrop click, X button, **ESC**
    - **Zoom**: Panzoom v4.5.1; custom wheel handler `handleModalWheel` (~343)
      with **cursor-centered** zoom (locks zoom point per gesture, ~387-456);
      buttons zoom-in ×1.2 / zoom-out ÷1.2 / reset; clamped **0.5×–5×**; live `%`
      via `updateZoomLevel()` (~500)
    - **Pan**: hold **Space** to enter pan mode (~359-382, 482-497); horizontal
      scroll pans in zoom mode (~460); mode indicator UI (~508)
    - **Download SVG**: `downloadDiagram(index)` serializes SVG → Blob → download
      (~542-560); modal download button (~572)
- Panzoom CDN script: `viewer-template.ts:106`
- Companion TS feature already specified:
  `spec/features/fullscreen-mermaid-diagram-viewer-with-zoom-and-pan-controls.feature`

## Behavior to replicate

1. **Theme-aware init**: mermaid initialized with `securityLevel: 'loose'`,
   monospace font, flowchart opts, theme following the selected light/dark theme,
   `mermaid.run()` on load.
2. **Hover buttons**: every `pre.mermaid` wrapped in `.mermaid-wrapper` with
   Fullscreen + Download-SVG buttons appearing on hover.
3. **Modal open/close**: clicking Fullscreen clones that diagram's SVG into a
   fullscreen modal; closes via X, backdrop click, or ESC.
4. **Zoom**: Panzoom loaded from CDN; wheel zoom is cursor-centered, clamped
   0.5×–5×; zoom-in/out/reset buttons; live percentage display.
5. **Pan**: Space enters pan mode; drag/scroll pans; mode indicator shows current
   mode.
6. **Download**: serialize the modal SVG to a Blob and trigger a `.svg` download;
   also available from per-diagram hover button.

## Implementation approach (Rust)

- Build on the `markdown/template/scripts.rs` module created in RPC-377.
- Add the Panzoom CDN `<script>` (v4.5.1) and the mermaid modal markup to the body
  emitted by `viewer_template`.
- Emit `getMermaidScript`, `addFullscreenButtons`, and `getInteractionScript`
  equivalents as Rust string constants reproducing the TS client JS (CDN URLs,
  clamp bounds 0.5–5, ×1.2 factors, localStorage-driven theme, ESC/Space handlers,
  SVG Blob download).
- Keep every file **< 300 lines**; split the modal JS/CSS into a dedicated
  submodule (e.g. `scripts/mermaid_modal.rs` + modal styles) if needed.

## Acceptance criteria (for Example Mapping → scenarios)

1. The served HTML loads Mermaid ESM v11 and initializes with
   `securityLevel: 'loose'`, monospace font, and a theme derived from the saved
   theme, calling `mermaid.run()` on load.
2. The HTML loads the Panzoom v4.5.1 CDN script.
3. The emitted client JS wraps each `pre.mermaid` in a `.mermaid-wrapper` and adds
   Fullscreen + Download-SVG buttons.
4. The emitted JS defines an open/close fullscreen modal flow, closing on ESC and
   backdrop click.
5. The zoom logic clamps scale to 0.5×–5× and exposes zoom-in (×1.2) / zoom-out
   (÷1.2) / reset with a live percentage readout.
6. The JS implements Space-to-pan and an SVG Blob download.
7. The existing markdown/mermaid `<pre class="mermaid">` server-render scenarios
   still pass; all files remain under 300 lines.

## Notes / constraints

- Rust tests assert on the **emitted HTML/JS string** (CDN URLs, clamp constants,
  handler wiring, localStorage theme key), not browser execution.
- Must not regress RPC-376 (anchors) or RPC-377 (Prism/theme/fonts).
- Preserve the fspec.pro axum architecture; no public-API breakage.
- Depends on RPC-377.
