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
        'codelet-napi',
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
        'net',
        'http',
        'https',
        'readline',
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
        'chokidar',
        'worker_threads',
        'diff',
        'open',
        'express',
        'marked',
        'dompurify',
        'execa',
        '@sengac/tree-sitter',
        '@sengac/tree-sitter-dart',
        '@sengac/tree-sitter-javascript',
        '@sengac/tree-sitter-typescript',
        '@sengac/tree-sitter-python',
        '@sengac/tree-sitter-go',
        '@sengac/tree-sitter-rust',
        '@sengac/tree-sitter-java',
        '@sengac/tree-sitter-ruby',
        '@sengac/tree-sitter-c-sharp',
        '@sengac/tree-sitter-php',
        '@sengac/tree-sitter-cpp',
        '@sengac/tree-sitter-bash',
        '@sengac/tree-sitter-json',
        '@sengac/tree-sitter-kotlin',
        '@sengac/tree-sitter-swift',
        '@sengac/tree-sitter-c',
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

        // Bundle git directory (includes diff-worker.js)
        cpSync(
          resolve(__dirname, 'src', 'git'),
          resolve(__dirname, 'dist', 'git'),
          { recursive: true }
        );

        // Bundle ast-queries directory (.scm query files)
        cpSync(
          resolve(__dirname, 'src', 'utils', 'ast-queries'),
          resolve(__dirname, 'dist', 'ast-queries'),
          { recursive: true }
        );
      },
    },
  ],
});
