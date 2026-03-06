import { defineConfig } from 'vite';
import { resolve } from 'path';

/**
 * Separate Vite build for the content script.
 *
 * Chrome loads content scripts as classic scripts (not ES modules),
 * so we must build as IIFE. This produces a single self-contained
 * dist/content-script.js with no import statements.
 *
 * Run after the main build (vite.config.ts) with emptyOutDir: false
 * so it doesn't wipe the service-worker and popup outputs.
 */
export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        'content-script': resolve(__dirname, 'src/content/content-script.ts'),
      },
      output: {
        dir: resolve(__dirname, 'dist'),
        entryFileNames: '[name].js',
        format: 'iife',
      },
    },
    outDir: 'dist',
    emptyOutDir: false,
    target: 'es2022',
    minify: false,
    sourcemap: true,
  },
});
