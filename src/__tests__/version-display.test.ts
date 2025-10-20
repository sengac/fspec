/**
 * Feature: spec/features/fspec-help-displays-hardcoded-version-0-0-1-instead-of-package-json-version.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { execSync } from 'child_process';
import { readFileSync, writeFileSync } from 'fs';
import { join } from 'path';

describe('Feature: fspec --help displays hardcoded version 0.0.1 instead of package.json version', () => {
  const projectRoot = join(process.cwd());
  const packageJsonPath = join(projectRoot, 'package.json');
  let originalPackageJson: string;

  beforeEach(() => {
    // Save original package.json
    originalPackageJson = readFileSync(packageJsonPath, 'utf-8');
  });

  afterEach(() => {
    // Restore original package.json
    writeFileSync(packageJsonPath, originalPackageJson, 'utf-8');
  });

  describe('Scenario: Display current version from package.json', () => {
    it('should display version 0.2.1 when package.json has version 0.2.1', () => {
      // Given package.json has version "0.2.1"
      const packageJson = JSON.parse(originalPackageJson);
      packageJson.version = '0.2.1';
      writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2), 'utf-8');

      // And the project has been built with npm run build
      execSync('npm run build', { stdio: 'ignore' });

      // When I run "fspec --help"
      const output = execSync('./dist/index.js --help', { encoding: 'utf-8' });

      // Then the output should contain "Version 0.2.1"
      expect(output).toContain('Version 0.2.1');
    });
  });

  describe('Scenario: Display updated version after package.json change', () => {
    it('should display version 0.3.0 after updating package.json and rebuilding', () => {
      // Given package.json has version "0.3.0"
      const packageJson = JSON.parse(originalPackageJson);
      packageJson.version = '0.3.0';
      writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2), 'utf-8');

      // And the project has been rebuilt with npm run build
      execSync('npm run build', { stdio: 'ignore' });

      // When I run "fspec --help"
      const output = execSync('./dist/index.js --help', { encoding: 'utf-8' });

      // Then the output should contain "Version 0.3.0"
      expect(output).toContain('Version 0.3.0');
    });
  });

  describe('Scenario: Omit version line when version cannot be read', () => {
    it('should not display version line when version is unavailable', () => {
      // Given the version cannot be read from package.json
      // (We'll simulate this by removing the version field)
      const packageJson = JSON.parse(originalPackageJson);
      delete packageJson.version;
      writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2), 'utf-8');

      // And rebuild
      execSync('npm run build', { stdio: 'ignore' });

      // When I run "fspec --help"
      const output = execSync('./dist/index.js --help', { encoding: 'utf-8' });

      // Then the output should not contain a "Version" line
      expect(output).not.toContain('Version');
    });
  });
});
