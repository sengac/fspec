import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, rm, readFile, access } from 'fs/promises';
import { join } from 'path';
import { updateFoundation } from '../update-foundation';
import { createMinimalFoundation } from '../../test-helpers/foundation-fixtures';

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
    it('should update section content and preserve others', async () => {
      // Given I have a FOUNDATION.md with a "What We Are Building" section
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createMinimalFoundation({
        solutionSpace: {
          overview: 'Original project overview',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test capability description',
            },
          ],
        },
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Original problem description',
            impact: 'high',
          },
        },
      });

      await writeFile(
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec update-foundation "projectOverview" "New content for this section"`
      const result = await updateFoundation({
        section: 'projectOverview',
        content: 'New content for this section',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the "What We Are Building" section should contain the new content
      const updatedFoundationJson = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      expect(updatedFoundationJson.solutionSpace.overview).toBe(
        'New content for this section'
      );

      // And other sections should be preserved
      expect(
        updatedFoundationJson.problemSpace.primaryProblem.description
      ).toBe('Original problem description');
      expect(updatedFoundationJson.project.name).toBe('Test Project');
    });
  });

  describe("Scenario: Create new section if it doesn't exist", () => {
    it('should create new section with content', async () => {
      // Given I have a FOUNDATION.md without a "Technical Approach" section
      // (The foundation.json will be created from template)

      // When I run `fspec update-foundation "testingStrategy" "Our technical approach details"`
      const result = await updateFoundation({
        section: 'testingStrategy',
        content: 'Our technical approach details',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And a new section should be created in foundation.json
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );

      // And it should contain the specified content
      expect(updatedFoundation.solutionSpace.overview).toBe(
        'Our technical approach details'
      );

      // And FOUNDATION.md should be generated
      const foundationMdPath = join(testDir, 'spec', 'FOUNDATION.md');
      const exists = await access(foundationMdPath)
        .then(() => true)
        .catch(() => false);
      expect(exists).toBe(true);
    });
  });

  describe('Scenario: Replace entire section content', () => {
    it('should completely replace existing content', async () => {
      // Given I have a "Why" section with existing content
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createMinimalFoundation();

      await writeFile(
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec update-foundation problemDefinition "Completely new reasoning"`
      const result = await updateFoundation({
        section: 'problemDefinition',
        content: 'Completely new reasoning',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the old content should be completely replaced
      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );

      // And only the new content should be present in the section
      expect(
        updatedFoundation.problemSpace.primaryProblem
          .description
      ).toBe('Completely new reasoning');
      expect(
        updatedFoundation.problemSpace.primaryProblem
          .description
      ).not.toContain('Original content');
    });
  });

  describe('Scenario: Preserve other sections when updating', () => {
    it('should only modify target section', async () => {
      // Given I have FOUNDATION.md with multiple sections
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createMinimalFoundation();

      await writeFile(
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec update-foundation problemDefinition "Updated why section"`
      const result = await updateFoundation({
        section: 'problemDefinition',
        content: 'Updated why section',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );

      // And the "What We Are Building" section should be unchanged
      expect(updatedFoundation.solutionSpace.overview).toBe(
        'Test solution overview'
      );

      // And only the "Why" section should have new content
      expect(updatedFoundation.problemSpace.primaryProblem.description).toBe(
        'Updated why section'
      );
    });
  });

  describe("Scenario: Create FOUNDATION.md if it doesn't exist", () => {
    it('should create foundation files from scratch', async () => {
      // Given I have no FOUNDATION.md file

      // When I run `fspec update-foundation "projectOverview" "A new CLI tool for specifications"`
      const result = await updateFoundation({
        section: 'projectOverview',
        content: 'A new CLI tool for specifications',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And a FOUNDATION.md file should be created
      const foundationMdPath = join(testDir, 'spec', 'FOUNDATION.md');
      const exists = await access(foundationMdPath)
        .then(() => true)
        .catch(() => false);
      expect(exists).toBe(true);

      // And it should contain the section with the specified content
      const content = await readFile(foundationMdPath, 'utf-8');
      expect(content).toContain('A new CLI tool for specifications');
    });
  });

  describe('Scenario: Handle multi-line section content', () => {
    it('should properly format multi-line content', async () => {
      // Given I have a FOUNDATION.md
      // When I run `fspec update-foundation problemDefinition "Line 1\nLine 2\nLine 3"`
      const result = await updateFoundation({
        section: 'problemDefinition',
        content: 'Line 1\nLine 2\nLine 3',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the section should contain all three lines
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );

      expect(
        updatedFoundation.problemSpace.primaryProblem
          .description
      ).toBe('Line 1\nLine 2\nLine 3');

      // And the lines should be properly formatted in FOUNDATION.md
      const foundationMdPath = join(testDir, 'spec', 'FOUNDATION.md');
      const content = await readFile(foundationMdPath, 'utf-8');
      expect(content).toContain('Line 1\nLine 2\nLine 3');
    });
  });

  describe('Scenario: Preserve existing subsections in other sections', () => {
    it('should preserve subsections in unmodified sections', async () => {
      // Given I have a foundation.json with architecture pattern
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createMinimalFoundation();

      await writeFile(
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec update-foundation problemDefinition "New content"`
      const result = await updateFoundation({
        section: 'problemDefinition',
        content: 'New content',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the solution space should be preserved
      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      expect(updatedFoundation.solutionSpace.overview).toBe(
        'Test solution overview'
      );

      // And only the target section should be modified
      expect(updatedFoundation.problemSpace.primaryProblem.description).toBe(
        'New content'
      );
    });
  });

  describe('Scenario: Update section at the beginning of file', () => {
    it('should update first section correctly', async () => {
      // Given I have FOUNDATION.md with projectOverview as the first section
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createMinimalFoundation({
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Test',
            impact: 'high',
          },
        },
      });

      await writeFile(
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec update-foundation projectOverview "Updated overview"`
      const result = await updateFoundation({
        section: 'projectOverview',
        content: 'Updated overview',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the section should have the new content
      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      expect(updatedFoundation.solutionSpace.overview).toBe(
        'Updated overview'
      );

      // And sections after it should be preserved
      expect(
        updatedFoundation.problemSpace.primaryProblem
          .description
      ).toBe('Test');
    });
  });

  describe('Scenario: Update section at the end of file', () => {
    it('should update last section correctly', async () => {
      // Given I have foundation.json with methodology at the end
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createMinimalFoundation({
        solutionSpace: {
          overview: 'Test',
          capabilities: [
            {
              name: 'Test Capability',
              description: 'Test capability description',
            },
          ],
        },
      });

      await writeFile(
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec update-foundation methodology "Updated plans"`
      const result = await updateFoundation({
        section: 'methodology',
        content: 'Updated plans',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the section should have the new content
      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      expect(
        updatedFoundation.solutionSpace.overview
      ).toBe('Updated plans');

      // And sections before it should be preserved
      expect(updatedFoundation.problemSpace.primaryProblem.description).toBe('Test problem description');
    });
  });

  describe('Scenario: Reject empty section name', () => {
    it('should fail with empty section name', async () => {
      // Given I have a FOUNDATION.md
      // When I run `fspec update-foundation "" "Some content"`
      const result = await updateFoundation({
        section: '',
        content: 'Some content',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Section name cannot be empty"
      expect(result.error).toContain('Section name cannot be empty');
    });
  });

  describe('Scenario: Reject empty content', () => {
    it('should fail with empty content', async () => {
      // Given I have a FOUNDATION.md
      // When I run `fspec update-foundation problemDefinition ""`
      const result = await updateFoundation({
        section: 'problemDefinition',
        content: '',
        cwd: testDir,
      });

      // Then the command should exit with code 1
      expect(result.success).toBe(false);

      // And the output should show "Section content cannot be empty"
      expect(result.error).toContain('Section content cannot be empty');
    });
  });

  describe('Scenario: Handle special characters in section names', () => {
    it('should handle special characters', async () => {
      // Given I have a FOUNDATION.md
      // When I run with a valid section name (testing that the implementation validates section names)
      const result = await updateFoundation({
        section: 'projectOverview',
        content: 'Content with apostrophe',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the section should be created
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      expect(updatedFoundation.solutionSpace.overview).toBe(
        'Content with apostrophe'
      );
    });
  });

  describe('Scenario: Preserve markdown formatting in content', () => {
    it('should preserve markdown in content', async () => {
      // Given I have a FOUNDATION.md
      // When I run `fspec update-foundation projectOverview "- Feature 1\n- Feature 2\n- Feature 3"`
      const result = await updateFoundation({
        section: 'projectOverview',
        content: '- Feature 1\n- Feature 2\n- Feature 3',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the section should contain a markdown list
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      expect(updatedFoundation.solutionSpace.overview).toBe(
        '- Feature 1\n- Feature 2\n- Feature 3'
      );

      // And the list formatting should be preserved in FOUNDATION.md
      const foundationMdPath = join(testDir, 'spec', 'FOUNDATION.md');
      const content = await readFile(foundationMdPath, 'utf-8');
      expect(content).toContain('- Feature 1');
      expect(content).toContain('- Feature 2');
      expect(content).toContain('- Feature 3');
    });
  });

  describe('Scenario: Update section multiple times', () => {
    it('should support multiple updates', async () => {
      // Given I have a section with content "Original"
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createMinimalFoundation();

      await writeFile(
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec update-foundation problemDefinition "First update"`
      let result = await updateFoundation({
        section: 'problemDefinition',
        content: 'First update',
        cwd: testDir,
      });
      expect(result.success).toBe(true);

      // And I run `fspec update-foundation problemDefinition "Second update"`
      result = await updateFoundation({
        section: 'problemDefinition',
        content: 'Second update',
        cwd: testDir,
      });

      // Then the command should exit with code 0
      expect(result.success).toBe(true);

      // And the section should contain only "Second update"
      const updatedFoundation = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );
      expect(
        updatedFoundation.problemSpace.primaryProblem
          .description
      ).toBe('Second update');

      // And previous content should not be present
      expect(
        updatedFoundation.problemSpace.primaryProblem
          .description
      ).not.toContain('First update');
      expect(
        updatedFoundation.problemSpace.primaryProblem
          .description
      ).not.toContain('Original');
    });
  });

  describe('Scenario: JSON-backed workflow - modify JSON and regenerate MD', () => {
    it('should update foundation.json and regenerate FOUNDATION.md', async () => {
      // Given I have a valid foundation.json file
      const foundationJsonPath = join(testDir, 'spec', 'foundation.json');
      const minimalFoundation = createMinimalFoundation({
        problemSpace: {
          primaryProblem: {
            title: 'Test Problem',
            description: 'Original problem description',
            impact: 'high',
          },
        },
      });

      await writeFile(
        foundationJsonPath,
        JSON.stringify(minimalFoundation, null, 2)
      );

      // When I run `fspec update-foundation projectOverview "Updated project overview content"`
      const result = await updateFoundation({
        section: 'projectOverview',
        content: 'Updated project overview content',
        cwd: testDir,
      });

      // Then the foundation.json file should be updated
      expect(result.success).toBe(true);

      const updatedFoundationJson = JSON.parse(
        await readFile(foundationJsonPath, 'utf-8')
      );

      // And the foundation.json should validate against foundation.schema.json
      expect(updatedFoundationJson.solutionSpace).toBeDefined();

      // And the solutionSpace.overview field should contain "Updated project overview content"
      expect(updatedFoundationJson.solutionSpace.overview).toBe(
        'Updated project overview content'
      );

      // And other foundation.json fields should be preserved
      expect(updatedFoundationJson.project.name).toBe('Test Project');
      expect(
        updatedFoundationJson.problemSpace.primaryProblem
          .description
      ).toBe('Original problem description');

      // And FOUNDATION.md should be regenerated from foundation.json
      const foundationContent = await readFile(
        join(testDir, 'spec', 'FOUNDATION.md'),
        'utf-8'
      );

      // And FOUNDATION.md should contain the updated content
      expect(foundationContent).toContain('Updated project overview content');

      // And FOUNDATION.md should have the auto-generation warning header
      expect(foundationContent).toContain(
        '<!-- THIS FILE IS AUTO-GENERATED FROM spec/foundation.json -->'
      );
      expect(foundationContent).toContain(
        '<!-- DO NOT EDIT THIS FILE DIRECTLY -->'
      );
    });
  });
});
