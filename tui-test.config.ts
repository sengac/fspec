import { defineConfig } from '@microsoft/tui-test';

export default defineConfig({
  retries: 1,
  trace: true,
  testMatch: 'e2e/**/*.test.ts',
  timeout: 60_000,
  expect: {
    timeout: 15_000,
  },
});
