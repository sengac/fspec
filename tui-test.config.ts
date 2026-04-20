import { defineConfig } from '@microsoft/tui-test';

export default defineConfig({
  retries: 1,
  trace: true,
  testMatch: 'e2e/**/*.test.ts',
  // PROV-095 e2e needs to wait for a real streaming LLM response,
  // which can take up to ~90s on Claude Opus through the Rhai
  // custom-provider dispatch path. Bump the per-test timeout well
  // above that so the worker isn't terminated mid-stream.
  timeout: 180_000,
  expect: {
    timeout: 15_000,
  },
});
