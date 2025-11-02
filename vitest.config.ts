import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    env: {
      FORCE_COLOR: '0', // Disable colors to ensure consistent test behavior regardless of TTY
    },

    // 🔥 Safest settings to prevent system crashes and memory leaks
    pool: 'forks', // Use fork pool instead of threads for better stability
    poolOptions: {
      forks: {
        singleFork: true, // ⭐ Single process prevents file conflicts and memory leaks
      },
    },
    fileParallelism: false, // ⭐ Sequential file execution prevents resource conflicts
    maxConcurrency: 1, // ⭐ 1 test at a time per file

    testTimeout: 30000, // 30 seconds timeout for slow tests
    hookTimeout: 30000, // 30 seconds for setup/teardown hooks

    include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      exclude: ['node_modules/', 'dist/', '**/*.test.ts', '**/*.test.tsx'],
    },
  },
});
