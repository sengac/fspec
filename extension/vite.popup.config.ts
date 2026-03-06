import { defineConfig } from 'vite';
import { resolve } from 'path';

/**
 * Separate Vite build for the popup script.
 *
 * Built as IIFE to produce a single self-contained file with no
 * import statements. Run after the service-worker build with
 * emptyOutDir: false so it doesn't wipe previous outputs.
 */
export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        popup: resolve(__dirname, 'src/popup/popup.ts'),
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
