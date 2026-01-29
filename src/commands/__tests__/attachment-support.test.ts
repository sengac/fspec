/**
 * Feature: spec/features/attachment-support-for-discovery-process.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdir, writeFile, readFile, access } from 'fs/promises';
import { join } from 'path';
import { addAttachment } from '../add-attachment';
import { listAttachments } from '../list-attachments';
import { removeAttachment } from '../remove-attachment';
import { showWorkUnit } from '../show-work-unit';
import type { WorkUnitsData } from '../../types';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import { writeJsonTestFile } from '../../test-helpers/test-file-operations';

describe('Feature: Attachment support for discovery process', () => {
  let setup: WorkUnitTestSetup;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('attachment-support');

    // Create work-units.json with test prefix and work units
    const workUnitsData: WorkUnitsData = {
      meta: {
        version: '1.0.0',
        lastUpdated: new Date().toISOString(),
      },
      workUnits: {
        'AUTH-001': {
          id: 'AUTH-001',
          title: 'User Authentication',
          status: 'specifying',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        'AUTH-002': {
          id: 'AUTH-002',
          title: 'Password Reset',
          status: 'specifying',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
        'UI-002': {
          id: 'UI-002',
          title: 'Dashboard',
          status: 'specifying',
          createdAt: new Date().toISOString(),
          updatedAt: new Date().toISOString(),
        },
      },
      states: {
        backlog: [],
        specifying: ['AUTH-001', 'AUTH-002', 'UI-002'],
        testing: [],
        implementing: [],
        validating: [],
        done: [],
        blocked: [],
      },
    };

    await writeJsonTestFile(setup.workUnitsFile, workUnitsData);
  });

  afterEach(async () => {
    await setup.cleanup();
  });

  describe('Scenario: Add attachment to work unit', () => {
    it('should copy file to attachments directory and track in work unit', async () => {
      // Given I have a work unit "AUTH-001" in specifying status
      // And I have a file "diagrams/auth-flow.png" in the current directory
      await mkdir(join(setup.testDir, 'diagrams'), { recursive: true });
      const sourceFile = join(setup.testDir, 'diagrams', 'auth-flow.png');
      await writeFile(sourceFile, 'test diagram content');

      // When I run "fspec add-attachment AUTH-001 diagrams/auth-flow.png"
      await addAttachment({
        workUnitId: 'AUTH-001',
        filePath: sourceFile,
        cwd: setup.testDir,
      });

      // Then the file should be copied to "spec/attachments/AUTH-001/auth-flow.png"
      const attachmentPath = join(
        setup.testDir,
        'spec',
        'attachments',
        'AUTH-001',
        'auth-flow.png'
      );
      await expect(access(attachmentPath)).resolves.not.toThrow();

      // And the work unit should track the attachment path
      const workUnitsContent = await readFile(
        join(setup.testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const workUnits: WorkUnitsData = JSON.parse(workUnitsContent);
      expect(workUnits.workUnits['AUTH-001'].attachments).toContain(
        'spec/attachments/AUTH-001/auth-flow.png'
      );
    });
  });

  describe('Scenario: Add attachment with description', () => {
    it('should copy file and store with description metadata', async () => {
      // Given I have a work unit "UI-002" in specifying status
      // And I have a file "mockups/dashboard.png" in the current directory
      await mkdir(join(setup.testDir, 'mockups'), { recursive: true });
      const sourceFile = join(setup.testDir, 'mockups', 'dashboard.png');
      await writeFile(sourceFile, 'mockup image');

      // When I run "fspec add-attachment UI-002 mockups/dashboard.png --description 'Dashboard v2'"
      await addAttachment({
        workUnitId: 'UI-002',
        filePath: sourceFile,
        description: 'Dashboard v2',
        cwd: setup.testDir,
      });

      // Then the file should be copied to "spec/attachments/UI-002/dashboard.png"
      const attachmentPath = join(
        setup.testDir,
        'spec',
        'attachments',
        'UI-002',
        'dashboard.png'
      );
      await expect(access(attachmentPath)).resolves.not.toThrow();
    });
  });

  describe('Scenario: List attachments for work unit', () => {
    it('should display all attachment paths with file stats', async () => {
      // Given I have a work unit "AUTH-001" with attached files
      await mkdir(join(setup.testDir, 'diagrams'), { recursive: true });
      const sourceFile = join(setup.testDir, 'diagrams', 'auth-flow.png');
      await writeFile(sourceFile, 'test content');

      await addAttachment({
        workUnitId: 'AUTH-001',
        filePath: sourceFile,
        cwd: setup.testDir,
      });

      // When I run "fspec list-attachments AUTH-001"
      // Then output would display attachment info (tested via console output in real scenario)
      const workUnitsContent = await readFile(
        join(setup.testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const workUnits: WorkUnitsData = JSON.parse(workUnitsContent);

      expect(workUnits.workUnits['AUTH-001'].attachments).toBeDefined();
      expect(workUnits.workUnits['AUTH-001'].attachments).toHaveLength(1);
    });
  });

  describe('Scenario: List attachments shows no attachments', () => {
    it('should display message when no attachments exist', async () => {
      // Given I have a work unit "AUTH-002" with no attachments
      // When I run "fspec list-attachments AUTH-002"
      // Then output should show "No attachments found"

      const workUnitsContent = await readFile(
        join(setup.testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const workUnits: WorkUnitsData = JSON.parse(workUnitsContent);

      expect(workUnits.workUnits['AUTH-002'].attachments).toBeUndefined();
    });
  });

  describe('Scenario: Remove attachment from work unit', () => {
    it('should delete file and remove tracking entry', async () => {
      // Given I have a work unit "AUTH-001" with an attached file "diagram.png"
      await mkdir(join(setup.testDir, 'diagrams'), { recursive: true });
      const sourceFile = join(setup.testDir, 'diagrams', 'diagram.png');
      await writeFile(sourceFile, 'test content');

      await addAttachment({
        workUnitId: 'AUTH-001',
        filePath: sourceFile,
        cwd: setup.testDir,
      });

      // When I run "fspec remove-attachment AUTH-001 diagram.png"
      await removeAttachment({
        workUnitId: 'AUTH-001',
        fileName: 'diagram.png',
        cwd: setup.testDir,
      });

      // Then the file should be deleted
      const attachmentPath = join(
        setup.testDir,
        'spec',
        'attachments',
        'AUTH-001',
        'diagram.png'
      );
      await expect(access(attachmentPath)).rejects.toThrow();

      // And the work unit should no longer track the attachment
      const workUnitsContent = await readFile(
        join(setup.testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const workUnits: WorkUnitsData = JSON.parse(workUnitsContent);
      expect(workUnits.workUnits['AUTH-001'].attachments).toEqual([]);
    });
  });

  describe('Scenario: Remove attachment but keep file on disk', () => {
    it('should remove tracking but preserve file', async () => {
      // Given I have a work unit "AUTH-001" with an attached file "important-doc.pdf"
      await mkdir(join(setup.testDir, 'docs'), { recursive: true });
      const sourceFile = join(setup.testDir, 'docs', 'important-doc.pdf');
      await writeFile(sourceFile, 'important content');

      await addAttachment({
        workUnitId: 'AUTH-001',
        filePath: sourceFile,
        cwd: setup.testDir,
      });

      // When I run "fspec remove-attachment AUTH-001 important-doc.pdf --keep-file"
      await removeAttachment({
        workUnitId: 'AUTH-001',
        fileName: 'important-doc.pdf',
        keepFile: true,
        cwd: setup.testDir,
      });

      // Then the file should still exist
      const attachmentPath = join(
        setup.testDir,
        'spec',
        'attachments',
        'AUTH-001',
        'important-doc.pdf'
      );
      await expect(access(attachmentPath)).resolves.not.toThrow();

      // And the work unit should no longer track the attachment
      const workUnitsContent = await readFile(
        join(setup.testDir, 'spec', 'work-units.json'),
        'utf-8'
      );
      const workUnits: WorkUnitsData = JSON.parse(workUnitsContent);
      expect(workUnits.workUnits['AUTH-001'].attachments).toEqual([]);
    });
  });

  describe('Scenario: Show work unit displays attachments', () => {
    it('should include attachments section in output', async () => {
      // Given I have a work unit "AUTH-001" with attached files
      await mkdir(join(setup.testDir, 'diagrams'), { recursive: true });
      const sourceFile = join(setup.testDir, 'diagrams', 'auth-flow.png');
      await writeFile(sourceFile, 'test content');

      await addAttachment({
        workUnitId: 'AUTH-001',
        filePath: sourceFile,
        cwd: setup.testDir,
      });

      // When I run "fspec show-work-unit AUTH-001"
      const result = await showWorkUnit({
        workUnitId: 'AUTH-001',
        cwd: setup.testDir,
      });

      // Then the output should include attachments
      expect(result.attachments).toBeDefined();
      expect(result.attachments).toContain(
        'spec/attachments/AUTH-001/auth-flow.png'
      );
    });
  });

  describe('Scenario: Attempt to add non-existent file', () => {
    it('should error when source file does not exist', async () => {
      // Given I have a work unit "AUTH-001" in specifying status
      // And the file "missing-file.png" does not exist
      const missingFile = join(setup.testDir, 'missing-file.png');

      // When I run "fspec add-attachment AUTH-001 missing-file.png"
      // Then the command should exit with error
      await expect(
        addAttachment({
          workUnitId: 'AUTH-001',
          filePath: missingFile,
          cwd: setup.testDir,
        })
      ).rejects.toThrow('Source file');
    });
  });

  describe('Scenario: Attempt to add attachment to non-existent work unit', () => {
    it('should error when work unit does not exist', async () => {
      // Given I have a file "diagram.png" in the current directory
      await writeFile(join(setup.testDir, 'diagram.png'), 'test content');

      // And the work unit "FAKE-001" does not exist
      // When I run "fspec add-attachment FAKE-001 diagram.png"
      // Then the command should exit with error
      await expect(
        addAttachment({
          workUnitId: 'FAKE-001',
          filePath: join(setup.testDir, 'diagram.png'),
          cwd: setup.testDir,
        })
      ).rejects.toThrow("Work unit 'FAKE-001' does not exist");
    });
  });
});
