import { defineConfig } from 'vite';
import { resolve } from 'path';

export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        'service-worker': resolve(__dirname, 'src/background/service-worker.ts'),
        'content-script': resolve(__dirname, 'src/content/content-script.ts'),
        'popup': resolve(__dirname, 'src/popup/popup.ts'),
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
