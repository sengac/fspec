/**
 * Feature: spec/features/codex-init-writes-prompt-to-home-directory.feature
 *
 * Tests validating Codex prompt generation behavior for fspec init.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, readFile, mkdir, writeFile, access } from 'fs/promises';
import { constants } from 'fs';
import os from 'os';
import path from 'path';
import { installAgentFiles } from '../../commands/init';
import { getAgentById } from '../../utils/agentRegistry';

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

  let projectRoot: string;
  let originalCwd: string;

  beforeEach(async () => {
    projectRoot = await mkdtemp(path.join(os.tmpdir(), 'fspec-project-'));
    originalCwd = process.cwd();
    process.chdir(projectRoot);
  });

  afterEach(async () => {
    process.chdir(originalCwd);
    await rm(projectRoot, { recursive: true, force: true });
    vi.restoreAllMocks();
  });

  it('writes the Codex prompt to ~/.codex/prompts on Unix-like systems', async () => {
    const homeDir = await mkdtemp(path.join(os.tmpdir(), 'fspec-home-'));
    const homedirSpy = vi.spyOn(os, 'homedir').mockReturnValue(homeDir);

    await installAgentFiles(projectRoot, codexAgent);

    const expectedPromptPath = path.join(
      homeDir,
      '.codex',
      'prompts',
      'fspec.md'
    );
    const promptContent = await readFile(expectedPromptPath, 'utf-8');
    expect(promptContent.length).toBeGreaterThan(0);

    const projectPromptPath = pathWithinProject(
      projectRoot,
      path.join('.codex', 'prompts', 'fspec.md')
    );

    expect(await fileExists(projectPromptPath)).toBe(false);

    homedirSpy.mockRestore();
    await rm(homeDir, { recursive: true, force: true });
  });

  it('resolves the Codex prompt path using os.homedir on Windows-style paths', async () => {
    const windowsHome = 'C:\\Users\\Riley';
    const homedirSpy = vi.spyOn(os, 'homedir').mockReturnValue(windowsHome);

    await installAgentFiles(projectRoot, codexAgent);

    const expectedRelative = path.join(
      windowsHome,
      '.codex',
      'prompts',
      'fspec.md'
    );
    const resolvedPath = pathWithinProject(projectRoot, expectedRelative);
    const promptContent = await readFile(resolvedPath, 'utf-8');
    expect(promptContent.length).toBeGreaterThan(0);

    homedirSpy.mockRestore();
  });

  it('re-running fspec init keeps project-level prompt intact', async () => {
    const homeDir = await mkdtemp(path.join(os.tmpdir(), 'fspec-home-'));
    const homedirSpy = vi.spyOn(os, 'homedir').mockReturnValue(homeDir);

    const projectPromptsDir = pathWithinProject(
      projectRoot,
      path.join('.codex', 'prompts')
    );
    await mkdir(projectPromptsDir, { recursive: true });
    const projectPromptPath = path.join(projectPromptsDir, 'fspec.md');
    await writeFile(projectPromptPath, 'project-level prompt', 'utf-8');

    await installAgentFiles(projectRoot, codexAgent);

    const homePromptPath = path.join(homeDir, '.codex', 'prompts', 'fspec.md');
    const homeContent = await readFile(homePromptPath, 'utf-8');
    expect(homeContent.length).toBeGreaterThan(0);

    const projectContent = await readFile(projectPromptPath, 'utf-8');
    expect(projectContent).toBe('project-level prompt');

    homedirSpy.mockRestore();
    await rm(homeDir, { recursive: true, force: true });
  });
});
