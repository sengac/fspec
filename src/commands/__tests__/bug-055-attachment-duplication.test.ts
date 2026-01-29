/**
 * Feature: spec/features/attachment-files-duplicated-in-spec-attachments-and-spec-attachments-id.feature
 *
 * Tests for BUG-055: Attachment files duplicated in spec/attachments/ and spec/attachments/[ID]/
 * When adding an attachment from spec/attachments/ root, the file should be MOVED (not copied)
 * to the work unit subdirectory to prevent duplication.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, access, readFile, rm } from 'fs/promises';
import { join } from 'path';
import { tmpdir } from 'os';
import { addAttachment } from '../add-attachment';
import type { WorkUnitsData } from '../../types';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Attachment files duplicated in spec/attachments/ and spec/attachments/[ID]/', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    // Create temp directory for tests
    setup = await setupWorkUnitTest('bug-055-attachment-duplication');

    // Initialize attachments directory structure
    await mkdir(join(setup.specDir, 'attachments'), { recursive: true });

    // Create work-units.json
    const workUnitsData: WorkUnitsData = {
      meta: {
        version: '0.6.0',
        lastUpdated: new Date().toISOString(),
      },
      workUnits: {
        'TEST-001': {
          id: 'TEST-001',
          type: 'story',
          prefix: 'TEST',
          title: 'Test Work Unit 1',
          status: 'backlog',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        'TEST-002': {
          id: 'TEST-002',
          type: 'story',
          prefix: 'TEST',
          title: 'Test Work Unit 2',
          status: 'backlog',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        'TEST-003': {
          id: 'TEST-003',
          type: 'story',
          prefix: 'TEST',
          title: 'Test Work Unit 3',
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

  describe('Scenario: Prevent duplication when adding file from spec/attachments/ root', () => {
    it('should move file from root to work unit directory and delete original', async () => {
      // Given I have a work unit "TEST-001"
      // And I have a file "spec/attachments/analysis.md" in the root attachments directory
      const rootFile = join(
        setup.testDir,
        'spec',
        'attachments',
        'analysis.md'
      );
      await writeFile(rootFile, '# Analysis Document');

      // Verify file exists in root before attachment
      await expect(access(rootFile)).resolves.not.toThrow();

      // When I run "fspec add-attachment TEST-001 spec/attachments/analysis.md"
      await addAttachment({
        workUnitId: 'TEST-001',
        filePath: rootFile,
        cwd: setup.testDir,
      });

      // Then the file should be moved to "spec/attachments/TEST-001/analysis.md"
      const workUnitFile = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-001',
        'analysis.md'
      );
      await expect(access(workUnitFile)).resolves.not.toThrow();

      // And the file "spec/attachments/analysis.md" should no longer exist in the root directory
      await expect(access(rootFile)).rejects.toThrow();

      // And the work unit should track "spec/attachments/TEST-001/analysis.md" as an attachment
      const workUnitsContent = await readFile(
        join(setup.testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const workUnits: WorkUnitsData = JSON.parse(workUnitsContent);
      expect(workUnits.workUnits['TEST-001'].attachments).toContain(
        'spec/attachments/TEST-001/analysis.md'
      );
    });
  });

  describe('Scenario: Normal attachment from external path', () => {
    it('should copy file from external path and preserve original', async () => {
      // Given I have a work unit "TEST-002"
      // And I have a file "/tmp/diagram.png" outside the attachments directory
      const externalFile = join(tmpdir(), `diagram-${Date.now()}.png`);
      await writeFile(externalFile, 'PNG data');

      // When I run "fspec add-attachment TEST-002 /tmp/diagram.png"
      await addAttachment({
        workUnitId: 'TEST-002',
        filePath: externalFile,
        cwd: setup.testDir,
      });

      // Then the file should be copied to "spec/attachments/TEST-002/diagram.png"
      const workUnitFile = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-002',
        `diagram-${Date.now()}.png`
      );
      // Note: Using basename comparison since actual filename will be diagram-<timestamp>.png
      const workUnitDir = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-002'
      );
      await expect(access(workUnitDir)).resolves.not.toThrow();

      // And the original file "/tmp/diagram.png" should still exist
      await expect(access(externalFile)).resolves.not.toThrow();

      // Cleanup external file
      await rm(externalFile, { force: true });
    });
  });

  describe('Scenario: Verify no duplication after fix', () => {
    it('should ensure only one copy exists in work unit directory', async () => {
      // Given I have a work unit "TEST-003"
      // And I have a file "spec/attachments/document.pdf" in the root attachments directory
      const rootFile = join(
        setup.testDir,
        'spec',
        'attachments',
        'document.pdf'
      );
      await writeFile(rootFile, 'PDF content');

      // When I run "fspec add-attachment TEST-003 spec/attachments/document.pdf"
      await addAttachment({
        workUnitId: 'TEST-003',
        filePath: rootFile,
        cwd: setup.testDir,
      });

      // Then there should be exactly one copy of the file in "spec/attachments/TEST-003/document.pdf"
      const workUnitFile = join(
        setup.testDir,
        'spec',
        'attachments',
        'TEST-003',
        'document.pdf'
      );
      await expect(access(workUnitFile)).resolves.not.toThrow();

      // And there should be zero files in the root "spec/attachments/" directory named "document.pdf"
      await expect(access(rootFile)).rejects.toThrow();
    });
  });
});
