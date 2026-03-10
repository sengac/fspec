import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',
    env: {
      FORCE_COLOR: '0',
    },

    pool: 'forks',
    poolOptions: {
      forks: {
        singleFork: true,
      },
    },
    fileParallelism: false,
    maxConcurrency: 1,

    testTimeout: 240_000,
    hookTimeout: 60_000,

    include: ['src/**/*e2e*.test.ts', 'src/**/*E2E*.test.ts'],
    exclude: ['node_modules/**', 'dist/**'],
  },
});
