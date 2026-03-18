/**
 * Feature: spec/features/native-rust-diff-operations.feature
 *
 * This test file validates that diff operations run natively in Rust via NAPI,
 * replacing the legacy worker_threads-based diff-worker.ts system.
 * Scenarios map directly to Gherkin scenarios in the feature file.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { existsSync, readFileSync } from 'fs';
import { execSync } from 'child_process';
import { join } from 'path';
import { tmpdir } from 'os';

import {
  getFileDiff,
  getCheckpointFileDiff,
  createGhostCheckpoint,
} from '@sengac/codelet-napi';

describe('Feature: Replace diff-worker.ts with native Rust NAPI diff operations', () => {
  let tempDir: string;

  beforeEach(async () => {
    tempDir = await mkdtemp(join(tmpdir(), 'fspec-native-diff-'));
    execSync('git init', { cwd: tempDir, stdio: 'pipe' });
    execSync('git config user.email "test@example.com"', {
      cwd: tempDir,
      stdio: 'pipe',
    });
    execSync('git config user.name "Test User"', {
      cwd: tempDir,
      stdio: 'pipe',
    });
  });

  afterEach(async () => {
    await rm(tempDir, { recursive: true, force: true });
  });

  describe('Scenario: Working directory diff via NAPI returns unified diff', () => {
    it('should return unified diff with +/- prefixed lines from NAPI', async () => {
      // @step Given a git repository with a tracked file that has uncommitted changes
      await writeFile(join(tempDir, 'example.ts'), 'line 1\nline 2\nline 3\n');
      execSync('git add example.ts', { cwd: tempDir, stdio: 'pipe' });
      execSync('git commit -m "initial"', { cwd: tempDir, stdio: 'pipe' });
      await writeFile(
        join(tempDir, 'example.ts'),
        'line 1\nmodified line 2\nline 3\nnew line 4\n'
      );

      // @step When the NAPI getFileDiff function is called with the repository path and file path
      const diff = getFileDiff(tempDir, 'example.ts');

      // @step Then it returns a unified diff string with lines prefixed by "+", "-", or " "
      expect(diff).not.toBeNull();
      expect(diff).toContain('-line 2');
      expect(diff).toContain('+modified line 2');
      expect(diff).toContain('+new line 4');

      // @step And the diff header contains line count information
      expect(diff).toMatch(/lines/i);

      // @step And the result is identical in format to the previous TypeScript implementation
      // Verify unified diff format: header lines + content lines with +/-/space prefix
      const lines = diff!.split('\n');
      const contentLines = lines.filter(
        (l: string) =>
          l.startsWith('+') ||
          l.startsWith('-') ||
          l.startsWith(' ') ||
          l.startsWith('---') ||
          l.startsWith('+++')
      );
      expect(contentLines.length).toBeGreaterThan(0);
    });
  });

  describe('Scenario: Checkpoint file diff via NAPI returns restore preview', () => {
    it('should return unified diff between checkpoint and HEAD', async () => {
      // @step Given a git repository with a ghost checkpoint containing a different version of a file
      await writeFile(
        join(tempDir, 'checkpoint-test.ts'),
        'original content\nline 2\n'
      );
      execSync('git add checkpoint-test.ts', { cwd: tempDir, stdio: 'pipe' });
      execSync('git commit -m "initial"', { cwd: tempDir, stdio: 'pipe' });

      // Create checkpoint at this state
      createGhostCheckpoint(tempDir, 'TEST-001', 'baseline');

      // Now modify the file and commit to HEAD
      await writeFile(
        join(tempDir, 'checkpoint-test.ts'),
        'modified content\nline 2\nnew line 3\n'
      );
      execSync('git add checkpoint-test.ts', { cwd: tempDir, stdio: 'pipe' });
      execSync('git commit -m "modify"', { cwd: tempDir, stdio: 'pipe' });

      // @step When the NAPI getCheckpointFileDiff function is called with the repository path, file path, and checkpoint ref
      const diff = getCheckpointFileDiff(
        tempDir,
        'checkpoint-test.ts',
        'refs/fspec-checkpoints/TEST-001/baseline'
      );

      // @step Then it returns a unified diff comparing HEAD content to checkpoint content
      expect(diff).not.toBeNull();

      // @step And the diff shows what will change when the checkpoint is restored
      // HEAD has "modified content", checkpoint has "original content"
      // When restoring checkpoint: HEAD content is "old", checkpoint is "new"
      expect(diff).toContain('-modified content');
      expect(diff).toContain('+original content');
    });
  });

  describe('Scenario: Checkpoint diff for file not in checkpoint returns deletion message', () => {
    it('should return deletion message for file missing from checkpoint', async () => {
      // @step Given a git repository with a ghost checkpoint
      await writeFile(join(tempDir, 'existing.ts'), 'content\n');
      execSync('git add existing.ts', { cwd: tempDir, stdio: 'pipe' });
      execSync('git commit -m "initial"', { cwd: tempDir, stdio: 'pipe' });

      createGhostCheckpoint(tempDir, 'TEST-002', 'before-new-file');

      // @step And a file that exists in HEAD but not in the checkpoint
      await writeFile(join(tempDir, 'new-file.ts'), 'added after checkpoint\n');
      execSync('git add new-file.ts', { cwd: tempDir, stdio: 'pipe' });
      execSync('git commit -m "add new file"', { cwd: tempDir, stdio: 'pipe' });

      // @step When the NAPI getCheckpointFileDiff function is called for that file
      const diff = getCheckpointFileDiff(
        tempDir,
        'new-file.ts',
        'refs/fspec-checkpoints/TEST-002/before-new-file'
      );

      // @step Then it returns a message containing "Will be deleted on restore"
      expect(diff).not.toBeNull();
      expect(diff!.toLowerCase()).toContain('will be deleted on restore');
    });
  });

  describe('Scenario: Binary file diff returns binary indicator', () => {
    it('should return binary indicator for binary files', async () => {
      // @step Given a git repository with a binary file that has uncommitted changes
      const binaryContent = Buffer.from([
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d,
      ]);
      await writeFile(join(tempDir, 'image.png'), binaryContent);
      execSync('git add image.png', { cwd: tempDir, stdio: 'pipe' });
      execSync('git commit -m "add binary"', { cwd: tempDir, stdio: 'pipe' });

      const modifiedBinary = Buffer.concat([
        binaryContent,
        Buffer.from([0x00, 0x01, 0x02]),
      ]);
      await writeFile(join(tempDir, 'image.png'), modifiedBinary);

      // @step When the NAPI getFileDiff function is called for the binary file
      const diff = getFileDiff(tempDir, 'image.png');

      // @step Then it returns "[Binary file - no diff available]"
      expect(diff).toBe('[Binary file - no diff available]');
    });
  });

  describe('Scenario: Large diff is truncated at 20000 lines', () => {
    it('should truncate diffs exceeding 20000 lines', async () => {
      // @step Given a git repository with a file that produces more than 20000 diff lines
      const initialContent = 'initial line\n';
      await writeFile(join(tempDir, 'large-file.txt'), initialContent);
      execSync('git add large-file.txt', { cwd: tempDir, stdio: 'pipe' });
      execSync('git commit -m "initial"', { cwd: tempDir, stdio: 'pipe' });

      // Generate a file with 25000 lines
      const largeLines: string[] = [];
      for (let i = 0; i < 25000; i++) {
        largeLines.push(`new line number ${i}`);
      }
      await writeFile(join(tempDir, 'large-file.txt'), largeLines.join('\n'));

      // @step When the NAPI getFileDiff function is called for that file
      const diff = getFileDiff(tempDir, 'large-file.txt');

      // @step Then the diff output contains no more than 20000 content lines
      expect(diff).not.toBeNull();
      const diffLines = diff!.split('\n');
      // The total lines should be less than the full diff would be
      // (header + 20000 content + truncation message)
      expect(diffLines.length).toBeLessThan(25000);

      // @step And the output ends with a "[File truncated" message
      expect(diff).toContain('[File truncated');
    });
  });

  describe('Scenario: FileDiffViewer uses direct NAPI call instead of worker thread', () => {
    it('should not import or use worker_threads in FileDiffViewer', () => {
      // @step Given the FileDiffViewer component is mounted with a list of changed files
      const fileDiffViewerPath = join(
        process.cwd(),
        'src',
        'tui',
        'components',
        'FileDiffViewer.tsx'
      );
      const content = readFileSync(fileDiffViewerPath, 'utf-8');

      // @step When a file is selected for diff viewing
      // @step Then the diff is loaded via a direct NAPI call without using worker_threads

      // @step And no Worker instance is created
      expect(content).not.toContain("from 'worker_threads'");
      expect(content).not.toContain('new Worker(');
      expect(content).not.toContain('getWorkerPath');

      // @step And the parsed diff lines are displayed with colored +/- indicators
      // Verify it still uses parseDiff for rendering
      expect(content).toContain('parseDiff');
    });
  });

  describe('Scenario: CheckpointViewer uses direct NAPI call instead of worker thread', () => {
    it('should not import or use worker_threads in CheckpointViewer', () => {
      // @step Given the CheckpointViewer component is mounted with checkpoint data
      const checkpointViewerPath = join(
        process.cwd(),
        'src',
        'tui',
        'components',
        'CheckpointViewer.tsx'
      );
      const content = readFileSync(checkpointViewerPath, 'utf-8');

      // @step When a checkpoint file is selected for diff viewing
      // @step Then the checkpoint diff is loaded via a direct NAPI call without using worker_threads

      // @step And no Worker instance is created
      expect(content).not.toContain("from 'worker_threads'");
      expect(content).not.toContain('new Worker(');
      expect(content).not.toContain('getWorkerPath');

      // @step And the restore preview is displayed with colored +/- indicators
      expect(content).toContain('parseDiff');
    });
  });

  describe('Scenario: Build pipeline does not include esbuild diff-worker step', () => {
    it('should not have esbuild diff-worker in package.json build script', () => {
      // @step Given the package.json build script
      const packageJsonPath = join(process.cwd(), 'package.json');
      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));
      const buildScript: string = packageJson.scripts?.build ?? '';

      // @step When "npm run build" is executed
      // @step Then the build does not run any esbuild command for diff-worker.ts
      expect(buildScript).not.toContain('diff-worker');
      expect(buildScript).not.toContain('esbuild');

      // @step And the build succeeds with only the Vite bundle and NAPI build steps
      expect(buildScript).toContain('vite build');

      // @step And no dist/git/diff-worker.js file is produced
      const workerFile = join(process.cwd(), 'dist', 'git', 'diff-worker.js');
      expect(existsSync(workerFile)).toBe(false);
    });
  });

  describe('Scenario: Legacy diff-worker files are removed from codebase', () => {
    it('should not have diff-worker or worker-path files in source tree', () => {
      // @step Given the fspec source tree
      const srcDir = join(process.cwd(), 'src');

      // @step Then src/git/diff-worker.ts does not exist
      expect(existsSync(join(srcDir, 'git', 'diff-worker.ts'))).toBe(false);

      // @step And src/git/worker-path.ts does not exist
      expect(existsSync(join(srcDir, 'git', 'worker-path.ts'))).toBe(false);

      // @step And src/tui/components/__tests__/worker-path-resolution.test.tsx does not exist
      expect(
        existsSync(
          join(
            srcDir,
            'tui',
            'components',
            '__tests__',
            'worker-path-resolution.test.tsx'
          )
        )
      ).toBe(false);

      // @step And no source file imports from "worker-path" or "diff-worker"
      // Check key files that previously imported these modules
      const fileDiffViewer = readFileSync(
        join(srcDir, 'tui', 'components', 'FileDiffViewer.tsx'),
        'utf-8'
      );
      const checkpointViewer = readFileSync(
        join(srcDir, 'tui', 'components', 'CheckpointViewer.tsx'),
        'utf-8'
      );
      // Check that no import statements reference the old modules
      expect(fileDiffViewer).not.toMatch(/import.*from.*['"].*worker-path['"]/);
      expect(fileDiffViewer).not.toMatch(/import.*from.*['"].*diff-worker['"]/);
      expect(checkpointViewer).not.toMatch(
        /import.*from.*['"].*worker-path['"]/
      );
      expect(checkpointViewer).not.toMatch(
        /import.*from.*['"].*diff-worker['"]/
      );
    });
  });
});
