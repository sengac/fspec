/**
 * Test suite for: spec/features/report-bug-to-github-with-ai-assistance.feature
 *                  spec/features/report-bug-to-github-project-root-handling.feature
 *
 * Tests for the report-bug-to-github command that provides AI-assisted
 * bug reporting to GitHub with automatic context gathering.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdtemp, rm, mkdir, writeFile } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import git from 'isomorphic-git';
import fs from 'fs';
import { reportBugToGitHub } from '../report-bug-to-github';
import type { BugReportContext, BugReport } from '../report-bug-to-github';

describe('Feature: Report bug to GitHub with AI assistance', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-test-'));
    await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Basic bug report flow', () => {
    it('should complete basic bug report flow', async () => {
      // @step Given I am in a project using fspec
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });

      // @step And I have encountered a bug
      const bugDescription = 'Command crashes with ENOENT';
      process.chdir(testDir);

      // @step When I run "fspec report-bug-to-github"
      const mockPrompt = vi.fn().mockResolvedValue('Test bug description');
      const mockConfirm = vi.fn().mockResolvedValue(true);
      const mockOpenBrowser = vi.fn().mockResolvedValue(undefined);

      const result = await reportBugToGitHub({
        projectRoot: testDir,
        interactive: true,
        prompt: mockPrompt,
        confirm: mockConfirm,
        openBrowser: mockOpenBrowser,
        bugDescription: bugDescription,
      });

      // @step Then the command should gather system context automatically
      expect(result.context).toBeDefined();
      expect(result.context.fspecVersion).toBeDefined();
      expect(result.context.nodeVersion).toBeDefined();
      expect(result.context.platform).toBeDefined();

      // @step And the command should prompt me interactively for bug details
      expect(mockPrompt).toHaveBeenCalled();

      // @step And the command should generate a complete bug report with markdown formatting
      expect(result.markdown).toBeDefined();
      expect(result.markdown).toContain('## Description');
      expect(result.markdown).toContain('## Expected Behavior');
      expect(result.markdown).toContain('## Actual Behavior');
      expect(result.markdown).toContain('## Steps to Reproduce');
      expect(result.markdown).toContain('## Environment');

      // @step And the command should display a preview of the issue
      expect(result.previewShown).toBe(true);

      // @step And the command should ask for confirmation
      expect(mockConfirm).toHaveBeenCalled();

      // @step And the command should open my browser to GitHub with pre-filled issue
      expect(mockOpenBrowser).toHaveBeenCalled();
      const url = mockOpenBrowser.mock.calls[0][0];
      expect(url).toContain('https://github.com');
      expect(url).toContain('/issues/new');
      expect(url).toContain('title=');
      expect(url).toContain('body=');
      // Check for URL-encoded labels (comma becomes %2C)
      expect(url).toMatch(/labels=bug(%2C|,)needs-triage/);
    });
  });

  describe('Scenario: Include work unit context', () => {
    it('should include work unit context', async () => {
      // @step Given I am working on work unit AUTH-001
      await mkdir(join(testDir, 'spec'), { recursive: true });
      const workUnitsData = {
        meta: { version: '1.0.0', lastUpdated: new Date().toISOString() },
        workUnits: {
          'AUTH-001': {
            id: 'AUTH-001',
            title: 'User Authentication',
            status: 'implementing',
            createdAt: new Date().toISOString(),
            updatedAt: new Date().toISOString(),
            children: [],
          },
        },
      };
      await writeFile(
        join(testDir, 'spec', 'work-units.json'),
        JSON.stringify(workUnitsData, null, 2)
      );

      // @step And the bug occurs while implementing AUTH-001
      await writeFile(
        join(testDir, 'spec', 'features', 'user-auth.feature'),
        '@AUTH-001\nFeature: User Authentication\n'
      );
      process.chdir(testDir);

      // @step When I run "fspec report-bug-to-github"
      const bugReport = await reportBugToGitHub({ projectRoot: testDir });

      // @step Then the bug report should include the work unit ID "AUTH-001"
      expect(bugReport.markdown).toContain('AUTH-001');

      // @step And the bug report should include the work unit title
      expect(bugReport.markdown).toContain('User Authentication');

      // @step And the bug report should include the current work unit status
      expect(bugReport.markdown).toContain('implementing');

      // @step And the bug report should include a link to the related feature file if it exists
      expect(bugReport.markdown).toContain('spec/features/user-auth.feature');
    });
  });

  describe('Scenario: Include git context', () => {
    it('should include git context', async () => {
      // @step Given I have uncommitted changes in my working directory
      // Initialize a proper git repository using isomorphic-git
      await git.init({ fs, dir: testDir, defaultBranch: 'feature-branch' });

      // Create and commit a file to establish the repository
      await writeFile(join(testDir, 'initial.txt'), 'initial content');
      await git.add({ fs, dir: testDir, filepath: 'initial.txt' });
      await git.commit({
        fs,
        dir: testDir,
        message: 'Initial commit',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // Create an uncommitted file
      await writeFile(join(testDir, 'test.txt'), 'uncommitted content');

      // @step And the bug reproduces with these changes
      // Simulated by having uncommitted file
      process.chdir(testDir);

      // @step When I run "fspec report-bug-to-github"
      const bugReport = await reportBugToGitHub({ projectRoot: testDir });

      // @step Then the bug report should include the current git branch
      expect(bugReport.markdown).toContain('feature-branch');

      // @step And the bug report should indicate there are uncommitted changes
      expect(bugReport.markdown).toMatch(
        /uncommitted changes|working directory changes/i
      );

      // @step And the bug report should note about providing git diff if needed
      expect(bugReport.markdown).toMatch(/git diff|provide diff/i);
    });
  });

  describe('Scenario: Error log capture', () => {
    it('should capture error logs', async () => {
      // @step Given fspec recently crashed with an error
      await mkdir(join(testDir, '.fspec', 'error-logs'), { recursive: true });
      const errorLog = {
        timestamp: new Date().toISOString(),
        error: 'ENOENT: no such file or directory',
        stack: 'Error: ENOENT\n    at Function.module.exports.readFileSync',
      };
      await writeFile(
        join(testDir, '.fspec', 'error-logs', 'error-latest.json'),
        JSON.stringify(errorLog)
      );
      process.chdir(testDir);

      // @step When I run "fspec report-bug-to-github"
      const bugReport = await reportBugToGitHub({ projectRoot: testDir });

      // @step Then the command should detect recent error logs
      expect(bugReport.context.recentErrors).toBeDefined();
      expect(bugReport.context.recentErrors.length).toBeGreaterThan(0);

      // @step And the command should include the stack trace if available
      expect(bugReport.markdown).toContain(
        'Function.module.exports.readFileSync'
      );

      // @step And the command should include the error message in the bug report
      expect(bugReport.markdown).toContain('ENOENT: no such file or directory');
    });
  });

  describe('Scenario: Preview and edit', () => {
    it('should support preview and edit', async () => {
      // @step Given the AI has generated a bug report
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      process.chdir(testDir);
      const initialBugReport = await reportBugToGitHub({
        projectRoot: testDir,
        generateOnly: true,
      });

      // @step When I review the preview
      // @step And I want to add additional context
      const mockEditTitle = vi.fn().mockResolvedValue('Updated Bug Title');
      const mockEditBody = vi
        .fn()
        .mockResolvedValue('Updated bug body content');
      const mockConfirm = vi.fn().mockResolvedValue(true);
      const mockOpenBrowser = vi.fn().mockResolvedValue(undefined);

      // @step Then I should be able to edit the title before submission
      const result = await reportBugToGitHub({
        projectRoot: testDir,
        interactive: true,
        editTitle: mockEditTitle,
        initialReport: initialBugReport,
      });
      expect(mockEditTitle).toHaveBeenCalled();
      expect(result.title).toBe('Updated Bug Title');

      // @step And I should be able to edit the body before submission
      const result2 = await reportBugToGitHub({
        projectRoot: testDir,
        interactive: true,
        editBody: mockEditBody,
        initialReport: initialBugReport,
      });
      expect(mockEditBody).toHaveBeenCalled();
      expect(result2.markdown).toContain('Updated bug body content');

      // @step And I should be able to cancel the submission
      const mockConfirmCancel = vi.fn().mockResolvedValue(false);
      const result3 = await reportBugToGitHub({
        projectRoot: testDir,
        interactive: true,
        confirm: mockConfirmCancel,
      });
      expect(mockConfirmCancel).toHaveBeenCalled();
      expect(result3.cancelled).toBe(true);
      expect(result3.browserOpened).toBe(false);

      // @step And I should be able to confirm and open the browser
      const result4 = await reportBugToGitHub({
        projectRoot: testDir,
        interactive: true,
        confirm: mockConfirm,
        openBrowser: mockOpenBrowser,
      });
      expect(mockConfirm).toHaveBeenCalled();
      expect(mockOpenBrowser).toHaveBeenCalled();
      expect(result4.browserOpened).toBe(true);
    });
  });

  describe('Scenario: URL encoding handling', () => {
    it('should handle URL encoding', async () => {
      // @step Given the bug report contains special characters
      await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
      process.chdir(testDir);
      const specialContent = 'Bug with "quotes" & <brackets> and spaces';

      // @step And the report contains markdown code blocks
      const codeBlock =
        '```typescript\nfunction test(): void {\n  console.log("test");\n}\n```';

      // @step When constructing the GitHub URL
      const mockOpenBrowser = vi.fn().mockResolvedValue(undefined);
      await reportBugToGitHub({
        projectRoot: testDir,
        bugDescription: specialContent + '\n' + codeBlock,
        openBrowser: mockOpenBrowser,
      });

      // @step Then all content should be properly URL-encoded
      const url = mockOpenBrowser.mock.calls[0][0];
      expect(url).not.toContain('"');
      expect(url).not.toContain('<');
      expect(url).not.toContain('>');
      expect(url).toContain('%22'); // encoded quote
      expect(url).toContain('%3C'); // encoded <
      expect(url).toContain('%3E'); // encoded >

      // @step And markdown formatting should be preserved
      const decodedBody = decodeURIComponent(
        url.split('body=')[1].split('&')[0]
      );
      expect(decodedBody).toContain('```typescript');
      expect(decodedBody).toContain('##'); // Headers preserved

      // @step And code blocks should render correctly in GitHub
      expect(decodedBody).toContain('function test(): void');
      expect(decodedBody).toContain('console.log("test")');
      // Verify code block has proper closing
      expect(decodedBody).toMatch(/```\s*\n\n## Expected Behavior/);
    });
  });

  describe('Feature: BUG-058 - Project root handling', () => {
    describe('Scenario: After fix - command works without --project-root using process.cwd()', () => {
      it('should use process.cwd() when projectRoot is not provided', async () => {
        // @step Given I am in a project directory
        await mkdir(join(testDir, 'spec', 'features'), { recursive: true });
        process.chdir(testDir);

        // @step And findProjectRoot() is called with process.cwd() parameter
        // @step When I run "fspec report-bug-to-github"
        const mockOpenBrowser = vi.fn().mockResolvedValue(undefined);
        const result = await reportBugToGitHub({
          // No projectRoot option - should use process.cwd()
          bugDescription: 'test bug',
          openBrowser: mockOpenBrowser,
        });

        // @step Then the command should gather system context successfully
        expect(result.context).toBeDefined();
        expect(result.context.fspecVersion).toBeDefined();
        expect(result.context.nodeVersion).toBeDefined();

        // @step And the command should use process.cwd() to find project root
        // @step And the browser should open with pre-filled bug report
        expect(mockOpenBrowser).toHaveBeenCalled();
      });
    });
  });
});
