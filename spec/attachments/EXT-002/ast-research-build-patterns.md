# AST Research: Build System Patterns for Extension Scaffolding

> **Work Unit:** EXT-002 — Extension Scaffolding & Build System
> **Date:** 2026-03-05
> **Purpose:** Analyze existing fspec Vite/TypeScript config patterns to inform extension build setup

---

## Findings

### Existing Vite Config (vite.config.ts)

Found `defineConfig` in:
- `./vite.config.ts` — main fspec build config
- `./vitest.config.ts` — test config

The main fspec build uses:
- **Vite lib mode** with `build.lib.entry` pointing to single entry
- **ES module format** (`formats: ['es']`)
- **rollupOptions.external** for Node.js built-ins and npm deps
- **target: 'node18'**
- **Custom plugin** for copying bundled files (spec/, schemas/, git/)
- **inlineDynamicImports: true** for single output file

### Key Differences for Extension Build

The extension build will differ significantly:
1. **Multiple entry points** (service-worker, content-script, popup) vs single entry
2. **Browser target** vs Node.js target — no Node built-in externals
3. **Chrome extension context** — service workers can't use DOM APIs, content scripts run in isolated world
4. **No lib mode** — use rollupOptions.input directly for multiple entries
5. **Output format: ES modules** — MV3 supports `"type": "module"` for service workers
6. **Static file copying** — popup.html needs to be in dist alongside JS

### TypeScript Config (tsconfig.json)

Existing fspec uses:
- `"target": "ES2022"` — can reuse for extension
- `"module": "ESNext"` — appropriate for Vite bundling
- `"moduleResolution": "bundler"` — Vite-compatible
- `"strict": true` — must maintain

Extension tsconfig.json will need:
- Chrome extension type definitions (though we'll avoid `@types/chrome` if not needed for stubs)
- Browser lib instead of Node types
- Separate rootDir pointing to extension/src

---

## Impact on EXT-002

The extension's Vite config will be structurally different from the main fspec config — it's a browser-targeted multi-entry build rather than a Node.js single-entry lib build. The TypeScript config can follow similar strict settings but target browser APIs instead of Node.
