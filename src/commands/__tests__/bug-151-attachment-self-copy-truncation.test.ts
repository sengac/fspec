/**
 * Feature: spec/features/work-unit-attachments.feature
 *
 * Tests for BUG-151: add-attachment truncates the source file to 0 bytes
 * when it already lives in spec/attachments/<ID>/.
 *
 * Node's copyFile opens the destination with O_TRUNC, so copying a file
 * onto itself destroys it. add-attachment must detect source === destination
 * (canonicalized via realpath) and register-only, and must run the
 * duplicate-registration check BEFORE any filesystem mutation.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
  mkdir,
  writeFile,
  readFile,
  access,
  symlink,
  chmod,
} from 'fs/promises';
import { join } from 'path';
import { addAttachment } from '../add-attachment';
import type { WorkUnitsData } from '../../types';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

const FILE_CONTENT = 'important research';

async function readWorkUnits(testDir: string): Promise<WorkUnitsData> {
  const content = await readFile(
    join(testDir, 'spec', 'work-units.json'),
    'utf-8'
  );
  return JSON.parse(content) as WorkUnitsData;
}

describe('Feature: add-attachment truncates the source file to 0 bytes when it already lives in spec/attachments/<ID>/', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('bug-151-attachment-self-copy');

    const workUnitsData: WorkUnitsData = {
      meta: {
        version: '1.0.0',
        lastUpdated: new Date().toISOString(),
      },
      workUnits: {
        'TEST-001': {
          id: 'TEST-001',
          type: 'bug',
          prefix: 'TEST',
          title: 'Test Work Unit 1',
          status: 'backlog',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      },
    };

    await writeJsonTestFile(setup.workUnitsFile, workUnitsData);
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Register a file that already lives in the work unit attachments directory', () => {
    it('should register the file without truncating it', async () => {
      // @step Given I have a work unit "TEST-001"
      // (created in beforeEach)

      // @step And a file "spec/attachments/TEST-001/notes.md" with content "important research"
      const attachmentsDir = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-001'
      );
      await mkdir(attachmentsDir, { recursive: true });
      const notesFile = join(attachmentsDir, 'notes.md');
      await writeFile(notesFile, FILE_CONTENT);

      // @step When I add the attachment "spec/attachments/TEST-001/notes.md" to work unit "TEST-001"
      // @step Then the command should succeed
      await expect(
        addAttachment({
          workUnitId: 'TEST-001',
          filePath: notesFile,
          cwd: setup.testDir,
        })
      ).resolves.not.toThrow();

      // @step And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
      const contentAfter = await readFile(notesFile, 'utf-8');
      expect(contentAfter).toBe(FILE_CONTENT);

      // @step And the work unit should track "spec/attachments/TEST-001/notes.md" as an attachment
      const workUnits = await readWorkUnits(setup.testDir);
      expect(workUnits.workUnits['TEST-001'].attachments).toContain(
        'spec/attachments/TEST-001/notes.md'
      );
    });
  });

  describe('Scenario: Duplicate registration is rejected without touching the file', () => {
    it('should throw already exists and leave the file content intact', async () => {
      // @step Given I have a work unit "TEST-001" with attachment "spec/attachments/TEST-001/notes.md" containing "important research"
      const attachmentsDir = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-001'
      );
      await mkdir(attachmentsDir, { recursive: true });
      const notesFile = join(attachmentsDir, 'notes.md');
      await writeFile(notesFile, FILE_CONTENT);

      const seeded = await readWorkUnits(setup.testDir);
      seeded.workUnits['TEST-001'].attachments = [
        'spec/attachments/TEST-001/notes.md',
      ];
      await writeJsonTestFile(setup.workUnitsFile, seeded);

      // @step When I add the attachment "spec/attachments/TEST-001/notes.md" to work unit "TEST-001" again
      // @step Then the command should fail with an "already exists" error
      await expect(
        addAttachment({
          workUnitId: 'TEST-001',
          filePath: notesFile,
          cwd: setup.testDir,
        })
      ).rejects.toThrow('already exists');

      // @step And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
      const contentAfter = await readFile(notesFile, 'utf-8');
      expect(contentAfter).toBe(FILE_CONTENT);
    });
  });

  describe('Scenario: Register a read-only file already in the attachments directory without attempting a copy', () => {
    it('should register the read-only file without attempting a copy', async () => {
      // @step Given I have a work unit "TEST-001"
      // (created in beforeEach)

      // @step And a read-only file "spec/attachments/TEST-001/notes.md" with content "important research"
      const attachmentsDir = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-001'
      );
      await mkdir(attachmentsDir, { recursive: true });
      const notesFile = join(attachmentsDir, 'notes.md');
      await writeFile(notesFile, FILE_CONTENT);
      await chmod(notesFile, 0o444);

      // @step When I add the attachment "spec/attachments/TEST-001/notes.md" to work unit "TEST-001"
      // @step Then the command should succeed
      await expect(
        addAttachment({
          workUnitId: 'TEST-001',
          filePath: notesFile,
          cwd: setup.testDir,
        })
      ).resolves.not.toThrow();

      // @step And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
      const contentAfter = await readFile(notesFile, 'utf-8');
      expect(contentAfter).toBe(FILE_CONTENT);

      // @step And the work unit should track "spec/attachments/TEST-001/notes.md" as an attachment
      const workUnits = await readWorkUnits(setup.testDir);
      expect(workUnits.workUnits['TEST-001'].attachments).toContain(
        'spec/attachments/TEST-001/notes.md'
      );

      // Restore write permission so temp dir cleanup succeeds everywhere
      await chmod(notesFile, 0o644);
    });
  });

  describe('Scenario: Duplicate registration from a different source file does not overwrite the registered attachment', () => {
    it('should throw already exists without overwriting the registered attachment', async () => {
      // @step Given I have a work unit "TEST-001" with attachment "spec/attachments/TEST-001/notes.md" containing "important research"
      const attachmentsDir = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-001'
      );
      await mkdir(attachmentsDir, { recursive: true });
      const notesFile = join(attachmentsDir, 'notes.md');
      await writeFile(notesFile, FILE_CONTENT);

      const seeded = await readWorkUnits(setup.testDir);
      seeded.workUnits['TEST-001'].attachments = [
        'spec/attachments/TEST-001/notes.md',
      ];
      await writeJsonTestFile(setup.workUnitsFile, seeded);

      // @step And a file "other/notes.md" with content "different content"
      const otherDir = join(setup.testDir, 'other');
      await mkdir(otherDir, { recursive: true });
      const otherFile = join(otherDir, 'notes.md');
      await writeFile(otherFile, 'different content');

      // @step When I add the attachment "other/notes.md" to work unit "TEST-001"
      // @step Then the command should fail with an "already exists" error
      await expect(
        addAttachment({
          workUnitId: 'TEST-001',
          filePath: otherFile,
          cwd: setup.testDir,
        })
      ).rejects.toThrow('already exists');

      // @step And the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
      const contentAfter = await readFile(notesFile, 'utf-8');
      expect(contentAfter).toBe(FILE_CONTENT);
    });
  });

  describe('Scenario: File in the attachments root is still moved into the work unit directory', () => {
    it('should copy the root file into the work unit dir and remove the root copy (BUG-055)', async () => {
      // @step Given I have a work unit "TEST-001"
      // (created in beforeEach)

      // @step And a file "spec/attachments/analysis.md" with content "root analysis"
      await mkdir(join(setup.testDir, 'spec', 'attachments'), {
        recursive: true,
      });
      const rootFile = join(
        setup.testDir,
        'spec',
        'attachments',
        'analysis.md'
      );
      await writeFile(rootFile, 'root analysis');

      // @step When I add the attachment "spec/attachments/analysis.md" to work unit "TEST-001"
      await addAttachment({
        workUnitId: 'TEST-001',
        filePath: rootFile,
        cwd: setup.testDir,
      });

      // @step Then the file should exist at "spec/attachments/TEST-001/analysis.md" with content "root analysis"
      const movedFile = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-001',
        'analysis.md'
      );
      const movedContent = await readFile(movedFile, 'utf-8');
      expect(movedContent).toBe('root analysis');

      // @step And the file "spec/attachments/analysis.md" should no longer exist
      await expect(access(rootFile)).rejects.toThrow();

      // @step And the work unit should track "spec/attachments/TEST-001/analysis.md" as an attachment
      const workUnits = await readWorkUnits(setup.testDir);
      expect(workUnits.workUnits['TEST-001'].attachments).toContain(
        'spec/attachments/TEST-001/analysis.md'
      );
    });
  });

  describe('Scenario: Symlink alias of the destination file does not truncate it', () => {
    it('should detect the symlink alias via realpath and register without truncating', async () => {
      // @step Given I have a work unit "TEST-001"
      // (created in beforeEach)

      // @step And a file "spec/attachments/TEST-001/notes.md" with content "important research"
      const attachmentsDir = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-001'
      );
      await mkdir(attachmentsDir, { recursive: true });
      const notesFile = join(attachmentsDir, 'notes.md');
      await writeFile(notesFile, FILE_CONTENT);

      // @step And a symlink outside the attachments directory pointing at "spec/attachments/TEST-001/notes.md"
      const aliasDir = join(setup.testDir, 'alias');
      await mkdir(aliasDir, { recursive: true });
      const symlinkPath = join(aliasDir, 'notes.md');
      await symlink(notesFile, symlinkPath);

      // @step When I add the attachment via the symlink path to work unit "TEST-001"
      await addAttachment({
        workUnitId: 'TEST-001',
        filePath: symlinkPath,
        cwd: setup.testDir,
      });

      // @step Then the file "spec/attachments/TEST-001/notes.md" should still contain "important research"
      const contentAfter = await readFile(notesFile, 'utf-8');
      expect(contentAfter).toBe(FILE_CONTENT);

      // @step And the work unit should track "spec/attachments/TEST-001/notes.md" as an attachment
      const workUnits = await readWorkUnits(setup.testDir);
      expect(workUnits.workUnits['TEST-001'].attachments).toContain(
        'spec/attachments/TEST-001/notes.md'
      );
    });
  });
});
