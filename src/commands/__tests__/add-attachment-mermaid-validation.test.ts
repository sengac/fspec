/**
 * Feature: spec/features/validate-mermaid-diagrams-in-attachments.feature
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fs } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { addAttachment } from '../add-attachment';

describe('Feature: Validate Mermaid diagrams in attachments', () => {
  let tempDir: string;
  let workUnitsFile: string;

  beforeEach(async () => {
    // Create temporary directory for testing
    tempDir = await fs.mkdtemp(join(tmpdir(), 'fspec-test-'));
    workUnitsFile = join(tempDir, 'spec', 'work-units.json');

    // Setup spec directory structure
    await fs.mkdir(join(tempDir, 'spec'), { recursive: true });

    // Create work-units.json with AUTH-001 in backlog
    await fs.writeFile(
      workUnitsFile,
      JSON.stringify(
        {
          workUnits: {
            'AUTH-001': {
              id: 'AUTH-001',
              prefix: 'AUTH',
              title: 'Test Work Unit',
              status: 'backlog',
              type: 'story',
              createdAt: new Date().toISOString(),
              updatedAt: new Date().toISOString(),
              stateHistory: [],
            },
          },
          states: {
            backlog: ['AUTH-001'],
            specifying: [],
            testing: [],
            implementing: [],
            validating: [],
            done: [],
            blocked: [],
          },
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    // Clean up temp directory
    await fs.rm(tempDir, { recursive: true, force: true });
  });

  describe('Scenario: Attach valid Mermaid diagram with .mmd extension', () => {
    it('should add the attachment successfully', async () => {
      // Given I have a work unit AUTH-001 in the backlog
      // (already setup in beforeEach)

      // And I have a file flowchart.mmd with valid Mermaid syntax (graph TD)
      const mmdFile = join(tempDir, 'flowchart.mmd');
      await fs.writeFile(mmdFile, 'graph TD\n  A[Start] --> B[End]\n');

      // When I run 'fspec add-attachment AUTH-001 flowchart.mmd'
      await addAttachment({
        workUnitId: 'AUTH-001',
        filePath: mmdFile,
        cwd: tempDir,
      });

      // Then the attachment should be added successfully
      // And the file should be copied to spec/attachments/AUTH-001/
      const attachmentPath = join(
        tempDir,
        'spec',
        'attachments',
        'AUTH-001',
        'flowchart.mmd'
      );
      const attachmentExists = await fs
        .access(attachmentPath)
        .then(() => true)
        .catch(() => false);

      expect(attachmentExists).toBe(true);

      // And the output should display a success message
      // (We'll need to capture console output or return a result object)
    });
  });

  describe('Scenario: Attach invalid Mermaid diagram with syntax errors', () => {
    it('should fail to attach and display error message with line number', async () => {
      // Given I have a work unit AUTH-001 in the backlog
      // (already setup in beforeEach)

      // And I have a file sequence.mermaid with invalid Mermaid syntax
      const mermaidFile = join(tempDir, 'sequence.mermaid');
      await fs.writeFile(
        mermaidFile,
        'sequenceDiagram\n  Alice->Bob: Hello\n  INVALID SYNTAX HERE\n'
      );

      // When I run 'fspec add-attachment AUTH-001 sequence.mermaid'
      // Then the attachment should fail
      await expect(
        addAttachment({
          workUnitId: 'AUTH-001',
          filePath: mermaidFile,
          cwd: tempDir,
        })
      ).rejects.toThrow();

      // And the file should NOT be copied to the attachments directory
      const attachmentPath = join(
        tempDir,
        'spec',
        'attachments',
        'AUTH-001',
        'sequence.mermaid'
      );
      const attachmentExists = await fs
        .access(attachmentPath)
        .then(() => true)
        .catch(() => false);

      expect(attachmentExists).toBe(false);

      // And the output should display an error message with 'Failed to attach sequence.mermaid'
      // And the error message should include the line number of the syntax error
      // (We'll need to check error message content)
    });
  });

  describe('Scenario: Attach non-Mermaid file without validation', () => {
    it('should add the attachment successfully without Mermaid validation', async () => {
      // Given I have a work unit AUTH-001 in the backlog
      // (already setup in beforeEach)

      // And I have a file diagram.png (image file)
      const pngFile = join(tempDir, 'diagram.png');
      // Create a minimal valid PNG file (1x1 transparent pixel)
      await fs.writeFile(
        pngFile,
        Buffer.from([
          0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00,
          0x0d, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
          0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4, 0x89,
          0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63,
          0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4,
          0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60,
          0x82,
        ])
      );

      // When I run 'fspec add-attachment AUTH-001 diagram.png'
      await addAttachment({
        workUnitId: 'AUTH-001',
        filePath: pngFile,
        cwd: tempDir,
      });

      // Then the attachment should be added successfully without Mermaid validation
      // And the file should be copied to spec/attachments/AUTH-001/
      const attachmentPath = join(
        tempDir,
        'spec',
        'attachments',
        'AUTH-001',
        'diagram.png'
      );
      const attachmentExists = await fs
        .access(attachmentPath)
        .then(() => true)
        .catch(() => false);

      expect(attachmentExists).toBe(true);
    });
  });

  describe('Scenario: Attach markdown file with valid Mermaid code block', () => {
    it('should extract and validate the mermaid code block', async () => {
      // Given I have a work unit AUTH-001 in the backlog
      // (already setup in beforeEach)

      // And I have a file architecture.md containing a mermaid code block with valid syntax
      const mdFile = join(tempDir, 'architecture.md');
      await fs.writeFile(
        mdFile,
        '# Architecture\n\nHere is our system flow:\n\n```mermaid\ngraph LR\n  A[Client] --> B[Server]\n  B --> C[Database]\n```\n'
      );

      // When I run 'fspec add-attachment AUTH-001 architecture.md'
      await addAttachment({
        workUnitId: 'AUTH-001',
        filePath: mdFile,
        cwd: tempDir,
      });

      // Then the mermaid code block should be extracted and validated
      // And the attachment should be added successfully
      // And the file should be copied to spec/attachments/AUTH-001/
      const attachmentPath = join(
        tempDir,
        'spec',
        'attachments',
        'AUTH-001',
        'architecture.md'
      );
      const attachmentExists = await fs
        .access(attachmentPath)
        .then(() => true)
        .catch(() => false);

      expect(attachmentExists).toBe(true);
    });
  });
});
