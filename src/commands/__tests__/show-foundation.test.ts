import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile, access } from 'fs/promises';
import { join } from 'path';
import { showFoundation } from '../show-foundation';

describe('Feature: Display Foundation Documentation', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-show-foundation');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Display entire FOUNDATION.md', () => {
    it('should display all sections', async () => {
      // Given I have a FOUNDATION.md with multiple sections
      const content = `# Project Foundation

## What We Are Building

A CLI tool.

## Why

Because we need it.

## Architecture

System design.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation`
      const result = await showFoundation({
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should display all sections
      expect(result.output).toContain('What We Are Building');
      expect(result.output).toContain('Why');
      expect(result.output).toContain('Architecture');

      // And the output should be in text format
      expect(result.format).toBe('text');
    });
  });

  describe('Scenario: Display specific section', () => {
    it('should display only specified section', async () => {
      // Given I have a FOUNDATION.md with a "What We Are Building" section
      const content = `# Project Foundation

## What We Are Building

A CLI tool for specifications.

## Why

Other content.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --section "What We Are Building"`
      const result = await showFoundation({
        section: 'What We Are Building',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should display only that section
      expect(result.output).toContain('A CLI tool for specifications');

      // And other sections should not be displayed
      expect(result.output).not.toContain('Other content');
    });
  });

  describe('Scenario: Display in markdown format', () => {
    it('should preserve markdown formatting', async () => {
      // Given I have a FOUNDATION.md
      const content = `# Project Foundation

## Why

Because we need it.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --format markdown`
      const result = await showFoundation({
        format: 'markdown',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should preserve markdown formatting
      expect(result.output).toContain('# Project Foundation');

      // And section headers should use ## syntax
      expect(result.output).toContain('## Why');
    });
  });

  describe('Scenario: Display in JSON format', () => {
    it('should output valid JSON with sections', async () => {
      // Given I have a FOUNDATION.md with "Why" and "Architecture" sections
      const content = `# Project Foundation

## Why

Because we need it.

## Architecture

System design here.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --format json`
      const result = await showFoundation({
        format: 'json',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should be valid JSON
      const parsed = JSON.parse(result.output!);

      // And the JSON should contain section names as keys
      expect(parsed).toHaveProperty('Why');
      expect(parsed).toHaveProperty('Architecture');

      // And the JSON should contain section content as values
      expect(parsed.Why).toContain('Because we need it');
      expect(parsed.Architecture).toContain('System design here');
    });
  });

  describe('Scenario: Write output to file', () => {
    it('should write content to file', async () => {
      // Given I have a FOUNDATION.md
      const content = `# Project Foundation

## Why

Test content.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --output foundation-copy.md`
      const result = await showFoundation({
        output: join(testDir, 'foundation-copy.md'),
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And a file "foundation-copy.md" should be created
      await expect(access(join(testDir, 'foundation-copy.md'))).resolves.toBeUndefined();

      // And it should contain the FOUNDATION.md content
      const copiedContent = await readFile(join(testDir, 'foundation-copy.md'), 'utf-8');
      expect(copiedContent).toContain('Project Foundation');
      expect(copiedContent).toContain('Why');
    });
  });

  describe('Scenario: Write specific section to file', () => {
    it('should write section to file', async () => {
      // Given I have a FOUNDATION.md with a "Why" section
      const content = `# Project Foundation

## Why

Reasoning here.

## Other

Other content.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --section Why --output why.txt`
      const result = await showFoundation({
        section: 'Why',
        output: join(testDir, 'why.txt'),
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And a file "why.txt" should be created
      await expect(access(join(testDir, 'why.txt'))).resolves.toBeUndefined();

      const fileContent = await readFile(join(testDir, 'why.txt'), 'utf-8');

      // And it should contain only the "Why" section content
      expect(fileContent).toContain('Reasoning here');
      expect(fileContent).not.toContain('Other content');
    });
  });

  describe('Scenario: Handle missing FOUNDATION.md', () => {
    it('should error on missing file', async () => {
      // Given I have no FOUNDATION.md file
      // When I run `fspec show-foundation`
      const result = await showFoundation({
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "FOUNDATION.md not found"
      expect(result.error).toMatch(/FOUNDATION\.md not found/i);
    });
  });

  describe('Scenario: Handle missing section', () => {
    it('should error on missing section', async () => {
      // Given I have a FOUNDATION.md without a "Missing Section"
      const content = `# Project Foundation

## Why

Some content.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --section "Missing Section"`
      const result = await showFoundation({
        section: 'Missing Section',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Section 'Missing Section' not found"
      expect(result.error).toMatch(/Section 'Missing Section' not found/i);
    });
  });

  describe('Scenario: Display section with subsections', () => {
    it('should include subsections in output', async () => {
      // Given I have an "Architecture" section with diagrams (### subsections)
      const content = `# Project Foundation

## Architecture

Main architecture content.

### System Diagram

\`\`\`mermaid
graph TD
  A-->B
\`\`\`

### Component Diagram

Another diagram here.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --section Architecture`
      const result = await showFoundation({
        section: 'Architecture',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should include the main section content
      expect(result.output).toContain('Main architecture content');

      // And the output should include all subsections (### headers)
      expect(result.output).toContain('System Diagram');
      expect(result.output).toContain('Component Diagram');

      // And the output should include diagram content
      expect(result.output).toContain('graph TD');
      expect(result.output).toContain('A-->B');
    });
  });

  describe('Scenario: Display preserves formatting', () => {
    it('should preserve markdown lists', async () => {
      // Given I have a "Features" section with markdown lists
      const content = `# Project Foundation

## Features

- Feature 1
- Feature 2
  - Nested item
- Feature 3
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --section Features`
      const result = await showFoundation({
        section: 'Features',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should preserve list formatting
      expect(result.output).toContain('- Feature 1');
      expect(result.output).toContain('- Feature 2');

      // And the output should preserve indentation
      expect(result.output).toContain('  - Nested item');
    });
  });

  describe('Scenario: JSON output includes all sections', () => {
    it('should have all sections in JSON', async () => {
      // Given I have FOUNDATION.md with 5 sections
      const content = `# Project Foundation

## Section 1

Content 1

## Section 2

Content 2

## Section 3

Content 3

## Section 4

Content 4

## Section 5

Content 5
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --format json`
      const result = await showFoundation({
        format: 'json',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const parsed = JSON.parse(result.output!);

      // And the JSON should have 5 top-level keys
      expect(Object.keys(parsed)).toHaveLength(5);

      // And each key should correspond to a section name
      expect(parsed).toHaveProperty('Section 1');
      expect(parsed).toHaveProperty('Section 2');
      expect(parsed).toHaveProperty('Section 3');
      expect(parsed).toHaveProperty('Section 4');
      expect(parsed).toHaveProperty('Section 5');
    });
  });

  describe('Scenario: Display section names only', () => {
    it('should list section names without content', async () => {
      // Given I have a FOUNDATION.md with multiple sections
      const content = `# Project Foundation

## What We Are Building

Content here.

## Why

More content.

## Architecture

Even more content.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --list-sections`
      const result = await showFoundation({
        listSections: true,
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should list all section names
      expect(result.output).toContain('What We Are Building');
      expect(result.output).toContain('Why');
      expect(result.output).toContain('Architecture');

      // And section content should not be displayed
      expect(result.output).not.toContain('Content here');
      expect(result.output).not.toContain('More content');
      expect(result.output).not.toContain('Even more content');
    });
  });

  describe('Scenario: Display with line numbers', () => {
    it('should include line numbers in output', async () => {
      // Given I have a FOUNDATION.md
      const content = `# Project Foundation

## Why

Line 1
Line 2
Line 3
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --line-numbers`
      const result = await showFoundation({
        lineNumbers: true,
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should include line numbers
      expect(result.output).toMatch(/\d+:/);

      // And the format should be "N: content"
      expect(result.output).toMatch(/\d+: # Project Foundation/);
    });
  });

  describe('Scenario: Handle special characters in section names', () => {
    it('should handle apostrophes in section names', async () => {
      // Given I have a section named "What We're Building"
      const content = `# Project Foundation

## What We're Building

Content with apostrophe.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec show-foundation --section "What We're Building"`
      const result = await showFoundation({
        section: "What We're Building",
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the output should display the section content
      expect(result.output).toContain('Content with apostrophe');
    });
  });
});
