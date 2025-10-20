/**
 * Feature: spec/features/hook-system-documentation-and-examples.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { readFile } from 'fs/promises';
import { join } from 'path';

const DOCS_DIR = join(process.cwd(), 'docs', 'hooks');
const EXAMPLES_DIR = join(process.cwd(), 'examples', 'hooks');

describe('Feature: Hook system documentation and examples', () => {
  describe('Scenario: Configuration documentation shows complete JSON schema', () => {
    it('should have complete configuration documentation', async () => {
      // Given I am reading the hook system documentation
      // When I look at the configuration section
      const configDoc = await readFile(
        join(DOCS_DIR, 'configuration.md'),
        'utf-8'
      );

      // Then I should see a complete fspec-hooks.json example
      expect(configDoc).toContain('fspec-hooks.json');

      // And the example should include global defaults section
      expect(configDoc).toContain('global');

      // And the example should include multiple hook definitions
      expect(configDoc).toContain('hooks');

      // And the example should document all configuration fields
      expect(configDoc).toContain('name');
      expect(configDoc).toContain('command');
      expect(configDoc).toContain('blocking');
      expect(configDoc).toContain('timeout');
      expect(configDoc).toContain('condition');

      // And the example should show hooks object with event names as keys
      expect(configDoc).toMatch(/pre-\w+/);
      expect(configDoc).toMatch(/post-\w+/);

      // And the example should show hook properties
      expect(configDoc).toContain('name');
      expect(configDoc).toContain('command');
      expect(configDoc).toContain('blocking');
      expect(configDoc).toContain('timeout');
      expect(configDoc).toContain('condition');
    });
  });

  describe('Scenario: Bash hook example reads context and validates feature file', () => {
    it('should have bash validation hook example', async () => {
      // Given I am looking at bash hook examples
      // When I read the validation hook example
      const bashExample = await readFile(
        join(EXAMPLES_DIR, 'validate-feature.sh'),
        'utf-8'
      );

      // Then I should see how to read JSON from stdin
      expect(bashExample).toContain('read');

      // And I should see how to parse workUnitId from context
      expect(bashExample).toContain('workUnitId');

      // And I should see how to check if feature file exists
      expect(bashExample).toMatch(/\[ -f .* \]/);

      // And I should see proper exit code 0 for success
      expect(bashExample).toContain('exit 0');

      // And I should see proper exit code 1 for validation failure
      expect(bashExample).toContain('exit 1');

      // And I should see stderr output for error messages
      expect(bashExample).toContain('>&2');
    });
  });

  describe('Scenario: Python hook example parses stdin and runs tests', () => {
    it('should have python test runner hook example', async () => {
      // Given I am looking at python hook examples
      // When I read the test runner hook example
      const pythonExample = await readFile(
        join(EXAMPLES_DIR, 'run-tests.py'),
        'utf-8'
      );

      // Then I should see how to import json and sys modules
      expect(pythonExample).toContain('import json');
      expect(pythonExample).toContain('import sys');

      // And I should see how to read stdin with sys.stdin.read()
      expect(pythonExample).toContain('sys.stdin.read()');

      // And I should see how to parse JSON context
      expect(pythonExample).toContain('json.loads');

      // And I should see how to run pytest subprocess
      expect(pythonExample).toContain('subprocess');
      expect(pythonExample).toContain('pytest');

      // And I should see how to capture test output
      expect(pythonExample).toContain('stdout');
      expect(pythonExample).toContain('stderr');

      // And I should see proper exit code handling from pytest
      expect(pythonExample).toContain('sys.exit');
    });
  });

  describe('Scenario: Node.js hook example reads stdin and sends notifications', () => {
    it('should have node.js notification hook example', async () => {
      // Given I am looking at node.js hook examples
      // When I read the notification hook example
      const nodeExample = await readFile(
        join(EXAMPLES_DIR, 'notify-slack.js'),
        'utf-8'
      );

      // Then I should see how to read stdin asynchronously
      expect(nodeExample).toMatch(/stdin|readline/);

      // And I should see how to parse JSON with JSON.parse()
      expect(nodeExample).toContain('JSON.parse');

      // And I should see how to extract workUnitId and event from context
      expect(nodeExample).toContain('workUnitId');
      expect(nodeExample).toContain('event');

      // And I should see how to send HTTP request to Slack webhook
      expect(nodeExample).toMatch(/fetch|axios|https/);

      // And I should see proper error handling for network failures
      expect(nodeExample).toMatch(/catch|try/);

      // And I should see process.exit() with appropriate codes
      expect(nodeExample).toContain('process.exit');
    });
  });

  describe('Scenario: Lint hook example demonstrates proper exit codes', () => {
    it('should have lint hook example with exit codes', async () => {
      // Given I am looking at common use case examples
      // When I read the linting hook example
      const lintExample = await readFile(
        join(EXAMPLES_DIR, 'lint.sh'),
        'utf-8'
      );

      // Then I should see exit code 0 for clean lint results
      expect(lintExample).toContain('exit 0');

      // And I should see exit code 1 for lint errors found
      expect(lintExample).toContain('exit 1');

      // And I should see lint errors written to stderr
      expect(lintExample).toContain('>&2');

      // And I should see summary written to stdout
      expect(lintExample).toContain('echo');

      // And the example should show blocking: true for lint enforcement
      // (This would be in the accompanying documentation or JSON config example)
      expect(lintExample).toMatch(/blocking|exit 1/);
    });
  });

  describe('Scenario: Troubleshooting section explains common errors', () => {
    it('should have troubleshooting documentation', async () => {
      // Given I am reading the troubleshooting documentation
      // When I look for "Hook command not found" error
      const troubleshootingDoc = await readFile(
        join(DOCS_DIR, 'troubleshooting.md'),
        'utf-8'
      );

      // Then I should see explanation of the error cause
      expect(troubleshootingDoc).toContain('Hook command not found');

      // And I should see solution: check file path is relative to project root
      expect(troubleshootingDoc).toContain('relative to project root');

      // And I should see solution: verify file has execute permissions
      expect(troubleshootingDoc).toMatch(/execute permission|chmod \+x/);

      // And I should see example of correct vs incorrect file paths
      expect(troubleshootingDoc).toMatch(/spec\/hooks\/.*\.sh/);

      // And I should see how to test hook script manually
      expect(troubleshootingDoc).toMatch(/Testing hook script manually/);
      expect(troubleshootingDoc).toContain('echo');
    });
  });
});
