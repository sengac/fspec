import { defineConfig } from 'vite';
import { resolve } from 'path';
import { cpSync } from 'fs';

export default defineConfig({
  build: {
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      formats: ['es'],
      fileName: () => 'index.js',
    },
    rollupOptions: {
      external: [
        'chalk',
        'commander',
        'child_process',
        'crypto',
        'util',
        'path',
        'os',
        'fs',
        'fs/promises',
        'url',
        'module',
        '@cucumber/gherkin',
        '@cucumber/messages',
        'tinyglobby',
        'jsdom',
        'mermaid',
        'ink',
        'react',
        'proper-lockfile',
        'isomorphic-git',
        'winston',
      ],
      output: {
        preserveModules: false,
        manualChunks: undefined,
        inlineDynamicImports: true,
      },
    },
    target: 'node18',
    outDir: 'dist',
    emptyOutDir: true,
  },
  plugins: [
    {
      name: 'copy-bundled-files',
      closeBundle() {
        // Bundle entire spec/ directory (all .md, .json, .feature files)
        cpSync(resolve(__dirname, 'spec'), resolve(__dirname, 'dist', 'spec'), {
          recursive: true,
        });

        // Bundle schemas directory
        cpSync(
          resolve(__dirname, 'src', 'schemas'),
          resolve(__dirname, 'dist', 'schemas'),
          { recursive: true }
        );
      },
    },
  ],
});
