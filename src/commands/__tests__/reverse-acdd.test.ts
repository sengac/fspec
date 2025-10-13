/**
 * Feature: spec/features/reverse-acdd-for-existing-codebases.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile, readFile, access } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { init } from '../init';

describe('Feature: Reverse ACDD for Existing Codebases', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Install rspec.md via fspec init', () => {
    it('should create rspec.md with proper content and link to fspec.md', async () => {
      // Given I run "fspec init" to set up a new project
      // When the init command completes
      await init({ cwd: testDir, installType: 'claude-code' });

      // Then a file ".claude/commands/rspec.md" should be created
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      await access(rspecFile); // Throws if file doesn't exist

      // And the first line should link to fspec.md: "fully read fspec.md"
      const content = await readFile(rspecFile, 'utf-8');
      const firstLine = content.split('\n')[0];
      expect(firstLine.toLowerCase()).toContain('fully read fspec.md');

      // And the file should contain reverse engineering workflow instructions
      expect(content).toContain('reverse engineer');
      expect(content).toContain('user stories');
      expect(content).toContain('acceptance criteria');

      // And the file should include examples of identifying user stories from code
      expect(content).toContain('example');
      expect(content).toContain('epic');
    });
  });

  describe('Scenario: Invoke rspec slash command in Claude Code', () => {
    it('should provide instructions for Claude to read rspec.md and fspec.md', async () => {
      // Given I have an existing codebase without specifications
      // And I have run "fspec init" to install rspec.md
      await init({ cwd: testDir, installType: 'claude-code' });

      // When I run "/rspec" in Claude Code
      // Then Claude should read rspec.md
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // And Claude should read fspec.md
      expect(content).toContain('fspec.md');

      // And Claude should begin analyzing the codebase for user interactions
      expect(content.toLowerCase()).toContain('analyze');
      expect(content.toLowerCase()).toContain('codebase');
    });
  });

  describe('Scenario: Identify user-facing interactions from codebase', () => {
    it('should guide Claude to identify user interactions from routes', async () => {
      // Given I have an Express.js application with routes
      // And the application has routes: POST /login, GET /dashboard, POST /checkout
      await init({ cwd: testDir, installType: 'claude-code' });

      // When I run "/rspec"
      // Then Claude should identify user interactions
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Instructions should guide identification of interactions
      expect(content).toContain('routes');
      expect(content).toContain('interaction');
    });
  });

  describe('Scenario: Group interactions into epics', () => {
    it('should guide Claude to group interactions into epics', async () => {
      // Given Claude has identified user interactions for authentication, payments, and dashboards
      await init({ cwd: testDir, installType: 'claude-code' });

      // When Claude groups interactions into epics
      // Then the following epics should be created
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Instructions should mention epics and grouping
      expect(content).toContain('epic');
      expect(content).toContain('group');
    });
  });

  describe('Scenario: Create work units for each user story', () => {
    it('should guide Claude to create work units using fspec commands', async () => {
      // Given Claude has created epics: AUTH, PAY, DASH
      // And Claude has identified user stories within each epic
      await init({ cwd: testDir, installType: 'claude-code' });

      // When Claude creates work units
      // Then the following work units should be created
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Instructions should reference work unit creation
      expect(content).toContain('work unit');
      expect(content).toContain('create-work-unit');
    });
  });

  describe('Scenario: Generate feature files from inferred acceptance criteria', () => {
    it('should guide Claude to generate feature files with inferred scenarios', async () => {
      // Given Claude has created work unit AUTH-001 for "User Login"
      // And Claude has analyzed the login route implementation
      await init({ cwd: testDir, installType: 'claude-code' });

      // When Claude generates the feature file
      // Then a file "spec/features/user-login.feature" should be created
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Instructions should guide feature file generation
      expect(content).toContain('feature file');
      expect(content).toContain('scenario');
      expect(content).toContain('infer');
    });
  });

  describe('Scenario: Create skeleton test files with feature links', () => {
    it('should guide Claude to create skeleton test files with proper headers', async () => {
      // Given Claude has created feature file "spec/features/user-login.feature"
      await init({ cwd: testDir, installType: 'claude-code' });

      // When Claude creates the skeleton test file
      // Then a file "src/routes/__tests__/login.test.ts" should be created
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Instructions should mention test skeleton creation
      expect(content).toContain('test');
      expect(content).toContain('skeleton');
      expect(content).toContain('header');
    });
  });

  describe('Scenario: Update foundation.json with user story maps', () => {
    it('should guide Claude to update foundation.json with Mermaid diagrams', async () => {
      // Given Claude has identified user stories and epics
      await init({ cwd: testDir, installType: 'claude-code' });

      // When Claude updates foundation.json
      // Then foundation.json should contain a user story map section
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Instructions should mention foundation.json and user story maps
      expect(content).toContain('foundation.json');
      expect(content.toLowerCase()).toContain('user story map');
      expect(content.toLowerCase()).toContain('mermaid');
    });
  });

  describe('Scenario: Handle ambiguous code with Example Mapping', () => {
    it('should guide Claude to use Example Mapping for unclear business logic', async () => {
      // Given Claude encounters ambiguous business logic in the checkout flow
      // And the code contains magic numbers and unclear conditional branches
      await init({ cwd: testDir, installType: 'claude-code' });

      // When Claude attempts to reverse engineer the acceptance criteria
      // Then Claude should document what is clear from the code
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Instructions should mention handling ambiguity
      expect(content).toContain('ambiguous');
      expect(content).toContain('Example Mapping');
      expect(content).toContain('clarify');
    });
  });

  describe('Scenario: Completion criteria - all user interactions documented', () => {
    it('should provide completion criteria for reverse engineering', async () => {
      // Given Claude has analyzed the entire codebase
      await init({ cwd: testDir, installType: 'claude-code' });

      // When Claude determines if reverse engineering is complete
      // Then all user-facing interactions should have feature files
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Instructions should define completion criteria
      expect(content).toContain('complete');
      expect(content).toContain('all');
    });
  });

  describe('Scenario: AI agent workflow - reverse then forward ACDD', () => {
    it('should establish workflow for transitioning from reverse to forward ACDD', async () => {
      // Given I have an existing Express.js application without specifications
      await init({ cwd: testDir, installType: 'claude-code' });

      // When I run "/rspec" in Claude Code
      // Then Claude creates all specifications using reverse ACDD
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Instructions should explain forward ACDD transition
      expect(content).toContain('ACDD');
      expect(content).toContain('workflow');
    });
  });

  describe('Scenario: rspec.md content structure', () => {
    it('should have proper sections for reverse engineering guidance', async () => {
      // Given I run "fspec init"
      await init({ cwd: testDir, installType: 'claude-code' });

      // Then rspec.md should have structured content
      const rspecFile = join(testDir, '.claude', 'commands', 'rspec.md');
      const content = await readFile(rspecFile, 'utf-8');

      // Verify structure
      expect(content).toContain('fspec.md'); // Link to fspec
      expect(content.length).toBeGreaterThan(500); // Substantial content

      // Should explain the process
      const sections = content.split('\n\n');
      expect(sections.length).toBeGreaterThan(3); // Multiple sections
    });
  });
});
