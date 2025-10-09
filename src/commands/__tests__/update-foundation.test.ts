import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile, access } from 'fs/promises';
import { join } from 'path';
import { updateFoundation } from '../update-foundation';

describe('Feature: Update Foundation Section Content', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = join(process.cwd(), 'test-tmp-update-foundation');
    await mkdir(testDir, { recursive: true });
    await mkdir(join(testDir, 'spec'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Update existing section content', () => {
    it('should update existing section and preserve others', async () => {
      // Given I have a FOUNDATION.md with a "What We Are Building" section
      const content = `# Project Foundation

## What We Are Building

Old content here.

## Why

Some reasoning.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec update-foundation "What We Are Building" "New content for this section"`
      const result = await updateFoundation({
        section: 'What We Are Building',
        content: 'New content for this section',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the "What We Are Building" section should contain the new content
      expect(updatedContent).toContain('New content for this section');

      // And other sections should be preserved
      expect(updatedContent).toContain('## Why');
      expect(updatedContent).toContain('Some reasoning.');

      // And the old content should be replaced
      expect(updatedContent).not.toContain('Old content here.');
    });
  });

  describe("Scenario: Create new section if it doesn't exist", () => {
    it('should create new section with content', async () => {
      // Given I have a FOUNDATION.md without a "Technical Approach" section
      const content = `# Project Foundation

## What We Are Building

Some content.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec update-foundation "Technical Approach" "Our technical approach details"`
      const result = await updateFoundation({
        section: 'Technical Approach',
        content: 'Our technical approach details',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And a new "Technical Approach" section should be created
      expect(updatedContent).toContain('## Technical Approach');

      // And it should contain the specified content
      expect(updatedContent).toContain('Our technical approach details');
    });
  });

  describe('Scenario: Replace entire section content', () => {
    it('should completely replace section content', async () => {
      // Given I have a "Why" section with existing content
      const content = `# Project Foundation

## Why

Original content line 1.
Original content line 2.
Original content line 3.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec update-foundation Why "Completely new reasoning"`
      const result = await updateFoundation({
        section: 'Why',
        content: 'Completely new reasoning',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the old content should be completely replaced
      expect(updatedContent).not.toContain('Original content line 1');
      expect(updatedContent).not.toContain('Original content line 2');
      expect(updatedContent).not.toContain('Original content line 3');

      // And only the new content should be present in the section
      expect(updatedContent).toContain('Completely new reasoning');
    });
  });

  describe('Scenario: Preserve other sections when updating', () => {
    it('should preserve unchanged sections', async () => {
      // Given I have FOUNDATION.md with "What We Are Building", "Why", and "Architecture" sections
      const content = `# Project Foundation

## What We Are Building

Building a CLI tool.

## Why

Because we need it.

## Architecture

System architecture details.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec update-foundation Why "Updated why section"`
      const result = await updateFoundation({
        section: 'Why',
        content: 'Updated why section',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the "What We Are Building" section should be unchanged
      expect(updatedContent).toContain('Building a CLI tool.');

      // And the "Architecture" section should be unchanged
      expect(updatedContent).toContain('System architecture details.');

      // And only the "Why" section should have new content
      expect(updatedContent).toContain('Updated why section');
      expect(updatedContent).not.toContain('Because we need it.');
    });
  });

  describe("Scenario: Create FOUNDATION.md if it doesn't exist", () => {
    it('should create FOUNDATION.md with section', async () => {
      // Given I have no FOUNDATION.md file
      // When I run `fspec update-foundation "What We Are Building" "A new CLI tool for specifications"`
      const result = await updateFoundation({
        section: 'What We Are Building',
        content: 'A new CLI tool for specifications',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And a FOUNDATION.md file should be created
      await expect(
        access(join(testDir, 'spec/FOUNDATION.md'))
      ).resolves.toBeUndefined();

      const content = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And it should contain the "What We Are Building" section
      expect(content).toContain('## What We Are Building');

      // And the section should have the specified content
      expect(content).toContain('A new CLI tool for specifications');
    });
  });

  describe('Scenario: Handle multi-line section content', () => {
    it('should preserve all lines in multi-line content', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(
        join(testDir, 'spec/FOUNDATION.md'),
        '# Project Foundation\n'
      );

      // When I run `fspec update-foundation Why "Line 1\nLine 2\nLine 3"`
      const result = await updateFoundation({
        section: 'Why',
        content: 'Line 1\nLine 2\nLine 3',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const content = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the "Why" section should contain all three lines
      expect(content).toContain('Line 1');
      expect(content).toContain('Line 2');
      expect(content).toContain('Line 3');

      // And the lines should be properly formatted
      expect(content).toMatch(/Line 1\s+Line 2\s+Line 3/);
    });
  });

  describe('Scenario: Preserve existing subsections in other sections', () => {
    it('should preserve diagrams in other sections', async () => {
      // Given I have an "Architecture" section with diagrams (### subsections)
      const content = `# Project Foundation

## Architecture

### System Diagram

\`\`\`mermaid
graph TD
  A-->B
\`\`\`

## Why

Original reasoning.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec update-foundation Why "New content"`
      const result = await updateFoundation({
        section: 'Why',
        content: 'New content',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the "Architecture" section diagrams should be preserved
      expect(updatedContent).toContain('### System Diagram');
      expect(updatedContent).toContain('graph TD');
      expect(updatedContent).toContain('A-->B');

      // And only the "Why" section should be modified
      expect(updatedContent).toContain('New content');
      expect(updatedContent).not.toContain('Original reasoning.');
    });
  });

  describe('Scenario: Update section at the beginning of file', () => {
    it('should update first section correctly', async () => {
      // Given I have FOUNDATION.md with "Overview" as the first section
      const content = `# Project Foundation

## Overview

Original overview.

## Details

Some details.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec update-foundation Overview "Updated overview"`
      const result = await updateFoundation({
        section: 'Overview',
        content: 'Updated overview',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the "Overview" section should have the new content
      expect(updatedContent).toContain('Updated overview');
      expect(updatedContent).not.toContain('Original overview.');

      // And sections after it should be preserved
      expect(updatedContent).toContain('## Details');
      expect(updatedContent).toContain('Some details.');
    });
  });

  describe('Scenario: Update section at the end of file', () => {
    it('should update last section correctly', async () => {
      // Given I have FOUNDATION.md with "Future Plans" as the last section
      const content = `# Project Foundation

## Current State

Current status.

## Future Plans

Original plans.
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec update-foundation "Future Plans" "Updated plans"`
      const result = await updateFoundation({
        section: 'Future Plans',
        content: 'Updated plans',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the "Future Plans" section should have the new content
      expect(updatedContent).toContain('Updated plans');
      expect(updatedContent).not.toContain('Original plans.');

      // And sections before it should be preserved
      expect(updatedContent).toContain('## Current State');
      expect(updatedContent).toContain('Current status.');
    });
  });

  describe('Scenario: Reject empty section name', () => {
    it('should reject empty section name', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(
        join(testDir, 'spec/FOUNDATION.md'),
        '# Project Foundation\n'
      );

      // When I run `fspec update-foundation "" "Some content"`
      const result = await updateFoundation({
        section: '',
        content: 'Some content',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Section name cannot be empty"
      expect(result.error).toMatch(/section name cannot be empty/i);
    });
  });

  describe('Scenario: Reject empty content', () => {
    it('should reject empty content', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(
        join(testDir, 'spec/FOUNDATION.md'),
        '# Project Foundation\n'
      );

      // When I run `fspec update-foundation Why ""`
      const result = await updateFoundation({
        section: 'Why',
        content: '',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Section content cannot be empty"
      expect(result.error).toMatch(/section content cannot be empty/i);
    });
  });

  describe('Scenario: Handle special characters in section names', () => {
    it('should handle apostrophes in section names', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(
        join(testDir, 'spec/FOUNDATION.md'),
        '# Project Foundation\n'
      );

      // When I run `fspec update-foundation "What We're Building" "Content with apostrophe"`
      const result = await updateFoundation({
        section: "What We're Building",
        content: 'Content with apostrophe',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const content = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the section "What We're Building" should be created
      expect(content).toContain("## What We're Building");

      // And it should contain the specified content
      expect(content).toContain('Content with apostrophe');
    });
  });

  describe('Scenario: Preserve markdown formatting in content', () => {
    it('should preserve markdown list formatting', async () => {
      // Given I have a FOUNDATION.md
      await writeFile(
        join(testDir, 'spec/FOUNDATION.md'),
        '# Project Foundation\n'
      );

      // When I run `fspec update-foundation Features "- Feature 1\n- Feature 2\n- Feature 3"`
      const result = await updateFoundation({
        section: 'Features',
        content: '- Feature 1\n- Feature 2\n- Feature 3',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const content = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the "Features" section should contain a markdown list
      expect(content).toContain('- Feature 1');
      expect(content).toContain('- Feature 2');
      expect(content).toContain('- Feature 3');

      // And the list formatting should be preserved
      expect(content).toMatch(/- Feature 1\s+- Feature 2\s+- Feature 3/);
    });
  });

  describe('Scenario: Update section multiple times', () => {
    it('should replace content on multiple updates', async () => {
      // Given I have a "Why" section with content "Original"
      const content = `# Project Foundation

## Why

Original
`;

      await writeFile(join(testDir, 'spec/FOUNDATION.md'), content);

      // When I run `fspec update-foundation Why "First update"`
      const result1 = await updateFoundation({
        section: 'Why',
        content: 'First update',
        cwd: testDir,
      });

      expect(result1.success).toBe(true);

      // And I run `fspec update-foundation Why "Second update"`
      const result2 = await updateFoundation({
        section: 'Why',
        content: 'Second update',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result2.success).toBe(true);

      const updatedContent = await readFile(
        join(testDir, 'spec/FOUNDATION.md'),
        'utf-8'
      );

      // And the "Why" section should contain only "Second update"
      expect(updatedContent).toContain('Second update');

      // And previous content should not be present
      expect(updatedContent).not.toContain('Original');
      expect(updatedContent).not.toContain('First update');
    });
  });
});
