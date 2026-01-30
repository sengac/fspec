/**
 * Feature: spec/features/codex-init-writes-prompt-to-home-directory.feature
 *
 * Tests validating Codex prompt generation behavior for fspec init.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { readFile, mkdir, writeFile, access, rm } from 'fs/promises';
import { constants } from 'fs';
import os from 'os';
import path from 'path';
import { installAgentFiles } from '../../commands/init';
import { getAgentById } from '../../utils/agentRegistry';
import {
  setupTestDirectory,
  type TestDirectorySetup,
} from '../../test-helpers/universal-test-setup';

function pathWithinProject(projectRoot: string, relativePath: string): string {
  return path.join(projectRoot, relativePath);
}

async function fileExists(filePath: string): Promise<boolean> {
  try {
    await access(filePath, constants.F_OK);
    return true;
  } catch {
    return false;
  }
}

describe('Feature: Codex init writes fspec prompt to user home directory', () => {
  const codexAgent = getAgentById('codex-cli');

  if (!codexAgent) {
    throw new Error('codex-cli agent configuration not found in registry');
  }

  let setup: TestDirectorySetup;
  let originalCwd: string;

  beforeEach(async () => {
    setup = await setupTestDirectory('init-codex');
    originalCwd = process.cwd();
    process.chdir(setup.testDir);
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    await setup.cleanup();
    vi.restoreAllMocks();
  });

  it('writes the Codex prompt to ~/.codex/prompts on Unix-like systems', async () => {
    const homeSetup = await setupTestDirectory('init-codex-home');
    const homeDir = homeSetup.testDir;
    const homedirSpy = vi.spyOn(os, 'homedir').mockReturnValue(homeDir);

    await installAgentFiles(setup.testDir, codexAgent);

    const expectedPromptPath = path.join(
      homeDir,
      '.codex',
      'prompts',
      'fspec.md'
    );
    const promptContent = await readFile(expectedPromptPath, 'utf-8');
    expect(promptContent.length).toBeGreaterThan(0);

    const projectPromptPath = pathWithinProject(
      setup.testDir,
      path.join('.codex', 'prompts', 'fspec.md')
    );

    expect(await fileExists(projectPromptPath)).toBe(false);

    homedirSpy.mockRestore();
    await homeSetup.cleanup();
  });

  it('resolves the Codex prompt path using os.homedir on Windows-style paths', async () => {
    const windowsHome = 'C:\\Users\\Riley';
    const homedirSpy = vi.spyOn(os, 'homedir').mockReturnValue(windowsHome);

    await installAgentFiles(setup.testDir, codexAgent);

    const expectedRelative = path.join(
      windowsHome,
      '.codex',
      'prompts',
      'fspec.md'
    );
    const resolvedPath = pathWithinProject(setup.testDir, expectedRelative);
    const promptContent = await readFile(resolvedPath, 'utf-8');
    expect(promptContent.length).toBeGreaterThan(0);

    homedirSpy.mockRestore();
  });

  it('re-running fspec init keeps project-level prompt intact', async () => {
    const homeSetup = await setupTestDirectory('init-codex-home');
    const homeDir = homeSetup.testDir;
    const homedirSpy = vi.spyOn(os, 'homedir').mockReturnValue(homeDir);

    const projectPromptsDir = pathWithinProject(
      setup.testDir,
      path.join('.codex', 'prompts')
    );
    await mkdir(projectPromptsDir, { recursive: true });
    const projectPromptPath = path.join(projectPromptsDir, 'fspec.md');
    await writeFile(projectPromptPath, 'project-level prompt', 'utf-8');

    await installAgentFiles(setup.testDir, codexAgent);

    const homePromptPath = path.join(homeDir, '.codex', 'prompts', 'fspec.md');
    const homeContent = await readFile(homePromptPath, 'utf-8');
    expect(homeContent.length).toBeGreaterThan(0);

    const projectContent = await readFile(projectPromptPath, 'utf-8');
    expect(projectContent).toBe('project-level prompt');

    homedirSpy.mockRestore();
    await homeSetup.cleanup();
  });
});
