import { defineConfig } from 'vite';
import { resolve } from 'path';

/**
 * Chrome extension build configuration — Service Worker.
 *
 * Each extension entry point is built as a separate IIFE to produce
 * a single self-contained file with zero import statements. This is
 * critical for Chrome extensions: code splitting creates shared chunks
 * with relative ES imports that Chrome's extension runtime cannot resolve.
 *
 * Build order (see package.json "build" script):
 *   1. vite.config.ts          — service-worker (emptyOutDir: true)
 *   2. vite.popup.config.ts    — popup          (emptyOutDir: false)
 *   3. vite.content.config.ts  — content-script (emptyOutDir: false)
 */
export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        'service-worker': resolve(
          __dirname,
          'src/background/service-worker.ts'
        ),
      },
      output: {
        dir: resolve(__dirname, 'dist'),
        entryFileNames: '[name].js',
        format: 'iife',
      },
    },
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
    minify: false,
    sourcemap: true,
  },
});
