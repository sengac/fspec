/**
 * Feature: spec/features/handle-binary-files-and-large-files-in-diff-display.feature
 *
 * Tests for binary file detection and large file truncation in diff display.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as fs from 'fs/promises';
import * as path from 'path';
import * as os from 'os';
import { getFileDiff } from '../diff';
import * as git from 'isomorphic-git';
import fsNode from 'fs';

describe('Feature: Handle binary files and large files in diff display', () => {
  let tmpDir: string;
  let testFilePath: string;

  beforeEach(async () => {
    // Create temporary git repository
    tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), 'fspec-diff-test-'));
    await git.init({ fs: fsNode, dir: tmpDir, defaultBranch: 'main' });

    // Configure git
    await git.setConfig({
      fs: fsNode,
      dir: tmpDir,
      path: 'user.name',
      value: 'Test User',
    });
    await git.setConfig({
      fs: fsNode,
      dir: tmpDir,
      path: 'user.email',
      value: 'test@example.com',
    });
  });

  afterEach(async () => {
    // Clean up temporary directory
    await fs.rm(tmpDir, { recursive: true, force: true });
  });

  describe('Scenario: Display binary file message for PNG image', () => {
    it('should show binary file message instead of garbled content', async () => {
      // @step Given I have a checkpoint that includes changes to a PNG image file
      testFilePath = path.join(tmpDir, 'image.png');

      // Create a binary PNG file (PNG header + some binary data)
      const pngHeader = Buffer.from([
        0x89,
        0x50,
        0x4e,
        0x47,
        0x0d,
        0x0a,
        0x1a,
        0x0a, // PNG signature
        0x00,
        0x00,
        0x00,
        0x0d,
        0x49,
        0x48,
        0x44,
        0x52, // IHDR chunk
      ]);
      await fs.writeFile(testFilePath, pngHeader);

      // @step When I view the diff for the PNG file in the checkpoint viewer
      const diff = await getFileDiff(tmpDir, 'image.png');

      // @step Then I should see the message '[Binary file - no diff available]'
      expect(diff).toContain('[Binary file - no diff available]');

      // @step And I should not see garbled binary content in the diff view
      // eslint-disable-next-line no-control-regex
      expect(diff).not.toMatch(/[\x00-\x08\x0B\x0C\x0E-\x1F]/);
    });
  });

  describe('Scenario: Truncate large file with over 20,000 lines', () => {
    it('should truncate diff after 20,000 lines with message', async () => {
      // @step Given I have a checkpoint that includes changes to a log file with 50,000 lines
      testFilePath = path.join(tmpDir, 'large.log');

      // Create initial file with 10 lines and commit
      const initialContent = Array.from(
        { length: 10 },
        (_, i) => `line ${i + 1}`
      ).join('\n');
      await fs.writeFile(testFilePath, initialContent);
      await git.add({ fs: fsNode, dir: tmpDir, filepath: 'large.log' });
      await git.commit({
        fs: fsNode,
        dir: tmpDir,
        message: 'Initial commit',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // Create large file with 50,000 lines
      const largeContent = Array.from(
        { length: 50000 },
        (_, i) => `log entry ${i + 1}`
      ).join('\n');
      await fs.writeFile(testFilePath, largeContent);

      // @step When I view the diff for the log file in the checkpoint viewer
      const diff = await getFileDiff(tmpDir, 'large.log');

      // @step Then I should see the first 20,000 lines of the diff
      expect(diff).toBeDefined();
      const diffLines = diff!.split('\n');

      // Count actual content lines (excluding headers and truncation message)
      // Headers start with --- or +++ (3 chars), content lines start with single +/-/space
      const contentLines = diffLines.filter(line => {
        const firstChar = line[0];
        return (
          (firstChar === '+' || firstChar === '-' || firstChar === ' ') &&
          !line.startsWith('---') &&
          !line.startsWith('+++') &&
          !line.startsWith('[File truncated')
        );
      });
      expect(contentLines.length).toBeLessThanOrEqual(20000);

      // @step And I should see the message '[File truncated - showing first 20,000 of 50,000 lines]'
      expect(diff).toContain(
        '[File truncated - showing first 20,000 of 50,000 lines]'
      );
    });
  });

  describe('Scenario: Display complete diff for normal-sized text file', () => {
    it('should show complete diff without truncation for files under 20,000 lines', async () => {
      // @step Given I have a checkpoint that includes changes to a source file with 500 lines
      testFilePath = path.join(tmpDir, 'source.ts');

      // Create initial file and commit
      const initialContent = Array.from(
        { length: 250 },
        (_, i) => `const x${i} = ${i};`
      ).join('\n');
      await fs.writeFile(testFilePath, initialContent);
      await git.add({ fs: fsNode, dir: tmpDir, filepath: 'source.ts' });
      await git.commit({
        fs: fsNode,
        dir: tmpDir,
        message: 'Initial commit',
        author: { name: 'Test User', email: 'test@example.com' },
      });

      // Create modified file with 500 lines
      const modifiedContent = Array.from(
        { length: 500 },
        (_, i) => `const y${i} = ${i * 2};`
      ).join('\n');
      await fs.writeFile(testFilePath, modifiedContent);

      // @step When I view the diff for the source file in the checkpoint viewer
      const diff = await getFileDiff(tmpDir, 'source.ts');

      // @step Then I should see the complete diff without truncation
      expect(diff).toBeDefined();
      expect(diff).not.toBeNull();

      // @step And I should not see any truncation message
      expect(diff).not.toContain('[File truncated');
    });
  });

  describe('Scenario: Display binary file message for executable binary', () => {
    it('should show binary file message for executable files', async () => {
      // @step Given I have a checkpoint that includes changes to an executable binary file
      testFilePath = path.join(tmpDir, 'app.exe');

      // Create a binary executable file (ELF header for Linux executable)
      const elfHeader = Buffer.from([
        0x7f,
        0x45,
        0x4c,
        0x46, // ELF magic number
        0x02,
        0x01,
        0x01,
        0x00, // 64-bit, little endian
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
        0x00,
      ]);
      await fs.writeFile(testFilePath, elfHeader);

      // @step When I view the diff for the executable in the checkpoint viewer
      const diff = await getFileDiff(tmpDir, 'app.exe');

      // @step Then I should see the message '[Binary file - no diff available]'
      expect(diff).toContain('[Binary file - no diff available]');

      // @step And I should not see garbled binary content in the diff view
      // eslint-disable-next-line no-control-regex
      expect(diff).not.toMatch(/[\x00-\x08\x0B\x0C\x0E-\x1F]/);
    });
  });
});
