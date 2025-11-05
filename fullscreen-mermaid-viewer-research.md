# Fullscreen Mermaid Diagram Viewer with Zoom/Pan - Technical Research

## Executive Summary

This document provides comprehensive research on implementing a fullscreen mermaid diagram viewer with zoom/pan capabilities in fspec, based on the successful implementation in mindstrike. The goal is to add:

1. **Fullscreen Modal**: Click a button on any mermaid diagram to open it in a fullscreen overlay
2. **Zoom Controls**: Mouse wheel zoom centered on cursor position (like mindstrike's mindmap)
3. **Pan Controls**: Click-drag panning and modifier key (Space/Shift/Alt) to toggle pan mode
4. **Visual Feedback**: On-screen indicator showing current mode (Zoom Mode / Pan Mode)

---

## Table of Contents

1. [Architecture Comparison](#architecture-comparison)
2. [Mindstrike Implementation Details](#mindstrike-implementation-details)
3. [fspec Current Implementation](#fspec-current-implementation)
4. [Proposed Implementation Strategy](#proposed-implementation-strategy)
5. [Key Technical Decisions](#key-technical-decisions)
6. [Implementation Checklist](#implementation-checklist)

---

## Architecture Comparison

### Mindstrike (React SPA)
- **Framework**: React 18.3.1 with TypeScript
- **Rendering**: Client-side React components
- **Mermaid**: Rendered in React components with custom hooks
- **Modals**: React Portal (createPortal) for fullscreen overlays
- **Zoom/Pan**: ReactFlow library (v11.11.4) with custom wheel event handlers
- **State**: Zustand for global state management
- **Styling**: Tailwind CSS

### fspec (TUI + Web Viewer)
- **Framework**: Dual architecture
  - TUI: Ink (React for CLI) in terminal
  - Viewer: Express server serving static HTML
- **Rendering**: Server-side markdown-to-HTML conversion
- **Mermaid**: Client-side rendering with Mermaid.js v11.12.0 from CDN
- **Modals**: Browser DOM (no framework in viewer)
- **Zoom/Pan**: Not implemented yet
- **State**: LocalStorage for viewer preferences (font size, theme)
- **Styling**: Vanilla CSS in template strings

**Key Difference**: fspec viewer is a **vanilla JavaScript web page**, not a React app. We cannot directly port React components. Instead, we must adapt the patterns using vanilla JS.

---

## Mindstrike Implementation Details

### 1. Fullscreen Mermaid Modal

**Component**: `MermaidModal.tsx` ([source](file:///home/rquast/projects/mindstrike/src/components/MermaidModal.tsx))

**Architecture**:
```typescript
// React Portal to render outside component hierarchy
createPortal(
  <div className="fixed inset-0 z-[9999] bg-black bg-opacity-75 backdrop-blur-sm">
    {/* Header with title, download, close buttons */}
    <header className="flex items-center justify-between p-4 bg-slate-800">
      <h2>Full Screen Diagram</h2>
      <button onClick={handleDownload}>Download SVG</button>
      <button onClick={onClose}>×</button>
    </header>

    {/* Diagram container */}
    <div className="flex-1 overflow-auto p-4">
      {loading ? <Spinner /> : <div className="mermaid">{code}</div>}
    </div>
  </div>,
  document.body
)
```

**Key Features**:
- **Z-index**: 9999 to appear above everything
- **Backdrop**: Semi-transparent black with blur (`backdrop-blur-sm`)
- **ESC handler**: Closes modal on Escape key
- **Body scroll lock**: `document.body.style.overflow = 'hidden'` when open
- **Animation**: 250ms scale/opacity transition using custom hook
- **Download**: Exports SVG using `<a download>` pattern

**Animation Hook**: `useDialogAnimation.ts` ([source](file:///home/rquast/projects/mindstrike/src/hooks/useDialogAnimation.ts))
```typescript
// Two-stage rendering for smooth transitions
const [shouldRender, setShouldRender] = useState(isOpen);
const [isVisible, setIsVisible] = useState(false);

useEffect(() => {
  if (isOpen) {
    setShouldRender(true);           // Mount component
    setTimeout(() => setIsVisible(true), 10);  // Trigger CSS transition
  } else {
    setIsVisible(false);              // Fade out
    setTimeout(() => setShouldRender(false), 250);  // Unmount after animation
  }
}, [isOpen]);

return { shouldRender, isVisible };
```

**Integration**: `MarkdownViewer.tsx` ([source](file:///home/rquast/projects/mindstrike/src/components/MarkdownViewer.tsx))
```typescript
// Hover controls on each mermaid diagram
<div className="relative group">
  <div className="mermaid" id={uniqueId}>{code}</div>

  {/* Hover overlay with buttons */}
  <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity">
    <button onClick={() => openModal(code)}>
      <Maximize2 className="w-5 h-5" />
    </button>
    <button onClick={() => downloadSVG(uniqueId)}>
      <Download className="w-5 h-5" />
    </button>
  </div>
</div>
```

### 2. MindMap Zoom/Pan System

**Component**: `MindMap.tsx` ([source](file:///home/rquast/projects/mindstrike/src/mindmaps/components/MindMap.tsx))

**Custom Wheel Handler** (lines 940-1041):
```typescript
const handleWheel = (event: WheelEvent) => {
  event.preventDefault();

  const deltaX = event.deltaX;
  const deltaY = event.deltaY;
  const deltaMode = event.deltaMode;

  // Check if pan modifier is held (Space/Shift/Alt)
  const isPanModifierHeld =
    (panModifierKey === 'Space' && event.code === 'Space') ||
    (panModifierKey === 'Shift' && event.shiftKey) ||
    (panModifierKey === 'Alt' && event.altKey) ||
    isPanModeActive;

  let newZoom = viewport.zoom;
  let newX = viewport.x;
  let newY = viewport.y;

  // VERTICAL SCROLL BEHAVIOR
  if (Math.abs(deltaY) > 0) {
    if (isPanModifierHeld) {
      // Pan mode: vertical scroll = vertical pan
      newY += deltaY;
    } else {
      // Zoom mode: vertical scroll = zoom (ReactFlow formula)
      const zoomDelta = -deltaY * (deltaMode === 1 ? 0.05 : deltaMode ? 1 : 0.002);
      newZoom = viewport.zoom * Math.pow(2, zoomDelta);
      newZoom = Math.max(0.1, Math.min(2, newZoom));

      // Zoom to cursor position (keep point under mouse fixed)
      const bounds = containerRef.current.getBoundingClientRect();
      const mouseX = event.clientX - bounds.left;
      const mouseY = event.clientY - bounds.top;

      const zoomRatio = newZoom / viewport.zoom;
      newX = viewport.x + (mouseX - viewport.x) * (1 - zoomRatio);
      newY = viewport.y + (mouseY - viewport.y) * (1 - zoomRatio);
    }
  }

  // HORIZONTAL SCROLL BEHAVIOR (always pan)
  if (Math.abs(deltaX) > 0) {
    newX += deltaX;
  }

  // Update viewport with new transform
  reactFlowInstance.setViewport({ x: newX, y: newY, zoom: newZoom }, { duration: 0 });
};
```

**Pan Modifier Key System** (lines 880-938):
```typescript
const [isPanModeActive, setIsPanModeActive] = useState(false);
const [isMouseOverContainer, setIsMouseOverContainer] = useState(false);
const panModifierKey = keyBindings.panModeModifier || 'Space';

// Global keyboard listeners
useEffect(() => {
  const handleKeyDown = (event: KeyboardEvent) => {
    // Only activate when mouse is over MindMap container
    if (!isMouseOverContainer) return;

    if (event.key === ' ' && panModifierKey === 'Space') {
      event.preventDefault();
      setIsPanModeActive(true);
    } else if (event.shiftKey && panModifierKey === 'Shift') {
      setIsPanModeActive(true);
    } else if (event.altKey && panModifierKey === 'Alt') {
      setIsPanModeActive(true);
    }
  };

  const handleKeyUp = (event: KeyboardEvent) => {
    if (event.key === ' ' || event.key === 'Shift' || event.key === 'Alt') {
      setIsPanModeActive(false);
    }
  };

  window.addEventListener('keydown', handleKeyDown);
  window.addEventListener('keyup', handleKeyUp);
  return () => {
    window.removeEventListener('keydown', handleKeyDown);
    window.removeEventListener('keyup', handleKeyUp);
  };
}, [isMouseOverContainer, panModifierKey]);

// Mouse enter/leave handlers
const handleMouseEnter = () => setIsMouseOverContainer(true);
const handleMouseLeave = () => {
  setIsMouseOverContainer(false);
  setIsPanModeActive(false);  // Clear pan mode when leaving
};
```

**Visual Indicator**: `ScrollModeOverlay.tsx` ([source](file:///home/rquast/projects/mindstrike/src/mindmaps/components/ScrollModeOverlay.tsx))
```typescript
// Bottom-left corner overlay
<div
  className={`
    absolute bottom-4 left-4 z-[1000]
    px-3 py-1.5 rounded-lg shadow-lg
    bg-slate-800 dark:bg-slate-900 text-white
    text-sm font-medium pointer-events-none
    transition-opacity duration-200
    ${isActive ? 'opacity-100' : 'opacity-50'}
  `}
>
  {isPanModeActive ? (
    'Pan Mode'
  ) : (
    `Zoom Mode (hold ${panModifierKeyName} for Pan Mode)`
  )}
</div>
```

**Opacity Control**:
- Semi-transparent (50%) when idle
- Fully opaque (100%) when pan mode active OR scrolling
- 500ms fade delay after last interaction

**ReactFlow Configuration**:
```typescript
<ReactFlow
  nodes={nodes}
  edges={edges}
  panOnScroll={false}      // Disabled - using custom handler
  zoomOnScroll={false}     // Disabled - using custom handler
  zoomOnPinch={true}       // Keep touchpad pinch-to-zoom
  zoomOnDoubleClick={false}
  minZoom={0.1}
  maxZoom={2}
/>
```

**Wheel Event Attachment**:
```typescript
useEffect(() => {
  const container = containerRef.current?.querySelector('.react-flow');
  if (!container) return;

  container.addEventListener('wheel', handleWheel, { passive: false });
  return () => container.removeEventListener('wheel', handleWheel);
}, [viewport, isPanModeActive]);
```

### 3. Mermaid Rendering System

**Global Initialization**: `mermaidRenderer.ts` ([source](file:///home/rquast/projects/mindstrike/src/utils/mermaidRenderer.ts))
```typescript
let mermaidInitialized = false;
let renderQueue: Promise<void> = Promise.resolve();

export async function initializeMermaid(): Promise<void> {
  if (mermaidInitialized) return;

  const config = getMermaidConfig();
  mermaid.initialize(config);
  mermaidInitialized = true;

  // MutationObserver to auto-render new diagrams
  const observer = new MutationObserver((mutations) => {
    const hasMermaidNodes = mutations.some(m =>
      Array.from(m.addedNodes).some(node =>
        node instanceof Element &&
        (node.classList.contains('mermaid') || node.querySelector('.mermaid'))
      )
    );
    if (hasMermaidNodes) {
      renderMermaidDiagrams();
    }
  });

  observer.observe(document.body, { childList: true, subtree: true });
}

export function renderMermaidDiagrams(): void {
  // Queue rendering to prevent race conditions
  renderQueue = renderQueue.then(async () => {
    const elements = document.querySelectorAll('.mermaid:not([data-processed])');
    for (const el of elements) {
      try {
        const code = cleanMermaidCode(el.textContent || '');
        const { svg } = await mermaid.render(`mermaid-${Date.now()}`, code);
        el.innerHTML = svg;
        el.setAttribute('data-processed', 'true');
      } catch (error) {
        console.error('Mermaid rendering failed:', error);
        el.textContent = 'Error rendering diagram';
      }
    }
  });
}
```

**Dark Theme Config**: `mermaidConfig.ts` ([source](file:///home/rquast/projects/mindstrike/src/utils/mermaidConfig.ts))
```typescript
export function getMermaidConfig() {
  return {
    startOnLoad: false,  // Manual rendering
    theme: 'base',
    securityLevel: 'loose',
    themeVariables: {
      primaryColor: '#374151',      // gray-700
      primaryTextColor: '#f3f4f6',  // gray-100
      primaryBorderColor: '#1f2937', // gray-800
      lineColor: '#9ca3af',         // gray-400
      secondaryColor: '#1f2937',
      tertiaryColor: '#111827',     // gray-900
      background: '#111827',
      mainBkg: '#1f2937',
      secondBkg: '#374151',
      // ... more theme variables
    }
  };
}
```

---

## fspec Current Implementation

### 1. Attachment Viewer Architecture

**Server**: `attachment-server.ts` ([source](file:///home/rquast/projects/fspec/src/server/attachment-server.ts))
```typescript
export function startAttachmentServer(): Promise<number> {
  const app = express();
  const server = app.listen(0);  // Random port to avoid conflicts

  app.get('/view/:path(*)', async (req, res) => {
    const filePath = path.join(process.cwd(), req.params.path);
    const markdown = await fs.readFile(filePath, 'utf-8');
    const html = renderMarkdownToHtml(markdown, filePath);
    const fullHtml = generateViewerTemplate(html, req.query.theme as string);
    res.send(fullHtml);
  });

  return Promise.resolve(server.address().port);
}
```

**HTML Template**: `viewer-template.ts` ([source](file:///home/rquast/projects/fspec/src/server/templates/viewer-template.ts))
```typescript
export function generateViewerTemplate(content: string, theme?: string): string {
  return `<!DOCTYPE html>
<html lang="en" data-theme="${theme || 'dark'}">
<head>
  <meta charset="UTF-8">
  <title>fspec Attachment Viewer</title>
  <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/prismjs@1.29.0/themes/prism-tomorrow.min.css">
  <script src="https://cdn.jsdelivr.net/npm/mermaid@11.12.0/dist/mermaid.min.js"></script>
  <style>${getViewerStyles()}</style>
</head>
<body>
  <div class="container">
    <!-- Top-right controls -->
    <div class="controls">
      <button id="theme-toggle">🌙 / ☀️</button>
      <button id="font-decrease">−</button>
      <span id="font-size-display">16px</span>
      <button id="font-increase">+</button>
    </div>

    <!-- Markdown content (includes mermaid blocks) -->
    <div class="content">${content}</div>
  </div>

  <script>${getViewerScripts()}</script>
</body>
</html>`;
}
```

**Mermaid Initialization**: `viewer-scripts.ts` ([source](file:///home/rquast/projects/fspec/src/server/templates/viewer-scripts.ts), lines 14-50)
```javascript
// Initialize Mermaid
const isDark = document.documentElement.getAttribute('data-theme') === 'dark';
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

// Render diagrams on page load
window.addEventListener('DOMContentLoaded', () => {
  mermaid.run();
});

// Re-render on theme change
function updateMermaidTheme() {
  const isDark = document.documentElement.getAttribute('data-theme') === 'dark';
  mermaid.initialize({
    theme: isDark ? 'dark' : 'default',
    // ... same config
  });
  mermaid.run();  // Re-render all diagrams
}
```

**Font Size Controls** (lines 142-189):
```javascript
const fontControls = {
  currentSize: 16,
  minSize: 10,
  maxSize: 24,
  step: 2,

  init() {
    this.currentSize = parseInt(localStorage.getItem('fontSize') || '16');
    this.updateFontSize();

    document.getElementById('font-increase').addEventListener('click', () => {
      if (this.currentSize < this.maxSize) {
        this.currentSize += this.step;
        this.updateFontSize();
      }
    });

    document.getElementById('font-decrease').addEventListener('click', () => {
      if (this.currentSize > this.minSize) {
        this.currentSize -= this.step;
        this.updateFontSize();
      }
    });
  },

  updateFontSize() {
    const scale = this.currentSize / 16;
    document.documentElement.style.setProperty('--base-font-size', `${this.currentSize}px`);
    document.documentElement.style.setProperty('--font-scale', scale.toString());
    localStorage.setItem('fontSize', this.currentSize.toString());

    // Update display
    document.getElementById('font-size-display').textContent = `${this.currentSize}px`;
  }
};

fontControls.init();
```

**CSS Styling**: `viewer-styles.ts` ([source](file:///home/rquast/projects/fspec/src/server/templates/viewer-styles.ts))
```css
:root {
  --base-font-size: 16px;
  --font-scale: 1;
}

body {
  font-family: 'Segoe UI', system-ui, sans-serif;
  font-size: var(--base-font-size);
  line-height: 1.6;
  background: var(--bg-color);
  color: var(--text-color);
}

.container {
  max-width: calc(900px * var(--font-scale));
  margin: 0 auto;
  padding: 2rem;
}

.content pre.mermaid {
  background: var(--code-bg);
  border-radius: 8px;
  padding: 1rem;
  overflow-x: auto;
}

/* Dark theme */
[data-theme="dark"] {
  --bg-color: #1a1a1a;
  --text-color: #e0e0e0;
  --code-bg: #2d2d2d;
}

/* Light theme */
[data-theme="light"] {
  --bg-color: #ffffff;
  --text-color: #333333;
  --code-bg: #f5f5f5;
}
```

### 2. TUI Integration

**BoardView Component**: `BoardView.tsx` ([source](file:///home/rquast/projects/fspec/src/tui/components/BoardView.tsx))

**Server Lifecycle** (lines 213-245):
```typescript
const [attachmentServerPort, setAttachmentServerPort] = useState<number | null>(null);

useEffect(() => {
  // Start server on mount
  startAttachmentServer()
    .then(port => {
      setAttachmentServerPort(port);
      debug(`Attachment server started on port ${port}`);
    })
    .catch(err => {
      debug(`Failed to start attachment server: ${err.message}`);
    });

  // Stop server on unmount
  return () => {
    if (attachmentServerPort) {
      stopAttachmentServer();
      debug('Attachment server stopped');
    }
  };
}, []);
```

**Attachment Dialog** (lines 288-293):
```typescript
if (input === 'a' && viewMode === 'board' && !showAttachmentDialog) {
  setShowAttachmentDialog(true);
  return;
}
```

**Open Attachment** (lines 463-480):
```typescript
const handleOpenAttachment = async (attachment: string) => {
  const absolutePath = path.resolve(process.cwd(), attachment);

  let url: string;
  if (attachmentServerPort) {
    url = `http://localhost:${attachmentServerPort}/view/${attachment}`;
  } else {
    // Fallback to file:// URL if server not running
    url = `file://${absolutePath}`;
  }

  try {
    await openInBrowser(url);
  } catch (err) {
    debug(`Failed to open attachment: ${err.message}`);
  }

  setShowAttachmentDialog(false);
};
```

**AttachmentDialog Component**: `AttachmentDialog.tsx` ([source](file:///home/rquast/projects/fspec/src/tui/components/AttachmentDialog.tsx))
- Lists all attachments from selected work unit
- Keyboard navigation with up/down arrows
- Enter key to open selected attachment
- ESC key to close dialog

### 3. Existing Fullscreen Components

**FullScreenWrapper**: `FullScreenWrapper.tsx` ([source](file:///home/rquast/projects/fspec/src/tui/components/FullScreenWrapper.tsx))
```typescript
export const FullScreenWrapper: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  useEffect(() => {
    // Clear screen on mount
    process.stdout.write('\x1Bc');
  }, []);

  return (
    <Box width={process.stdout.columns} height={process.stdout.rows}>
      {children}
    </Box>
  );
};
```

**Usage in BoardView** (lines 628-650):
```typescript
if (viewMode === 'detail' && selectedWorkUnit) {
  return (
    <FullScreenWrapper>
      <WorkUnitDetailView
        workUnit={selectedWorkUnit}
        onClose={() => setViewMode('board')}
      />
    </FullScreenWrapper>
  );
}

if (viewMode === 'checkpoint-viewer') {
  return (
    <FullScreenWrapper>
      <CheckpointViewer onClose={() => setViewMode('board')} />
    </FullScreenWrapper>
  );
}
```

---

## Proposed Implementation Strategy

### Phase 1: Fullscreen Modal for Mermaid Diagrams

**Goal**: Add a "Fullscreen" button to each mermaid diagram that opens it in a fullscreen overlay.

**Files to Modify**:
1. `viewer-template.ts` - Add modal HTML structure
2. `viewer-scripts.ts` - Add modal open/close logic and button injection
3. `viewer-styles.ts` - Add modal styles (backdrop, animations, layout)

**Implementation Steps**:

#### Step 1.1: Add Modal HTML Structure

In `viewer-template.ts`, add modal container at end of `<body>`:
```html
<!-- Fullscreen Mermaid Modal -->
<div id="mermaid-modal" class="modal-backdrop" style="display: none;">
  <div class="modal-container">
    <header class="modal-header">
      <h2 class="modal-title">Diagram Fullscreen View</h2>
      <div class="modal-controls">
        <button id="modal-download" class="modal-button" title="Download SVG">
          <svg><!-- Download icon --></svg>
        </button>
        <button id="modal-close" class="modal-button" title="Close (ESC)">
          <svg><!-- Close icon --></svg>
        </button>
      </div>
    </header>
    <div class="modal-body">
      <div id="modal-diagram-container" class="diagram-container"></div>
    </div>
  </div>
</div>
```

#### Step 1.2: Add Fullscreen Buttons to Diagrams

In `viewer-scripts.ts`, add function to inject buttons:
```javascript
function addFullscreenButtons() {
  const diagrams = document.querySelectorAll('pre.mermaid');

  diagrams.forEach((diagram, index) => {
    // Skip if already has button
    if (diagram.parentElement?.classList.contains('mermaid-wrapper')) return;

    // Wrap diagram in container
    const wrapper = document.createElement('div');
    wrapper.className = 'mermaid-wrapper';
    diagram.parentNode.insertBefore(wrapper, diagram);
    wrapper.appendChild(diagram);

    // Add button overlay
    const overlay = document.createElement('div');
    overlay.className = 'mermaid-overlay';
    overlay.innerHTML = `
      <button class="mermaid-fullscreen-btn" data-index="${index}" title="Fullscreen">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M8 3H5a2 2 0 0 0-2 2v3m18 0V5a2 2 0 0 0-2-2h-3m0 18h3a2 2 0 0 0 2-2v-3M3 16v3a2 2 0 0 0 2 2h3"/>
        </svg>
      </button>
    `;
    wrapper.appendChild(overlay);
  });

  // Attach event listeners
  document.querySelectorAll('.mermaid-fullscreen-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      const index = parseInt(e.currentTarget.getAttribute('data-index'));
      openMermaidModal(index);
    });
  });
}

// Call after mermaid renders
window.addEventListener('DOMContentLoaded', () => {
  mermaid.run().then(() => {
    addFullscreenButtons();
  });
});
```

#### Step 1.3: Modal Open/Close Logic

In `viewer-scripts.ts`, add modal functions:
```javascript
let currentModalDiagram = null;

function openMermaidModal(index) {
  const diagrams = document.querySelectorAll('pre.mermaid');
  const diagram = diagrams[index];
  if (!diagram) return;

  // Clone diagram content to modal
  const modal = document.getElementById('mermaid-modal');
  const container = document.getElementById('modal-diagram-container');
  container.innerHTML = diagram.innerHTML;

  // Store reference for download
  currentModalDiagram = container.firstElementChild;

  // Show modal with animation
  modal.style.display = 'flex';
  requestAnimationFrame(() => {
    modal.classList.add('modal-visible');
  });

  // Lock body scroll
  document.body.style.overflow = 'hidden';
}

function closeMermaidModal() {
  const modal = document.getElementById('mermaid-modal');
  modal.classList.remove('modal-visible');

  // Wait for fade animation, then hide
  setTimeout(() => {
    modal.style.display = 'none';
    document.body.style.overflow = '';
    currentModalDiagram = null;
  }, 250);
}

// Event listeners
document.getElementById('modal-close').addEventListener('click', closeMermaidModal);
document.getElementById('mermaid-modal').addEventListener('click', (e) => {
  // Close on backdrop click
  if (e.target.id === 'mermaid-modal') {
    closeMermaidModal();
  }
});

// ESC key handler
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && currentModalDiagram) {
    closeMermaidModal();
  }
});

// Download button
document.getElementById('modal-download').addEventListener('click', () => {
  if (!currentModalDiagram) return;

  const svgData = currentModalDiagram.outerHTML;
  const blob = new Blob([svgData], { type: 'image/svg+xml' });
  const url = URL.createObjectURL(blob);

  const a = document.createElement('a');
  a.href = url;
  a.download = `mermaid-diagram-${Date.now()}.svg`;
  a.click();

  URL.revokeObjectURL(url);
});
```

#### Step 1.4: Modal Styles

In `viewer-styles.ts`, add modal CSS:
```css
/* Mermaid diagram wrapper (for hover buttons) */
.mermaid-wrapper {
  position: relative;
  margin: 1rem 0;
}

.mermaid-overlay {
  position: absolute;
  top: 0.5rem;
  right: 0.5rem;
  opacity: 0;
  transition: opacity 0.2s ease;
  pointer-events: none;
}

.mermaid-wrapper:hover .mermaid-overlay {
  opacity: 1;
  pointer-events: auto;
}

.mermaid-fullscreen-btn {
  background: rgba(0, 0, 0, 0.7);
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 6px;
  padding: 0.5rem;
  cursor: pointer;
  color: white;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.mermaid-fullscreen-btn:hover {
  background: rgba(0, 0, 0, 0.9);
  transform: scale(1.1);
}

/* Modal backdrop */
.modal-backdrop {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.85);
  backdrop-filter: blur(4px);
  z-index: 9999;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.25s ease;
}

.modal-backdrop.modal-visible {
  opacity: 1;
}

/* Modal container */
.modal-container {
  width: 95vw;
  height: 95vh;
  background: var(--bg-color);
  border-radius: 12px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  transform: scale(0.95);
  transition: transform 0.25s ease;
}

.modal-backdrop.modal-visible .modal-container {
  transform: scale(1);
}

/* Modal header */
.modal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.5rem;
  background: var(--code-bg);
  border-bottom: 1px solid var(--border-color);
}

.modal-title {
  margin: 0;
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--text-color);
}

.modal-controls {
  display: flex;
  gap: 0.5rem;
}

.modal-button {
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 0.5rem;
  cursor: pointer;
  color: var(--text-color);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.modal-button:hover {
  background: var(--hover-bg);
  transform: scale(1.1);
}

/* Modal body */
.modal-body {
  flex: 1;
  overflow: auto;
  padding: 2rem;
  display: flex;
  align-items: center;
  justify-content: center;
}

.diagram-container {
  max-width: 100%;
  max-height: 100%;
}

.diagram-container svg {
  max-width: 100%;
  max-height: 100%;
  height: auto;
}

/* Dark theme colors */
[data-theme="dark"] {
  --border-color: #444;
  --hover-bg: rgba(255, 255, 255, 0.1);
}

/* Light theme colors */
[data-theme="light"] {
  --border-color: #ddd;
  --hover-bg: rgba(0, 0, 0, 0.05);
}
```

---

### Phase 2: Zoom and Pan Controls

**Goal**: Add mouse wheel zoom (centered on cursor) and pan controls (modifier key + drag) to the fullscreen modal.

**Technology Choice**: Use **panzoom.js** library (11KB, no dependencies, MIT license)
- Simpler than ReactFlow for vanilla JS
- Proven zoom-to-cursor algorithm
- Built-in mouse wheel and drag handlers
- Touch gesture support

**Files to Modify**:
1. `viewer-template.ts` - Add panzoom CDN script
2. `viewer-scripts.ts` - Initialize panzoom on modal open
3. `viewer-styles.ts` - Add zoom control buttons and mode indicator

**Implementation Steps**:

#### Step 2.1: Add Panzoom Library

In `viewer-template.ts`, add CDN link:
```html
<script src="https://cdn.jsdelivr.net/npm/@panzoom/panzoom@4.5.1/dist/panzoom.min.js"></script>
```

#### Step 2.2: Panzoom Initialization

In `viewer-scripts.ts`, modify `openMermaidModal`:
```javascript
let panzoomInstance = null;
let isPanMode = false;
let panModifierKey = 'Space';  // Configurable: Space, Shift, Alt

function openMermaidModal(index) {
  // ... existing code to show modal ...

  // Initialize panzoom on diagram container
  const diagramContainer = document.getElementById('modal-diagram-container');
  panzoomInstance = Panzoom(diagramContainer.firstElementChild, {
    maxScale: 5,
    minScale: 0.5,
    startScale: 1,
    cursor: 'default',  // Don't show move cursor by default
    canvas: true,       // Use canvas-based pan (smooth)
  });

  // Disable default zoom behavior
  const modalBody = document.querySelector('.modal-body');
  modalBody.addEventListener('wheel', handleModalWheel, { passive: false });

  // Add zoom controls
  addZoomControls();

  // Show mode indicator
  updateModeIndicator();
}

function closeMermaidModal() {
  // ... existing code ...

  // Cleanup panzoom
  if (panzoomInstance) {
    panzoomInstance.destroy();
    panzoomInstance = null;
  }

  // Remove zoom controls
  document.querySelector('.zoom-controls')?.remove();
  document.querySelector('.mode-indicator')?.remove();
}
```

#### Step 2.3: Custom Wheel Handler with Pan Modifier

In `viewer-scripts.ts`, add wheel handler:
```javascript
let isMouseOverModal = false;

function handleModalWheel(event) {
  if (!panzoomInstance) return;

  event.preventDefault();

  const deltaX = event.deltaX;
  const deltaY = event.deltaY;
  const deltaMode = event.deltaMode;

  // Check if pan modifier is held
  const isPanModifierHeld =
    (panModifierKey === 'Space' && isPanMode) ||
    (panModifierKey === 'Shift' && event.shiftKey) ||
    (panModifierKey === 'Alt' && event.altKey);

  // VERTICAL SCROLL
  if (Math.abs(deltaY) > 0) {
    if (isPanModifierHeld) {
      // Pan mode: vertical scroll = vertical pan
      const scale = panzoomInstance.getScale();
      panzoomInstance.pan(0, -deltaY / scale);
    } else {
      // Zoom mode: vertical scroll = zoom (centered on cursor)
      const zoomDelta = -deltaY * (deltaMode === 1 ? 0.05 : deltaMode ? 1 : 0.002);
      const scale = panzoomInstance.getScale();
      const newScale = scale * Math.pow(2, zoomDelta);

      // Get mouse position relative to modal body
      const modalBody = document.querySelector('.modal-body');
      const rect = modalBody.getBoundingClientRect();
      const clientX = event.clientX - rect.left;
      const clientY = event.clientY - rect.top;

      panzoomInstance.zoomToPoint(newScale, { clientX, clientY });
    }
  }

  // HORIZONTAL SCROLL (always pan)
  if (Math.abs(deltaX) > 0) {
    const scale = panzoomInstance.getScale();
    panzoomInstance.pan(-deltaX / scale, 0);
  }

  // Update indicator opacity on scroll
  showModeIndicator();
}

// Pan modifier key tracking
document.addEventListener('keydown', (e) => {
  if (!isMouseOverModal || !panzoomInstance) return;

  if (e.key === ' ' && panModifierKey === 'Space') {
    e.preventDefault();
    isPanMode = true;
    updateModeIndicator();
  } else if (e.key === 'Shift' && panModifierKey === 'Shift') {
    isPanMode = true;
    updateModeIndicator();
  } else if (e.key === 'Alt' && panModifierKey === 'Alt') {
    isPanMode = true;
    updateModeIndicator();
  }
});

document.addEventListener('keyup', (e) => {
  if (e.key === ' ' || e.key === 'Shift' || e.key === 'Alt') {
    isPanMode = false;
    updateModeIndicator();
  }
});

// Mouse tracking for scope
document.querySelector('.modal-body')?.addEventListener('mouseenter', () => {
  isMouseOverModal = true;
});

document.querySelector('.modal-body')?.addEventListener('mouseleave', () => {
  isMouseOverModal = false;
  isPanMode = false;
  updateModeIndicator();
});
```

#### Step 2.4: Zoom Control Buttons

In `viewer-scripts.ts`, add zoom controls:
```javascript
function addZoomControls() {
  const modalContainer = document.querySelector('.modal-container');

  const controls = document.createElement('div');
  controls.className = 'zoom-controls';
  controls.innerHTML = `
    <button id="zoom-in" class="zoom-btn" title="Zoom In (+)">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"/>
        <line x1="11" y1="8" x2="11" y2="14"/>
        <line x1="8" y1="11" x2="14" y2="11"/>
        <line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
    </button>
    <button id="zoom-out" class="zoom-btn" title="Zoom Out (-)">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="11" cy="11" r="8"/>
        <line x1="8" y1="11" x2="14" y2="11"/>
        <line x1="21" y1="21" x2="16.65" y2="16.65"/>
      </svg>
    </button>
    <button id="zoom-reset" class="zoom-btn" title="Reset Zoom (0)">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M1 4v6h6M23 20v-6h-6"/>
        <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15"/>
      </svg>
    </button>
    <span id="zoom-level" class="zoom-level">100%</span>
  `;

  modalContainer.appendChild(controls);

  // Event listeners
  document.getElementById('zoom-in').addEventListener('click', () => {
    panzoomInstance.zoomIn();
    updateZoomLevel();
  });

  document.getElementById('zoom-out').addEventListener('click', () => {
    panzoomInstance.zoomOut();
    updateZoomLevel();
  });

  document.getElementById('zoom-reset').addEventListener('click', () => {
    panzoomInstance.reset();
    updateZoomLevel();
  });

  // Keyboard shortcuts
  document.addEventListener('keydown', (e) => {
    if (!isMouseOverModal || !panzoomInstance) return;

    if (e.key === '+' || e.key === '=') {
      panzoomInstance.zoomIn();
      updateZoomLevel();
    } else if (e.key === '-' || e.key === '_') {
      panzoomInstance.zoomOut();
      updateZoomLevel();
    } else if (e.key === '0') {
      panzoomInstance.reset();
      updateZoomLevel();
    }
  });

  // Update zoom level on pan/zoom
  panzoomInstance.on('zoom', updateZoomLevel);
}

function updateZoomLevel() {
  const scale = panzoomInstance.getScale();
  const percentage = Math.round(scale * 100);
  document.getElementById('zoom-level').textContent = `${percentage}%`;
}
```

#### Step 2.5: Mode Indicator

In `viewer-scripts.ts`, add mode indicator:
```javascript
let modeIndicatorTimeout = null;

function updateModeIndicator() {
  let indicator = document.querySelector('.mode-indicator');

  if (!indicator) {
    indicator = document.createElement('div');
    indicator.className = 'mode-indicator';
    document.querySelector('.modal-container').appendChild(indicator);
  }

  if (isPanMode) {
    indicator.textContent = 'Pan Mode';
    indicator.classList.add('active');
  } else {
    const keyName = panModifierKey === 'Space' ? 'Space' : panModifierKey;
    indicator.textContent = `Zoom Mode (hold ${keyName} for Pan)`;
    indicator.classList.remove('active');
  }

  showModeIndicator();
}

function showModeIndicator() {
  const indicator = document.querySelector('.mode-indicator');
  if (!indicator) return;

  indicator.classList.add('visible');

  // Fade after 2 seconds of inactivity
  clearTimeout(modeIndicatorTimeout);
  modeIndicatorTimeout = setTimeout(() => {
    indicator.classList.remove('visible');
  }, 2000);
}
```

#### Step 2.6: Zoom Control Styles

In `viewer-styles.ts`, add zoom control CSS:
```css
/* Zoom controls (bottom-right corner) */
.zoom-controls {
  position: absolute;
  bottom: 1.5rem;
  right: 1.5rem;
  display: flex;
  gap: 0.5rem;
  align-items: center;
  background: var(--code-bg);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 0.5rem;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  z-index: 10;
}

.zoom-btn {
  background: transparent;
  border: 1px solid var(--border-color);
  border-radius: 6px;
  padding: 0.5rem;
  cursor: pointer;
  color: var(--text-color);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
}

.zoom-btn:hover {
  background: var(--hover-bg);
  transform: scale(1.1);
}

.zoom-level {
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-color);
  min-width: 3.5rem;
  text-align: center;
}

/* Mode indicator (bottom-left corner) */
.mode-indicator {
  position: absolute;
  bottom: 1.5rem;
  left: 1.5rem;
  background: var(--code-bg);
  border: 1px solid var(--border-color);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--text-color);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  z-index: 10;
  opacity: 0.5;
  transition: opacity 0.2s ease;
  pointer-events: none;
}

.mode-indicator.visible {
  opacity: 1;
}

.mode-indicator.active {
  opacity: 1;
  background: var(--accent-color);
  color: white;
  border-color: var(--accent-color);
}

/* Accent color for active state */
[data-theme="dark"] {
  --accent-color: #3b82f6;  /* blue-500 */
}

[data-theme="light"] {
  --accent-color: #2563eb;  /* blue-600 */
}
```

---

### Phase 3: Configuration and Persistence

**Goal**: Allow users to configure pan modifier key and persist zoom level across sessions.

**Files to Modify**:
1. `viewer-scripts.ts` - Add settings menu and localStorage persistence

**Implementation Steps**:

#### Step 3.1: Settings Menu

In `viewer-template.ts`, add settings button to modal header:
```html
<button id="modal-settings" class="modal-button" title="Settings">
  <svg><!-- Gear icon --></svg>
</button>
```

In `viewer-scripts.ts`, add settings modal:
```javascript
function openSettingsModal() {
  const modal = document.createElement('div');
  modal.id = 'settings-modal';
  modal.className = 'settings-modal-backdrop';
  modal.innerHTML = `
    <div class="settings-modal">
      <h3>Diagram Viewer Settings</h3>

      <div class="setting-row">
        <label>Pan Modifier Key:</label>
        <select id="pan-modifier-select">
          <option value="Space" ${panModifierKey === 'Space' ? 'selected' : ''}>Space</option>
          <option value="Shift" ${panModifierKey === 'Shift' ? 'selected' : ''}>Shift</option>
          <option value="Alt" ${panModifierKey === 'Alt' ? 'selected' : ''}>Alt</option>
        </select>
      </div>

      <div class="setting-row">
        <label>
          <input type="checkbox" id="persist-zoom-checkbox" ${localStorage.getItem('persistZoom') === 'true' ? 'checked' : ''}>
          Remember zoom level
        </label>
      </div>

      <div class="setting-actions">
        <button id="settings-save" class="primary-btn">Save</button>
        <button id="settings-cancel" class="secondary-btn">Cancel</button>
      </div>
    </div>
  `;

  document.body.appendChild(modal);

  // Event listeners
  document.getElementById('settings-save').addEventListener('click', () => {
    panModifierKey = document.getElementById('pan-modifier-select').value;
    const persistZoom = document.getElementById('persist-zoom-checkbox').checked;

    localStorage.setItem('panModifierKey', panModifierKey);
    localStorage.setItem('persistZoom', persistZoom.toString());

    modal.remove();
    updateModeIndicator();
  });

  document.getElementById('settings-cancel').addEventListener('click', () => {
    modal.remove();
  });
}

// Load settings on page load
function loadSettings() {
  panModifierKey = localStorage.getItem('panModifierKey') || 'Space';
}

loadSettings();
```

#### Step 3.2: Persist Zoom Level

In `viewer-scripts.ts`, modify panzoom initialization:
```javascript
function openMermaidModal(index) {
  // ... existing code ...

  // Restore zoom level if persistence enabled
  if (localStorage.getItem('persistZoom') === 'true') {
    const savedScale = parseFloat(localStorage.getItem('diagramZoomScale') || '1');
    panzoomInstance.zoom(savedScale, { animate: false });
  }

  // Save zoom level on change
  panzoomInstance.on('zoom', () => {
    if (localStorage.getItem('persistZoom') === 'true') {
      const scale = panzoomInstance.getScale();
      localStorage.setItem('diagramZoomScale', scale.toString());
    }
    updateZoomLevel();
  });
}
```

---

## Key Technical Decisions

### 1. Library Choice: Panzoom vs ReactFlow vs Custom

**Options Considered**:
- **ReactFlow**: Used in mindstrike, but requires React (not available in vanilla JS viewer)
- **Panzoom.js**: Lightweight (11KB), no dependencies, vanilla JS compatible
- **Custom implementation**: Would replicate ReactFlow's wheel handler algorithm

**Decision**: **Use Panzoom.js**
- ✅ Proven zoom-to-cursor algorithm
- ✅ Built-in gesture support (mouse wheel, touch, drag)
- ✅ Small bundle size (11KB minified)
- ✅ No framework dependencies
- ✅ MIT license

**Alternative**: If panzoom doesn't provide enough control, implement custom wheel handler using ReactFlow's algorithm (from mindstrike `MindMap.tsx` lines 994-1013).

### 2. Pan Modifier Key: Space vs Shift vs Alt

**Mindstrike uses**: Space (default), configurable to Shift or Alt

**Consideration**: Space bar conflicts with native browser scroll (spacebar = page down)

**Decision**: **Default to Space, but make configurable**
- In fullscreen modal, spacebar conflicts are minimal (no page scrolling)
- Provide dropdown in settings to change to Shift or Alt
- Persist preference in localStorage

### 3. Zoom Range: 0.1-2.0 vs 0.5-5.0

**Mindstrike uses**: 0.1x - 2.0x for mindmap

**Consideration**: Diagrams may benefit from higher zoom for detail viewing

**Decision**: **Use 0.5x - 5.0x**
- Diagrams often have dense text (unlike mindmaps which have sparse nodes)
- Higher zoom (5x) allows reading small text
- Lower minimum (0.5x) still provides sufficient overview
- Can be adjusted in panzoom config if needed

### 4. Mode Indicator: Always Visible vs Fade on Inactivity

**Mindstrike uses**: Semi-transparent (50%) when idle, fully opaque when active/scrolling

**Decision**: **Semi-transparent always, fully opaque when active, fade after 2s inactivity**
- Always visible at 50% opacity for discoverability
- Fully opaque (100%) when pan mode active OR scrolling
- Fade to 50% after 2 seconds of no interaction
- Never fully hidden (unlike mindstrike which can hide completely)

### 5. Diagram Positioning: Center vs Top-Left

**Consideration**: How should diagram be positioned in fullscreen modal?

**Decision**: **Center with flex layout**
- Modal body uses `display: flex; align-items: center; justify-content: center;`
- Diagram starts centered for best first impression
- User can pan to any position after initial render
- Reset button returns to centered view

### 6. Button Visibility: Always Show vs Hover Show

**Mindstrike uses**: Hover to show fullscreen button

**Decision**: **Hover to show** (matching mindstrike pattern)
- Reduces visual clutter
- Intuitive interaction pattern
- Fade in on hover with 200ms transition
- Button positioned top-right corner of each diagram

---

## Implementation Checklist

### Phase 1: Fullscreen Modal (MVP)

- [ ] **Template Changes** (`viewer-template.ts`)
  - [ ] Add modal HTML structure at end of `<body>`
  - [ ] Add modal-related buttons to template
  - [ ] Add SVG icons for buttons (fullscreen, download, close)

- [ ] **Script Changes** (`viewer-scripts.ts`)
  - [ ] Add `addFullscreenButtons()` function to inject buttons on each diagram
  - [ ] Add `openMermaidModal(index)` function
  - [ ] Add `closeMermaidModal()` function
  - [ ] Add ESC key handler for closing modal
  - [ ] Add backdrop click handler to close
  - [ ] Add download SVG functionality
  - [ ] Call `addFullscreenButtons()` after mermaid renders

- [ ] **Style Changes** (`viewer-styles.ts`)
  - [ ] Add `.mermaid-wrapper` and `.mermaid-overlay` styles
  - [ ] Add `.mermaid-fullscreen-btn` styles with hover effects
  - [ ] Add `.modal-backdrop` styles with fade animation
  - [ ] Add `.modal-container` styles with scale animation
  - [ ] Add `.modal-header`, `.modal-body` layout styles
  - [ ] Add dark/light theme color variables

- [ ] **Testing**
  - [ ] Test with markdown files containing multiple mermaid diagrams
  - [ ] Verify button appears on hover
  - [ ] Verify modal opens/closes smoothly
  - [ ] Verify ESC key closes modal
  - [ ] Verify backdrop click closes modal
  - [ ] Verify download SVG works
  - [ ] Test in both dark and light themes

### Phase 2: Zoom and Pan (Core Feature)

- [ ] **Template Changes** (`viewer-template.ts`)
  - [ ] Add panzoom CDN script tag
  - [ ] Add zoom control buttons to modal header

- [ ] **Script Changes** (`viewer-scripts.ts`)
  - [ ] Initialize panzoom on modal open
  - [ ] Add `handleModalWheel()` custom wheel handler
  - [ ] Implement vertical scroll = zoom logic (matching ReactFlow formula)
  - [ ] Implement horizontal scroll = pan logic
  - [ ] Add pan modifier key tracking (keydown/keyup listeners)
  - [ ] Add mouse enter/leave tracking for scope control
  - [ ] Add `addZoomControls()` function
  - [ ] Add zoom in/out/reset button handlers
  - [ ] Add keyboard shortcuts (+, -, 0 keys)
  - [ ] Add `updateZoomLevel()` to show current zoom percentage
  - [ ] Add `updateModeIndicator()` function
  - [ ] Add `showModeIndicator()` with timeout fade
  - [ ] Cleanup panzoom instance on modal close

- [ ] **Style Changes** (`viewer-styles.ts`)
  - [ ] Add `.zoom-controls` styles (bottom-right position)
  - [ ] Add `.zoom-btn` styles with hover effects
  - [ ] Add `.zoom-level` display styles
  - [ ] Add `.mode-indicator` styles (bottom-left position)
  - [ ] Add `.mode-indicator.visible` and `.mode-indicator.active` states
  - [ ] Add accent color variables for active state

- [ ] **Testing**
  - [ ] Test vertical scroll zooms in/out
  - [ ] Verify zoom is centered on cursor position
  - [ ] Test horizontal scroll pans left/right
  - [ ] Test modifier key (Space) toggles pan mode
  - [ ] Verify mode indicator shows correct text
  - [ ] Test mode indicator fades after 2s inactivity
  - [ ] Test zoom in/out/reset buttons work
  - [ ] Test keyboard shortcuts (+, -, 0) work
  - [ ] Verify zoom level percentage updates correctly
  - [ ] Test zoom range limits (0.5x - 5.0x)
  - [ ] Test pan outside diagram bounds (should be prevented by panzoom)

### Phase 3: Configuration (Nice-to-Have)

- [ ] **Template Changes** (`viewer-template.ts`)
  - [ ] Add settings button to modal header

- [ ] **Script Changes** (`viewer-scripts.ts`)
  - [ ] Add `openSettingsModal()` function
  - [ ] Add settings modal HTML with pan modifier dropdown
  - [ ] Add "Remember zoom level" checkbox
  - [ ] Add `loadSettings()` function (from localStorage)
  - [ ] Add `saveSettings()` function (to localStorage)
  - [ ] Persist zoom level if option enabled
  - [ ] Restore zoom level on modal open if enabled

- [ ] **Style Changes** (`viewer-styles.ts`)
  - [ ] Add `.settings-modal-backdrop` and `.settings-modal` styles
  - [ ] Add `.setting-row` and form control styles
  - [ ] Add `.primary-btn` and `.secondary-btn` styles

- [ ] **Testing**
  - [ ] Test settings modal opens/closes
  - [ ] Test pan modifier key dropdown saves selection
  - [ ] Test "Remember zoom level" checkbox works
  - [ ] Verify settings persist across page reloads
  - [ ] Test changing pan modifier key updates indicator text
  - [ ] Test zoom level restoration on modal open

### Phase 4: Documentation and Polish

- [ ] **Documentation**
  - [ ] Add feature file: `fullscreen-mermaid-diagram-viewer.feature`
  - [ ] Add feature file: `mermaid-diagram-zoom-and-pan.feature`
  - [ ] Update README with new viewer capabilities
  - [ ] Add keyboard shortcuts reference to help modal

- [ ] **Polish**
  - [ ] Add loading spinner while diagram renders in modal
  - [ ] Add smooth transition when opening modal
  - [ ] Add aria-labels for accessibility
  - [ ] Test keyboard navigation (tab focus on buttons)
  - [ ] Optimize performance (debounce wheel events if needed)
  - [ ] Test with very large diagrams (performance)
  - [ ] Test with mobile/touchpad gestures

---

## File Path Reference

### fspec Files to Modify

**Server/Viewer**:
- `/home/rquast/projects/fspec/src/server/templates/viewer-template.ts` - HTML structure
- `/home/rquast/projects/fspec/src/server/templates/viewer-scripts.ts` - Client-side JavaScript
- `/home/rquast/projects/fspec/src/server/templates/viewer-styles.ts` - CSS styling

**TUI** (no changes needed, but for reference):
- `/home/rquast/projects/fspec/src/tui/components/BoardView.tsx` - Server lifecycle
- `/home/rquast/projects/fspec/src/tui/components/AttachmentDialog.tsx` - File selection
- `/home/rquast/projects/fspec/src/server/attachment-server.ts` - Express server

### Mindstrike Reference Files

**Mermaid Modal**:
- `/home/rquast/projects/mindstrike/src/components/MermaidModal.tsx`
- `/home/rquast/projects/mindstrike/src/hooks/useDialogAnimation.ts`
- `/home/rquast/projects/mindstrike/src/components/MarkdownViewer.tsx`

**Zoom/Pan System**:
- `/home/rquast/projects/mindstrike/src/mindmaps/components/MindMap.tsx` (lines 880-1041)
- `/home/rquast/projects/mindstrike/src/mindmaps/components/ScrollModeOverlay.tsx`

**Mermaid Rendering**:
- `/home/rquast/projects/mindstrike/src/utils/mermaidRenderer.ts`
- `/home/rquast/projects/mindstrike/src/utils/mermaidConfig.ts`

---

## Conclusion

This implementation brings mindstrike's superior mermaid viewing experience to fspec by:

1. **Fullscreen modal** for distraction-free diagram viewing
2. **Zoom controls** with cursor-centered zooming (matching mindstrike's mindmap behavior)
3. **Pan controls** with configurable modifier key (Space/Shift/Alt)
4. **Visual feedback** with mode indicator showing Zoom/Pan state

The key adaptation is translating React-based patterns (mindstrike) to vanilla JavaScript (fspec viewer), while preserving the core UX patterns that make mindstrike's viewer excellent.

**Estimated Implementation Time**:
- Phase 1 (Fullscreen Modal): 2-3 hours
- Phase 2 (Zoom/Pan): 3-4 hours
- Phase 3 (Configuration): 1-2 hours
- Phase 4 (Documentation/Polish): 1-2 hours
- **Total**: 7-11 hours (or 5-8 story points)

**Next Steps**:
1. Create work unit in fspec
2. Attach this document to work unit
3. Begin Example Mapping session with human
4. Generate scenarios from example map
5. Estimate story points
6. Begin ACDD cycle (Specifying → Testing → Implementing → Validating → Done)
