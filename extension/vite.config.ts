import { defineConfig } from 'vite';
import { resolve } from 'path';

/**
 * Chrome extension build configuration.
 *
 * Builds service-worker and popup as ES modules (service worker declares
 * "type": "module" in manifest.json; popup is loaded via <script type="module">).
 *
 * Content script is built separately (see vite.content.config.ts) as IIFE
 * because Chrome loads content scripts as classic scripts — ES module
 * imports are not supported.
 */
export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        'service-worker': resolve(
          __dirname,
          'src/background/service-worker.ts'
        ),
        popup: resolve(__dirname, 'src/popup/popup.ts'),
      },
      output: {
        dir: resolve(__dirname, 'dist'),
        entryFileNames: '[name].js',
        format: 'es',
      },
    },
    outDir: 'dist',
    emptyOutDir: true,
    target: 'es2022',
    minify: false,
    sourcemap: true,
  },
});
