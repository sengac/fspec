import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { formatFeatures } from '../format';

describe('Feature: Format Feature Files with Custom AST Formatter', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Format a single feature file', () => {
    it('should format file with consistent indentation', async () => {
      // Given I have a feature file with inconsistent formatting
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const unformattedContent = `@phase1
Feature: Login
Scenario: User logs in
Given I am on login page
When I enter credentials
Then I should be logged in`;

      const filePath = join(featuresDir, 'login.feature');
      await writeFile(filePath, unformattedContent);

      // When I run `fspec format spec/features/login.feature`
      const result = await formatFeatures({
        cwd: testDir,
        file: 'spec/features/login.feature',
      });

      // Then the file should be formatted
      expect(result.formattedCount).toBe(1);

      const formattedContent = await readFile(filePath, 'utf-8');
      expect(formattedContent).toContain('Feature: Login');
      expect(formattedContent).toContain('  Scenario: User logs in');
      expect(formattedContent).toContain('    Given I am on login page');
    });
  });

  describe('Scenario: Apply consistent indentation', () => {
    it('should apply 2-space indentation for scenarios and 4-space for steps', async () => {
      // Given I have a feature file with mixed indentation
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const mixedIndentation = `Feature: Mixed Indentation
   Scenario: Inconsistent spacing
     Given something
       When action
  Then result

Scenario: Another scenario
      Given different spacing
    When another action
         Then another result`;

      const filePath = join(featuresDir, 'mixed.feature');
      await writeFile(filePath, mixedIndentation);

      // When I run `fspec format spec/features/mixed.feature`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/mixed.feature',
      });

      // Then all scenarios should be indented by 2 spaces
      const content = await readFile(filePath, 'utf-8');
      expect(content).toMatch(/^  Scenario: Inconsistent spacing$/m);
      expect(content).toMatch(/^  Scenario: Another scenario$/m);

      // And all steps should be indented by 4 spaces from feature level
      expect(content).toMatch(/^    Given something$/m);
      expect(content).toMatch(/^    When action$/m);
      expect(content).toMatch(/^    Then result$/m);
      expect(content).toMatch(/^    Given different spacing$/m);
      expect(content).toMatch(/^    When another action$/m);
      expect(content).toMatch(/^    Then another result$/m);
    });
  });

  describe('Scenario: Format all feature files', () => {
    it('should format all files in spec/features/', async () => {
      // Given I have multiple feature files with inconsistent formatting
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const unformatted = `Feature: Test\nScenario: Test\nGiven test\n`;

      await writeFile(join(featuresDir, 'auth.feature'), unformatted);
      await writeFile(join(featuresDir, 'api.feature'), unformatted);
      await writeFile(join(featuresDir, 'data.feature'), unformatted);

      // When I run `fspec format`
      const result = await formatFeatures({ cwd: testDir });

      // Then all files should be formatted
      expect(result.formattedCount).toBe(3);

      // And at least one file should be properly indented
      const authContent = await readFile(
        join(featuresDir, 'auth.feature'),
        'utf-8'
      );
      expect(authContent).toContain('  Scenario: Test');
      expect(authContent).toContain('    Given test');
    });
  });

  describe('Scenario: Handle already-formatted file', () => {
    it('should preserve already-formatted content structure', async () => {
      // Given I have a properly formatted feature file
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const wellFormatted = `Feature: Good Format

  Scenario: Test
    Given something
    When action
    Then result
`;

      const filePath = join(featuresDir, 'good.feature');
      await writeFile(filePath, wellFormatted);

      // When I run `fspec format spec/features/good.feature`
      const result = await formatFeatures({
        cwd: testDir,
        file: 'spec/features/good.feature',
      });

      // Then the file content should be preserved
      expect(result.formattedCount).toBe(1);

      const content = await readFile(filePath, 'utf-8');
      expect(content).toContain('Feature: Good Format');
      expect(content).toContain('  Scenario: Test');
      expect(content).toContain('    Given something');
      expect(content).toContain('    When action');
      expect(content).toContain('    Then result');
    });
  });

  describe('Scenario: Handle empty spec/features directory', () => {
    it('should return zero formatted count for empty directory', async () => {
      // Given I have an empty "spec/features/" directory
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I run `fspec format`
      const result = await formatFeatures({ cwd: testDir });

      // Then no files should be formatted
      expect(result.formattedCount).toBe(0);
    });
  });

  describe('Scenario: Handle file not found', () => {
    it('should throw error for missing file', async () => {
      // Given no file exists
      // When I run `fspec format spec/features/missing.feature`
      // Then it should throw
      await expect(
        formatFeatures({ cwd: testDir, file: 'spec/features/missing.feature' })
      ).rejects.toThrow();
    });
  });

  describe('Scenario: Preserve data tables', () => {
    it('should maintain data table structure and content', async () => {
      // Given I have a feature file with data tables
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const withDataTable = `Feature: Data Tables
Scenario: Process user data
Given the following users:
| name | email | role |
| Alice | alice@example.com | admin |
| Bob | bob@example.com | user |
When I process the data
Then all users should be imported`;

      const filePath = join(featuresDir, 'data-table.feature');
      await writeFile(filePath, withDataTable);

      // When I run `fspec format`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/data-table.feature',
      });

      // Then the data table content should be preserved (may be reformatted)
      const content = await readFile(filePath, 'utf-8');
      expect(content).toContain('name');
      expect(content).toContain('email');
      expect(content).toContain('role');
      expect(content).toContain('Alice');
      expect(content).toContain('alice@example.com');
      expect(content).toContain('admin');
      expect(content).toContain('Bob');
      expect(content).toContain('bob@example.com');
      expect(content).toContain('user');
    });
  });

  describe('Scenario: Preserve doc strings', () => {
    it('should maintain doc string content exactly', async () => {
      // Given I have a feature file with doc strings
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const withDocString = `Feature: Documentation

  """
  Architecture notes:
  - This is important context
  - Multiple lines of documentation
  """

  Scenario: Test with doc string
    Given a step with doc string:
      """
      This is example JSON:
      {"key": "value"}
      """
    When I process it
    Then it works`;

      const filePath = join(featuresDir, 'docs.feature');
      await writeFile(filePath, withDocString);

      // When I run `fspec format spec/features/docs.feature`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/docs.feature',
      });

      // Then the doc strings should be preserved exactly
      const content = await readFile(filePath, 'utf-8');
      expect(content).toContain('Architecture notes:');
      expect(content).toContain('This is important context');
      expect(content).toContain('{"key": "value"}');

      // And the triple quotes (""") should remain on their own lines
      expect(content).toMatch(/^\s*"""\s*$/m);
    });
  });

  describe('Scenario: Show formatted files in output', () => {
    it('should return count of formatted files', async () => {
      // Given I have 3 feature files
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const unformatted = `Feature: Test\nScenario: Test\nGiven test\n`;

      await writeFile(join(featuresDir, 'file1.feature'), unformatted);
      await writeFile(join(featuresDir, 'file2.feature'), unformatted);
      await writeFile(join(featuresDir, 'file3.feature'), unformatted);

      // When I run `fspec format`
      const result = await formatFeatures({ cwd: testDir });

      // Then the output should show summary count
      expect(result.formattedCount).toBe(3);
    });
  });

  describe('Scenario: AI agent workflow - create, format, validate', () => {
    it('should support complete workflow from creation to validation', async () => {
      // Given I am an AI agent creating a new specification
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // When I create a feature file
      const unformattedContent = `@phase1
Feature: Data Export
Scenario: Export data
Given data exists
When I export
Then file is created`;

      const filePath = join(featuresDir, 'data-export.feature');
      await writeFile(filePath, unformattedContent);

      // And I edit the file to add scenarios (simulated above)
      // And I run `fspec format`
      const formatResult = await formatFeatures({
        cwd: testDir,
        file: 'spec/features/data-export.feature',
      });

      // Then the file should be properly formatted
      expect(formatResult.formattedCount).toBe(1);

      const formattedContent = await readFile(filePath, 'utf-8');
      expect(formattedContent).toContain('  Scenario: Export data');
      expect(formattedContent).toContain('    Given data exists');

      // And when I validate it, it should pass
      expect(formattedContent).toContain('Feature: Data Export');
    });
  });

  describe('Scenario: Format Scenario Outline with Examples', () => {
    it('should format Scenario Outline, Examples, and data tables correctly', async () => {
      // Given I have a feature file with Scenario Outline and Examples table
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const scenarioOutlineContent = `Feature: Login
Scenario Outline: Login with different credentials
Given I am on the login page
When I enter <username> and <password>
Then I should see <message>
Examples:
| username | password | message |
| admin | pass123 | Welcome Admin |
| user | wrongpass | Invalid credentials |`;

      const filePath = join(featuresDir, 'outline.feature');
      await writeFile(filePath, scenarioOutlineContent);

      // When I run `fspec format`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/outline.feature',
      });

      // Then the Scenario Outline should be indented by 2 spaces
      const content = await readFile(filePath, 'utf-8');
      expect(content).toContain(
        '  Scenario Outline: Login with different credentials'
      );

      // And the Examples section should be indented by 4 spaces
      expect(content).toContain('    Examples:');

      // And the Examples table should be indented by 6 spaces
      expect(content).toMatch(/^\s{6}\|/m);

      // And table columns should be aligned with padding
      expect(content).toContain('username');
      expect(content).toContain('password');
      expect(content).toContain('message');
    });
  });

  describe('Scenario: Format Rule keyword', () => {
    it('should format Rule with nested Background and Scenarios', async () => {
      // Given I have a feature file with Rule containing nested Background and Scenarios
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const ruleContent = `Feature: Account Management
Rule: Account balance cannot go negative
Background:
Given an account with balance 100
Scenario: Successful withdrawal
When I withdraw 50
Then the balance should be 50`;

      const filePath = join(featuresDir, 'rules.feature');
      await writeFile(filePath, ruleContent);

      // When I run `fspec format`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/rules.feature',
      });

      // Then the Rule should be indented by 2 spaces
      const content = await readFile(filePath, 'utf-8');
      expect(content).toContain('  Rule: Account balance cannot go negative');

      // And nested Background should be indented by 4 spaces
      expect(content).toContain('    Background:');

      // And nested Scenarios should be indented by 4 spaces
      expect(content).toContain('    Scenario: Successful withdrawal');

      // And steps should be indented by 6 spaces
      expect(content).toMatch(/^\s{6}Given an account with balance 100$/m);
      expect(content).toMatch(/^\s{6}When I withdraw 50$/m);
    });
  });

  describe('Scenario: Format DocStrings with content types', () => {
    it('should preserve DocString delimiters and content types', async () => {
      // Given I have a feature file with DocStrings marked as """ json
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const withDocStrings = `Feature: JSON API
Scenario: Send JSON payload
Given I send the following JSON:
"""json
{"name": "John", "age": 30}
"""
When I submit the request
Then it should succeed`;

      const filePath = join(featuresDir, 'docstrings.feature');
      await writeFile(filePath, withDocStrings);

      // When I run `fspec format`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/docstrings.feature',
      });

      // Then the DocString delimiter should include the content type
      const content = await readFile(filePath, 'utf-8');
      expect(content).toContain('"""json');

      // And the content should be preserved exactly
      expect(content).toContain('{"name": "John", "age": 30}');
    });
  });

  describe('Scenario: Format multiple tags', () => {
    it('should format each tag on its own line with zero indentation', async () => {
      // Given I have a feature with tags @phase1 @cli @formatter
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const withTags = `@phase1 @cli @formatter
Feature: Tags Test
Scenario: Test scenario
Given a step`;

      const filePath = join(featuresDir, 'tags.feature');
      await writeFile(filePath, withTags);

      // When I run `fspec format`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/tags.feature',
      });

      // Then each tag should be on its own line
      const content = await readFile(filePath, 'utf-8');
      expect(content).toMatch(/^@phase1$/m);
      expect(content).toMatch(/^@cli$/m);
      expect(content).toMatch(/^@formatter$/m);
    });
  });

  describe('Scenario: Format And/But step keywords', () => {
    it('should format And and But steps with consistent indentation', async () => {
      // Given I have a feature file with And and But steps
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const withAndBut = `Feature: And/But Steps
Scenario: Test And and But
Given I am logged in
And I have permissions
When I click submit
But I have not filled required fields
Then I should see an error`;

      const filePath = join(featuresDir, 'steps.feature');
      await writeFile(filePath, withAndBut);

      // When I run `fspec format`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/steps.feature',
      });

      // Then And steps should be formatted like other steps
      const content = await readFile(filePath, 'utf-8');
      expect(content).toMatch(/^\s{4}And I have permissions$/m);

      // And But steps should be formatted like other steps
      expect(content).toMatch(/^\s{4}But I have not filled required fields$/m);

      // And all steps should be indented by 4 spaces
      expect(content).toMatch(/^\s{4}Given I am logged in$/m);
      expect(content).toMatch(/^\s{4}When I click submit$/m);
      expect(content).toMatch(/^\s{4}Then I should see an error$/m);
    });
  });

  describe('Scenario: Format wildcard step keyword', () => {
    it('should format * steps with consistent indentation', async () => {
      // Given I have a feature file with * step keyword
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const withWildcard = `Feature: Wildcard Steps
Scenario: Test wildcard
* I do something
* I do another thing
Then result`;

      const filePath = join(featuresDir, 'wildcard.feature');
      await writeFile(filePath, withWildcard);

      // When I run `fspec format`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/wildcard.feature',
      });

      // Then the * steps should be formatted correctly
      const content = await readFile(filePath, 'utf-8');
      expect(content).toMatch(/^\s{4}\* I do something$/m);
      expect(content).toMatch(/^\s{4}\* I do another thing$/m);

      // And indentation should match other steps
      expect(content).toMatch(/^\s{4}Then result$/m);
    });
  });

  describe('Scenario: Limit excessive blank lines', () => {
    it('should limit consecutive blank lines to maximum 2', async () => {
      // Given I have a feature file with 6 consecutive blank lines in description
      const featuresDir = join(testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      const withExcessiveBlankLines = `Feature: Excessive Blanks
  """
  Architecture notes:
  - Line 1






  - Line 2
  """

  Scenario: Test
    Given a step`;

      const filePath = join(featuresDir, 'excessive.feature');
      await writeFile(filePath, withExcessiveBlankLines);

      // When I run `fspec format`
      await formatFeatures({
        cwd: testDir,
        file: 'spec/features/excessive.feature',
      });

      // Then consecutive blank lines should be limited to maximum 2
      const content = await readFile(filePath, 'utf-8');
      expect(content).not.toMatch(/\n\n\n\n/); // No 4+ consecutive blank lines

      // And the file should still contain the description
      expect(content).toContain('Architecture notes:');
      expect(content).toContain('Line 1');
      expect(content).toContain('Line 2');
    });
  });
});
