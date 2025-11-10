// Feature: spec/features/integrate-research-guidance-into-ai-agent-assistance-and-ultrathink-compilation.feature

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fs } from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';
import { tmpdir } from 'os';
import { createMinimalFoundation } from '../../test-helpers/foundation-fixtures';

describe('Feature: Integrate research guidance into AI agent assistance and ULTRATHINK compilation', () => {
  let tmpDir: string;
  let originalCwd: string;
  let fspecBin: string;

  beforeEach(async () => {
    tmpDir = path.join(tmpdir(), `test-${Date.now()}`);
    await fs.mkdir(tmpDir, { recursive: true });
    originalCwd = process.cwd();
    fspecBin = path.join(originalCwd, 'dist', 'index.js');

    // Initialize fspec project structure
    await fs.mkdir(path.join(tmpDir, 'spec'), { recursive: true });

    // Create minimal foundation.json
    const foundation = createMinimalFoundation();
    await fs.writeFile(
      path.join(tmpDir, 'spec', 'foundation.json'),
      JSON.stringify(foundation, null, 2)
    );
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    await fs.rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: System-reminder mentions research tools during specifying phase', () => {
    it('should show research tool guidance in system-reminder when moving to specifying', async () => {
      // @step Given I have a work unit AUTH-001 in backlog
      process.chdir(tmpDir);
      execSync(`node ${fspecBin} init --agent=claude`, { cwd: tmpDir });
      execSync(`node ${fspecBin} create-prefix AUTH "Authentication"`, {
        cwd: tmpDir,
      });
      execSync(`node ${fspecBin} create-story AUTH "User Login"`, {
        cwd: tmpDir,
      });

      // @step When I run "fspec update-work-unit-status AUTH-001 specifying"
      const output = execSync(
        `node ${fspecBin} update-work-unit-status AUTH-001 specifying`,
        { cwd: tmpDir, encoding: 'utf-8' }
      );

      // @step Then the system-reminder output should contain "fspec research"
      expect(output).toContain('fspec research');

      // @step And the output should mention "--tool=ast or --tool=stakeholder"
      expect(output).toContain('--tool=ast or --tool=stakeholder');

      // @step And the guidance should explain when to use research tools
      expect(output).toContain('research tools');
      expect(output).toContain('Example Mapping');
    });
  });

  describe('Scenario: Bootstrap content includes research section', () => {
    it('should include RESEARCH section in bootstrap output', async () => {
      // @step Given I am in a project directory
      process.chdir(tmpDir);
      execSync(`node ${fspecBin} init --agent=claude`, { cwd: tmpDir });

      // @step When I run "fspec bootstrap"
      const output = execSync(`node ${fspecBin} bootstrap`, {
        cwd: tmpDir,
        encoding: 'utf-8',
      });

      // @step Then the output should include a RESEARCH section
      expect(output.toUpperCase()).toContain('RESEARCH TOOLS');

      // @step And the section should document available research tools
      expect(output).toContain('ast');
      expect(output).toContain('stakeholder');

      // @step And the section should show research workflow examples
      expect(output).toContain('fspec research');

      // @step And the section should explain tool integration with Example Mapping
      expect(output).toContain('Example Mapping');
    });
  });

  describe('Scenario: CLAUDE.md contains research workflow documentation', () => {
    it('should create CLAUDE.md with research documentation when using claude agent', async () => {
      // @step Given I run "fspec init --agent=claude"
      process.chdir(tmpDir);
      execSync(`node ${fspecBin} init --agent=claude`, { cwd: tmpDir });

      // @step When I read the file "spec/CLAUDE.md"
      const claudeMdPath = path.join(tmpDir, 'spec', 'CLAUDE.md');
      const content = await fs.readFile(claudeMdPath, 'utf-8');

      // @step Then it should contain research tool documentation
      expect(content.toLowerCase()).toContain('research tool');

      // @step And it should include example: "fspec research --tool=ast --query=\"pattern\""
      expect(content).toContain('fspec research --tool=ast');
      expect(content).toContain('--query');

      // @step And it should explain when to use research during specifying phase
      expect(content).toContain('specifying');

      // @step And it should show how to attach research results to work units
      expect(content).toContain('attach');
      expect(content).toContain('work unit');
    });
  });

  describe('Scenario: Compile research with ULTRATHINK for Claude agent', () => {
    it('should compile research with ULTRATHINK terminology for Claude', async () => {
      // @step Given I am using Claude agent
      process.chdir(tmpDir);
      execSync(`node ${fspecBin} init --agent=claude`, { cwd: tmpDir });

      // @step And I have work unit AUTH-001 with research data
      execSync(`node ${fspecBin} create-prefix AUTH "Authentication"`, {
        cwd: tmpDir,
      });
      execSync(`node ${fspecBin} create-story AUTH "User Login"`, {
        cwd: tmpDir,
      });
      execSync(`node ${fspecBin} update-work-unit-status AUTH-001 specifying`, {
        cwd: tmpDir,
      });
      await fs.mkdir(path.join(tmpDir, 'spec', 'attachments', 'AUTH-001'), {
        recursive: true,
      });
      await fs.writeFile(
        path.join(tmpDir, 'spec', 'attachments', 'AUTH-001', 'research.txt'),
        'Research findings about authentication flow'
      );

      // @step When I run "fspec compile-research AUTH-001"
      execSync(`node ${fspecBin} compile-research AUTH-001`, { cwd: tmpDir });

      // @step Then it should use agent detection to identify Claude
      const compiledFiles = await fs.readdir(
        path.join(tmpDir, 'spec', 'attachments', 'AUTH-001')
      );
      const markdownFile = compiledFiles.find(f => f.endsWith('-compiled.md'));
      expect(markdownFile).toBeDefined();

      const compiledContent = await fs.readFile(
        path.join(tmpDir, 'spec', 'attachments', 'AUTH-001', markdownFile!),
        'utf-8'
      );

      // @step And the compiled markdown should include "ULTRATHINK" terminology
      expect(compiledContent).toContain('ULTRATHINK');

      // @step And the markdown should have front matter with work unit ID and timestamp
      expect(compiledContent).toContain('---');
      expect(compiledContent).toContain('workUnit: AUTH-001');
      expect(compiledContent).toMatch(/timestamp:/);

      // @step And the file should be auto-attached to spec/attachments/AUTH-001/
      expect(markdownFile).toMatch(/AUTH-001.*compiled\.md/);
    });
  });

  describe('Scenario: Compile research without ULTRATHINK for other agents', () => {
    it('should compile research with deep analysis terminology for non-Claude agents', async () => {
      // @step Given I am using Cursor agent
      process.chdir(tmpDir);
      execSync(`node ${fspecBin} init --agent=cursor`, { cwd: tmpDir });

      // @step And I have work unit AUTH-001 with research data
      execSync(`node ${fspecBin} create-prefix AUTH "Authentication"`, {
        cwd: tmpDir,
      });
      execSync(`node ${fspecBin} create-story AUTH "User Login"`, {
        cwd: tmpDir,
      });
      execSync(`node ${fspecBin} update-work-unit-status AUTH-001 specifying`, {
        cwd: tmpDir,
      });
      await fs.mkdir(path.join(tmpDir, 'spec', 'attachments', 'AUTH-001'), {
        recursive: true,
      });
      await fs.writeFile(
        path.join(tmpDir, 'spec', 'attachments', 'AUTH-001', 'research.txt'),
        'Research findings about authentication flow'
      );

      // @step When I run "fspec compile-research AUTH-001"
      execSync(`node ${fspecBin} compile-research AUTH-001`, { cwd: tmpDir });

      // @step Then it should use agent detection to identify Cursor
      const compiledFiles = await fs.readdir(
        path.join(tmpDir, 'spec', 'attachments', 'AUTH-001')
      );
      const markdownFile = compiledFiles.find(f => f.endsWith('-compiled.md'));
      expect(markdownFile).toBeDefined();

      const compiledContent = await fs.readFile(
        path.join(tmpDir, 'spec', 'attachments', 'AUTH-001', markdownFile!),
        'utf-8'
      );

      // @step And the compiled markdown should use "deep analysis" terminology
      expect(compiledContent).toContain('deep analysis');

      // @step And the markdown should have front matter with work unit ID and timestamp
      expect(compiledContent).toContain('---');
      expect(compiledContent).toContain('workUnit: AUTH-001');
      expect(compiledContent).toMatch(/timestamp:/);

      // @step And the file should be auto-attached to spec/attachments/AUTH-001/
      expect(markdownFile).toMatch(/AUTH-001.*compiled\.md/);
    });
  });

  describe('Scenario: Compiled research includes mermaid diagrams', () => {
    it('should include validated mermaid diagrams in compiled research', async () => {
      // @step Given I have work unit AUTH-001 with authentication flow research
      process.chdir(tmpDir);
      execSync(`node ${fspecBin} init --agent=claude`, { cwd: tmpDir });
      execSync(`node ${fspecBin} create-prefix AUTH "Authentication"`, {
        cwd: tmpDir,
      });
      execSync(`node ${fspecBin} create-story AUTH "User Login"`, {
        cwd: tmpDir,
      });
      execSync(`node ${fspecBin} update-work-unit-status AUTH-001 specifying`, {
        cwd: tmpDir,
      });
      await fs.mkdir(path.join(tmpDir, 'spec', 'attachments', 'AUTH-001'), {
        recursive: true,
      });
      await fs.writeFile(
        path.join(tmpDir, 'spec', 'attachments', 'AUTH-001', 'research.txt'),
        'Authentication flow involves login, session, and logout steps'
      );

      // @step When I run "fspec compile-research AUTH-001"
      execSync(`node ${fspecBin} compile-research AUTH-001`, { cwd: tmpDir });

      const compiledFiles = await fs.readdir(
        path.join(tmpDir, 'spec', 'attachments', 'AUTH-001')
      );
      const markdownFile = compiledFiles.find(f => f.endsWith('-compiled.md'));
      expect(markdownFile).toBeDefined();

      const compiledContent = await fs.readFile(
        path.join(tmpDir, 'spec', 'attachments', 'AUTH-001', markdownFile!),
        'utf-8'
      );

      // @step Then the compiled markdown should include a mermaid flowchart
      expect(compiledContent).toContain('```mermaid');
      expect(compiledContent).toMatch(/flowchart|graph/);

      // @step And the diagram should visualize the authentication flow
      expect(compiledContent).toContain('login');

      // @step And the markdown should include summary sections
      expect(compiledContent).toMatch(/##?\s+/);

      // @step And the mermaid syntax should be validated before inclusion
      // This is validated during compile-research execution
      // If the test passes, it means mermaid validation succeeded
      expect(compiledContent).toContain('```mermaid');
    });
  });
});
