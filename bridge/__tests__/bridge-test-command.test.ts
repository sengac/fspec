/**
 * Feature: spec/features/bridge-test-suite-runs-reproducibly.feature
 *
 * BUG-503: the bridge's vitest suite runs with one documented command
 * (cd bridge && npm ci && npm test). These tests read the package wiring so
 * removing the dev dependency, the test script, the lock entry or the config
 * fails the suite.
 */
import { describe, it, expect } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const bridgeDir = join(dirname(fileURLToPath(import.meta.url)), '..');

function readJson(name: string): Record<string, unknown> {
  return JSON.parse(readFileSync(join(bridgeDir, name), 'utf8')) as Record<
    string,
    unknown
  >;
}

describe('Feature: Bridge test suite runs reproducibly', () => {
  describe('Scenario: The bridge package declares its test runner and test command', () => {
    it('declares vitest, a vitest run test script and a lock entry for vitest', () => {
      // @step Given the bridge package manifest
      const manifest = readJson('package.json');

      // @step When its dev dependencies and scripts are read
      const devDependencies = (manifest.devDependencies ?? {}) as Record<
        string,
        string
      >;
      const scripts = (manifest.scripts ?? {}) as Record<string, string>;

      // @step Then vitest is a dev dependency
      expect(devDependencies.vitest).toMatch(/^\^?4\./);

      // @step And the test script runs 'vitest run'
      expect(scripts.test).toBe('vitest run');

      // @step And the lock file pins the vitest version that is installed
      const lock = readJson('package-lock.json');
      const packages = (lock.packages ?? {}) as Record<
        string,
        { version?: string }
      >;
      const locked = packages['node_modules/vitest']?.version;
      expect(locked).toMatch(/^4\./);
      const installed = readJson('node_modules/vitest/package.json');
      expect(installed.version).toBe(locked);
    });
  });

  describe('Scenario: The vitest config runs exactly the bridge tests in node', () => {
    it('includes __tests__/**/*.test.ts in the node environment', () => {
      // @step Given the bridge vitest config
      const configPath = join(bridgeDir, 'vitest.config.ts');
      expect(existsSync(configPath)).toBe(true);

      // @step When its test settings are read
      const config = readFileSync(configPath, 'utf8');

      // @step Then it includes the test files under __tests__ ending in .test.ts
      expect(config).toContain("include: ['__tests__/**/*.test.ts']");

      // @step And it runs them in the node environment
      expect(config).toContain("environment: 'node'");
    });
  });
});
