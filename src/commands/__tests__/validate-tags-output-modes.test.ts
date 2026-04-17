/**
 * Feature: spec/features/validate-tags-failures-only-and-summary-flags.feature
 *
 * This test file validates the CLI output modes for `fspec validate-tags`:
 * - default (failures-only)
 * - --verbose (restores per-file ✓ lines)
 * - --summary (only the summary count lines)
 * - --summary wins when combined with --verbose
 *
 * Scenarios map directly to the Gherkin scenarios in the feature file.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mkdir, writeFile } from 'fs/promises';
import { join } from 'path';
import { validateTagsCommand } from '../validate-tags';
import {
  setupWorkUnitTest,
  type WorkUnitTestSetup,
} from '../../test-helpers/universal-test-setup';
import {
  createCaptureContext,
  setOutputContext,
  resetOutputContext,
} from '../../utils/output';

interface CaptureHandles {
  stdout: string[];
  stderr: string[];
}

interface ExitRecorder {
  code: number | undefined;
}

async function writeTagsJson(specDir: string): Promise<void> {
  const tagsJson = {
    $schema: '../src/schemas/tags.schema.json',
    categories: [
      {
        name: 'Phase Tags',
        description: 'Development phase tags',
        required: false,
        tags: [{ name: '@critical', description: 'Phase 1' }],
      },
      {
        name: 'Component Tags',
        description: 'Architectural components',
        required: false,
        tags: [{ name: '@cli', description: 'CLI' }],
      },
      {
        name: 'Feature Group Tags',
        description: 'Functional areas',
        required: false,
        tags: [
          { name: '@authentication', description: 'Authentication' },
          { name: '@validation', description: 'Validation' },
        ],
      },
    ],
    combinationExamples: [],
    usageGuidelines: {
      minimumTagsPerFeature: 1,
      recommendedTagsPerFeature: 3,
      tagNamingConvention: 'kebab-case with @ prefix',
    },
  };

  await mkdir(specDir, { recursive: true });
  await writeFile(
    join(specDir, 'tags.json'),
    JSON.stringify(tagsJson, null, 2)
  );
}

function validFeatureContent(title: string): string {
  return `@critical @cli @authentication
Feature: ${title}

  Scenario: Example
    Given a step
    When an action
    Then a result
`;
}

function invalidFeatureContent(title: string): string {
  return `@critical @cli @authentication @nonexistent
Feature: ${title}

  Scenario: Example
    Given a step
    When an action
    Then a result
`;
}

describe('Feature: validate-tags default output shows only failures, with opt-in --verbose and --summary flags', () => {
  let setup: WorkUnitTestSetup;
  let originalCwd: string;
  let processExitSpy: ReturnType<typeof vi.spyOn>;
  let captured: CaptureHandles;
  let exitRecorder: ExitRecorder;

  beforeEach(async () => {
    setup = await setupWorkUnitTest('validate-tags-output-modes');
    originalCwd = process.cwd();
    process.chdir(setup.testDir);

    await writeTagsJson(setup.specDir);

    const { context, stdout, stderr } = createCaptureContext();
    setOutputContext(context);
    captured = { stdout, stderr };

    exitRecorder = { code: undefined };
    processExitSpy = vi.spyOn(process, 'exit').mockImplementation(((
      code?: number
    ) => {
      if (exitRecorder.code === undefined) {
        exitRecorder.code = code;
      }
      return undefined as never;
    }) as never);
  });

  afterEach(async () => {
    processExitSpy.mockRestore();
    resetOutputContext();
    process.chdir(originalCwd);
    await setup.cleanup();
  });

  describe('Scenario: Default behavior on an all-valid tree prints no per-file ✓ lines', () => {
    it('should exit with code 0 and print only the summary line', async () => {
      // @step Given a project with 3 feature files that all have valid registered tags
      const featuresDir = join(setup.testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });
      await writeFile(
        join(featuresDir, 'one.feature'),
        validFeatureContent('One')
      );
      await writeFile(
        join(featuresDir, 'two.feature'),
        validFeatureContent('Two')
      );
      await writeFile(
        join(featuresDir, 'three.feature'),
        validFeatureContent('Three')
      );

      // @step When I run `fspec validate-tags` with no flags
      await validateTagsCommand(undefined, {});

      // @step Then the command should exit with code 0
      expect(exitRecorder.code).toBe(0);

      // @step And the output should NOT contain any line starting with "✓ All tags in "
      const passLines = captured.stdout.filter(line =>
        line.startsWith('✓ All tags in ')
      );
      expect(passLines).toHaveLength(0);

      // @step And the output should contain exactly one line: "✓ 3 files passed"
      const summaryLines = captured.stdout.filter(
        line => line === '✓ 3 files passed'
      );
      expect(summaryLines).toHaveLength(1);
    });
  });

  describe('Scenario: Default behavior with some failures prints only violation blocks plus summary', () => {
    it('should exit with code 1 and print only violation blocks + summary', async () => {
      // @step Given a project with 5 feature files
      const featuresDir = join(setup.testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // @step And 3 of those files have valid registered tags
      await writeFile(
        join(featuresDir, 'valid1.feature'),
        validFeatureContent('Valid1')
      );
      await writeFile(
        join(featuresDir, 'valid2.feature'),
        validFeatureContent('Valid2')
      );
      await writeFile(
        join(featuresDir, 'valid3.feature'),
        validFeatureContent('Valid3')
      );

      // @step And 2 of those files contain an unregistered tag "@nonexistent"
      await writeFile(
        join(featuresDir, 'bad1.feature'),
        invalidFeatureContent('Bad1')
      );
      await writeFile(
        join(featuresDir, 'bad2.feature'),
        invalidFeatureContent('Bad2')
      );

      // @step When I run `fspec validate-tags` with no flags
      await validateTagsCommand(undefined, {});

      // @step Then the command should exit with code 1
      expect(exitRecorder.code).toBe(1);

      // @step And the output should NOT contain any line starting with "✓ All tags in "
      const passLines = captured.stdout.filter(line =>
        line.startsWith('✓ All tags in ')
      );
      expect(passLines).toHaveLength(0);

      // @step And the output should contain exactly 2 "✗ <file> has tag violations:" blocks
      const violationHeaderLines = captured.stdout.filter(line =>
        /^✗ .+ has tag violations:$/.test(line)
      );
      expect(violationHeaderLines).toHaveLength(2);

      // @step And the output should contain the line "✓ 3 files passed"
      expect(captured.stdout).toContain('✓ 3 files passed');

      // @step And the output should contain the line "✗ 2 files have tag violations"
      expect(captured.stdout).toContain('✗ 2 files have tag violations');
    });
  });

  describe('Scenario: --verbose on an all-valid tree restores the old one-line-per-file output', () => {
    it('should exit 0 and print one ✓ line per file plus summary', async () => {
      // @step Given a project with 3 feature files that all have valid registered tags
      const featuresDir = join(setup.testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });
      await writeFile(
        join(featuresDir, 'one.feature'),
        validFeatureContent('One')
      );
      await writeFile(
        join(featuresDir, 'two.feature'),
        validFeatureContent('Two')
      );
      await writeFile(
        join(featuresDir, 'three.feature'),
        validFeatureContent('Three')
      );

      // @step When I run `fspec validate-tags --verbose`
      await validateTagsCommand(undefined, { verbose: true });

      // @step Then the command should exit with code 0
      expect(exitRecorder.code).toBe(0);

      // @step And the output should contain exactly 3 lines starting with "✓ All tags in "
      const passLines = captured.stdout.filter(line =>
        line.startsWith('✓ All tags in ')
      );
      expect(passLines).toHaveLength(3);

      // @step And the output should contain the line "✓ 3 files passed"
      expect(captured.stdout).toContain('✓ 3 files passed');
    });
  });

  describe('Scenario: --summary prints only the two summary count lines when some files fail', () => {
    it('should suppress all per-file output and print only the count lines', async () => {
      // @step Given a project with 5 feature files
      const featuresDir = join(setup.testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // @step And 3 of those files have valid registered tags
      await writeFile(
        join(featuresDir, 'valid1.feature'),
        validFeatureContent('Valid1')
      );
      await writeFile(
        join(featuresDir, 'valid2.feature'),
        validFeatureContent('Valid2')
      );
      await writeFile(
        join(featuresDir, 'valid3.feature'),
        validFeatureContent('Valid3')
      );

      // @step And 2 of those files contain an unregistered tag "@nonexistent"
      await writeFile(
        join(featuresDir, 'bad1.feature'),
        invalidFeatureContent('Bad1')
      );
      await writeFile(
        join(featuresDir, 'bad2.feature'),
        invalidFeatureContent('Bad2')
      );

      // @step When I run `fspec validate-tags --summary`
      await validateTagsCommand(undefined, { summary: true });

      // @step Then the command should exit with code 1
      expect(exitRecorder.code).toBe(1);

      // @step And the output should NOT contain any line starting with "✓ All tags in "
      const passLines = captured.stdout.filter(line =>
        line.startsWith('✓ All tags in ')
      );
      expect(passLines).toHaveLength(0);

      // @step And the output should NOT contain any line starting with "✗ " followed by a file path
      const violationHeaderLines = captured.stdout.filter(line =>
        /^✗ .+ has tag violations:$/.test(line)
      );
      expect(violationHeaderLines).toHaveLength(0);

      // @step And the output should contain the line "✓ 3 files passed"
      expect(captured.stdout).toContain('✓ 3 files passed');

      // @step And the output should contain the line "✗ 2 files have tag violations"
      expect(captured.stdout).toContain('✗ 2 files have tag violations');
    });
  });

  describe('Scenario: Default behavior on a single valid file produces no output', () => {
    it('should exit 0 with no stdout output', async () => {
      // @step Given a single feature file "spec/features/foo.feature" with valid registered tags
      const featuresDir = join(setup.testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });
      await writeFile(
        join(featuresDir, 'foo.feature'),
        validFeatureContent('Foo')
      );

      // @step When I run `fspec validate-tags spec/features/foo.feature`
      await validateTagsCommand('spec/features/foo.feature', {});

      // @step Then the command should exit with code 0
      expect(exitRecorder.code).toBe(0);

      // @step And the output should be empty
      expect(captured.stdout).toHaveLength(0);
    });
  });

  describe('Scenario: --summary combined with --verbose behaves identically to --summary alone', () => {
    it('should print only the two summary count lines when both flags are passed', async () => {
      // @step Given a project with 2 feature files
      const featuresDir = join(setup.testDir, 'spec', 'features');
      await mkdir(featuresDir, { recursive: true });

      // @step And 1 of those files has valid registered tags
      await writeFile(
        join(featuresDir, 'good.feature'),
        validFeatureContent('Good')
      );

      // @step And 1 of those files contains an unregistered tag "@nonexistent"
      await writeFile(
        join(featuresDir, 'bad.feature'),
        invalidFeatureContent('Bad')
      );

      // @step When I run `fspec validate-tags --summary --verbose`
      await validateTagsCommand(undefined, {
        summary: true,
        verbose: true,
      });

      // @step Then the command should exit with code 1
      expect(exitRecorder.code).toBe(1);

      // @step And the output should NOT contain any line starting with "✓ All tags in "
      const passLines = captured.stdout.filter(line =>
        line.startsWith('✓ All tags in ')
      );
      expect(passLines).toHaveLength(0);

      // @step And the output should NOT contain any "✗ <file> has tag violations:" block
      const violationHeaderLines = captured.stdout.filter(line =>
        /^✗ .+ has tag violations:$/.test(line)
      );
      expect(violationHeaderLines).toHaveLength(0);

      // @step And the output should contain exactly these two lines: | ✓ 1 files passed | ✗ 1 files have tag violations |
      const nonEmpty = captured.stdout.filter(line => line.length > 0);
      expect(nonEmpty).toEqual([
        '✓ 1 files passed',
        '✗ 1 files have tag violations',
      ]);
    });
  });
});
