import { defineConfig } from 'vite';
import { resolve } from 'path';
import { copyFileSync, mkdirSync } from 'fs';

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
      ],
      output: {
        preserveModules: false,
      },
    },
    target: 'node18',
    outDir: 'dist',
    emptyOutDir: true,
  },
  plugins: [
    {
      name: 'copy-templates',
      closeBundle() {
        const templatesDir = resolve(__dirname, 'dist', 'templates');
        mkdirSync(templatesDir, { recursive: true });
        copyFileSync(
          resolve(__dirname, 'templates', 'CLAUDE.md'),
          resolve(templatesDir, 'CLAUDE.md')
        );
      },
    },
  ],
});
