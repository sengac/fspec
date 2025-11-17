// Feature: spec/features/discover-foundation-regenerates-draft-when-used-to-check-next-field.feature

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { discoverFoundation } from '../discover-foundation';
import { updateFoundation } from '../update-foundation';
import { readFile, writeFile, unlink, mkdir } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';

describe('discover-foundation workflow clarity', () => {
  let testDir: string;
  let draftPath: string;

  beforeEach(async () => {
    // Create temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await mkdir(join(testDir, 'spec'), { recursive: true });
    draftPath = join(testDir, 'spec', 'foundation.json.draft');
  });

  afterEach(async () => {
    // Cleanup
    try {
      await unlink(draftPath);
    } catch {
      // Ignore
    }
  });

  it('AI accidentally runs discover-foundation and draft is protected', async () => {
    // @step Given a foundation draft exists with 3 fields already filled
    await discoverFoundation({ cwd: testDir, draftPath, force: true });

    // @step And the draft contains filled values for projectName, projectVision, and projectType
    const draftContent = JSON.parse(await readFile(draftPath, 'utf-8'));
    draftContent.project.name = 'fspec';
    draftContent.project.vision = 'Test vision';
    draftContent.project.projectType = 'cli-tool';
    await writeFile(draftPath, JSON.stringify(draftContent, null, 2));

    // Verify fields are filled
    const filledDraft = JSON.parse(await readFile(draftPath, 'utf-8'));
    expect(filledDraft.project.name).toBe('fspec');
    expect(filledDraft.project.vision).toBe('Test vision');
    expect(filledDraft.project.projectType).toBe('cli-tool');

    // @step When AI runs "fspec discover-foundation" without flags
    const result = await discoverFoundation({ cwd: testDir, draftPath });

    // @step Then the command fails with error
    expect(result.valid).toBe(false);
    expect(result.systemReminder).toContain(
      'foundation.json.draft already exists'
    );

    // @step And the draft is NOT overwritten
    const unchangedDraft = JSON.parse(await readFile(draftPath, 'utf-8'));

    // @step And all 3 previously filled fields remain unchanged
    expect(unchangedDraft.project.name).toBe('fspec');
    expect(unchangedDraft.project.vision).toBe('Test vision');
    expect(unchangedDraft.project.projectType).toBe('cli-tool');

    // @step And no progress is lost
    expect(unchangedDraft.project.name).not.toContain('[QUESTION:');
  });

  it('update-foundation emits next field reminder', async () => {
    // @step Given a foundation draft exists
    await discoverFoundation({ cwd: testDir, draftPath, force: true });

    // @step When AI runs "fspec update-foundation projectName 'fspec'"
    const result = await updateFoundation({
      cwd: testDir,
      draftPath,
      section: 'projectName',
      content: 'fspec',
    });

    // @step Then a system-reminder is emitted
    expect(result).toHaveProperty('systemReminder');
    expect(result.systemReminder).toBeDefined();
    expect(result.systemReminder.length).toBeGreaterThan(0);

    // @step And the system-reminder contains "Field 2/8: project.vision"
    expect(result.systemReminder).toContain('Field 2/8');
    expect(result.systemReminder).toContain('project.vision');

    // @step And the system-reminder contains the correct next command "fspec update-foundation projectVision"
    expect(result.systemReminder).toContain(
      'fspec update-foundation projectVision'
    );
  });

  it('System-reminder clarifies workflow to prevent confusion', async () => {
    // @step Given a foundation draft exists
    await discoverFoundation({ cwd: testDir, draftPath });

    // @step When AI runs any "fspec update-foundation" command successfully
    const result = await updateFoundation({
      cwd: testDir,
      draftPath,
      section: 'projectName',
      content: 'test-project',
    });

    // @step Then the system-reminder must NOT suggest running "fspec discover-foundation"
    // It's okay to mention discover-foundation in workflow explanation, but NOT as next command
    expect(result.systemReminder).not.toMatch(
      /^Run: fspec discover-foundation/m
    );
    expect(result.systemReminder).not.toMatch(
      /Next.*fspec discover-foundation/i
    );

    // @step And the system-reminder must explain the workflow: "discover once → update many → finalize once"
    expect(
      result.systemReminder.toLowerCase().includes('discover') &&
        result.systemReminder.toLowerCase().includes('update') &&
        result.systemReminder.toLowerCase().includes('finalize')
    ).toBe(true);

    // @step And the system-reminder must show the next update-foundation or add-persona/add-capability command
    const hasUpdateCommand =
      result.systemReminder.includes('fspec update-foundation') ||
      result.systemReminder.includes('fspec add-persona') ||
      result.systemReminder.includes('fspec add-capability');
    expect(hasUpdateCommand).toBe(true);
  });
});
