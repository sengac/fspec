// BUG-503: the bridge's vitest suite (cd bridge && npm ci && npm test).
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['__tests__/**/*.test.ts'],
    environment: 'node',
  },
});
