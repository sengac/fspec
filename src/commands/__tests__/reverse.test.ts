/**
 * Feature: spec/features/interactive-reverse-acdd-strategy-planning-command.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { promises as fs } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { reverse } from '../reverse.js';
import { getSessionPath } from '../../utils/reverse-session.js';

describe('Feature: Interactive reverse ACDD strategy planning command', () => {
  let testDir: string;
  let sessionFile: string;

  beforeEach(async () => {
    // Create temporary test directory
    testDir = join(tmpdir(), `fspec-test-${Date.now()}`);
    await fs.mkdir(testDir, { recursive: true });

    // Session file will be in OS tmpdir with project-specific hash
    sessionFile = await getSessionPath(testDir);
  });

  afterEach(async () => {
    // Cleanup test directory
    await fs.rm(testDir, { recursive: true, force: true });

    // Cleanup session file (may be in OS tmpdir)
    try {
      await fs.unlink(sessionFile);
    } catch {
      // Session file may not exist - that's fine
    }
  });

  describe('Scenario: Initial analysis detects test files without features and suggests Strategy A', () => {
    it('should analyze project and suggest Strategy A when tests exist without features', async () => {
      // Given: I have a project with 3 test files in "src/__tests__/"
      const testFilesDir = join(testDir, 'src', '__tests__');
      await fs.mkdir(testFilesDir, { recursive: true });
      await fs.writeFile(join(testFilesDir, 'auth.test.ts'), 'test content');
      await fs.writeFile(join(testFilesDir, 'user.test.ts'), 'test content');
      await fs.writeFile(join(testFilesDir, 'api.test.ts'), 'test content');

      // And: those test files have no corresponding feature files
      // (no feature files exist)

      // And: no reverse session exists
      // (session file doesn't exist)

      // When: I run "fspec reverse"
      const result = await reverse({ cwd: testDir });

      // Then: the command should analyze the project structure
      expect(result.analysis).toBeDefined();
      expect(result.analysis.testFiles).toHaveLength(3);

      // And: the command should create "spec/.reverse-session.json"
      await expect(fs.access(sessionFile)).resolves.not.toThrow();

      // And: the session should be in "gap-detection" phase
      const session = JSON.parse(await fs.readFile(sessionFile, 'utf-8'));
      expect(session.phase).toBe('gap-detection');

      // And: the output should show "3 test files without features"
      expect(result.gaps.testsWithoutFeatures).toBe(3);

      // And: the output should suggest "Strategy A: Spec Gap Filling"
      expect(result.suggestedStrategy).toBe('A');
      expect(result.strategyName).toBe('Spec Gap Filling');

      // And: the output should emit a system-reminder with strategy guidance
      expect(result.systemReminder).toContain('Strategy A');
      expect(result.systemReminder).toContain('Spec Gap Filling');
    });
  });

  describe('Scenario: Choose Strategy A and receive step-by-step guidance', () => {
    it('should transition to executing phase and emit step 1 guidance when strategy A is chosen', async () => {
      // Given: I have a reverse session in "strategy-planning" phase
      await fs.mkdir(join(testDir, 'spec'), { recursive: true });
      await fs.writeFile(
        sessionFile,
        JSON.stringify({
          phase: 'strategy-planning',
          gaps: {
            testsWithoutFeatures: 3,
            files: ['test1.ts', 'test2.ts', 'test3.ts'],
          },
        })
      );

      // And: the session detected 3 test files without features
      // (already in session data above)

      // When: I run "fspec reverse --strategy=A"
      const result = await reverse({ cwd: testDir, strategy: 'A' });

      // Then: the session should transition to "executing" phase
      const session = JSON.parse(await fs.readFile(sessionFile, 'utf-8'));
      expect(session.phase).toBe('executing');

      // And: the session should set currentStep to 1
      expect(session.currentStep).toBe(1);

      // And: the command should emit a system-reminder with step 1 guidance
      expect(result.systemReminder).toContain('Step 1');

      // And: the guidance should instruct me to read the first test file
      expect(result.guidance).toContain('Read test file');
      expect(result.guidance).toContain('test1.ts');

      // And: the guidance should instruct me to create a feature file
      expect(result.guidance).toContain('create feature file');

      // And: the guidance should instruct me to run "fspec link-coverage" with --skip-validation
      expect(result.guidance).toContain('fspec link-coverage');
      expect(result.guidance).toContain('--skip-validation');
    });
  });

  describe('Scenario: Continue to next step in strategy execution', () => {
    it('should increment step and emit next guidance when continuing', async () => {
      // Given: I have a reverse session in "executing" phase
      // And: the session is on step 1 of 3
      await fs.mkdir(join(testDir, 'spec'), { recursive: true });
      await fs.writeFile(
        sessionFile,
        JSON.stringify({
          phase: 'executing',
          strategy: 'A',
          currentStep: 1,
          totalSteps: 3,
          gaps: { files: ['test1.ts', 'test2.ts', 'test3.ts'] },
        })
      );

      // And: I have completed the work for step 1
      // (assume work is done)

      // When: I run "fspec reverse --continue"
      const result = await reverse({ cwd: testDir, continue: true });

      // Then: the session should increment currentStep to 2
      const session = JSON.parse(await fs.readFile(sessionFile, 'utf-8'));
      expect(session.currentStep).toBe(2);

      // And: the command should emit a system-reminder with step 2 guidance
      expect(result.systemReminder).toContain('Step 2');

      // And: the guidance should reference the next test file to process
      expect(result.guidance).toContain('test2.ts');
    });
  });

  describe('Scenario: Check session status during execution', () => {
    it('should display current session status with progress details', async () => {
      // Given: I have a reverse session in "executing" phase
      // And: the chosen strategy is "A"
      // And: the session detected "3 test files without features"
      // And: the current step is 2 of 3
      await fs.mkdir(join(testDir, 'spec'), { recursive: true });
      await fs.writeFile(
        sessionFile,
        JSON.stringify({
          phase: 'executing',
          strategy: 'A',
          currentStep: 2,
          totalSteps: 3,
          gaps: {
            testsWithoutFeatures: 3,
            files: ['test1.ts', 'test2.ts', 'test3.ts'],
          },
        })
      );

      // When: I run "fspec reverse --status"
      const result = await reverse({ cwd: testDir, status: true });

      // Then: the output should show "Phase: executing"
      expect(result.phase).toBe('executing');

      // And: the output should show "Strategy: A (Spec Gap Filling)"
      expect(result.strategy).toBe('A');
      expect(result.strategyName).toBe('Spec Gap Filling');

      // And: the output should show "Gaps detected: 3 test files without features"
      expect(result.gapsDetected).toBe('3 test files without features');

      // And: the output should show "Progress: Step 2 of 3"
      expect(result.progress).toBe('Step 2 of 3');

      // And: the output should list the 3 gaps with completion status
      expect(result.gapList).toHaveLength(3);
      expect(result.gapList[0].completed).toBe(true); // Step 1 done
      expect(result.gapList[1].completed).toBe(false); // Step 2 in progress
      expect(result.gapList[2].completed).toBe(false); // Step 3 pending
    });
  });

  describe('Scenario: Reset session and start fresh', () => {
    it('should delete session file and allow fresh start when reset', async () => {
      // Given: I have a reverse session in "executing" phase
      // And: the session file exists
      await fs.mkdir(join(testDir, 'spec'), { recursive: true });
      await fs.writeFile(
        sessionFile,
        JSON.stringify({
          phase: 'executing',
          strategy: 'A',
          currentStep: 2,
        })
      );

      // When: I run "fspec reverse --reset"
      const result = await reverse({ cwd: testDir, reset: true });

      // Then: the session file should be deleted
      await expect(fs.access(sessionFile)).rejects.toThrow();

      // And: the output should show "Session reset"
      expect(result.message).toBe('Session reset');

      // And: I should be able to run "fspec reverse" to start a new session
      const freshStart = await reverse({ cwd: testDir });
      expect(freshStart.analysis).toBeDefined();
    });
  });

  describe('Scenario: Prevent starting new session when one already exists', () => {
    it('should detect existing session and prevent overwrite with guidance', async () => {
      // Given: I have a reverse session in "executing" phase
      // And: the session is on step 2 of 3
      // And: the session file exists
      await fs.writeFile(
        sessionFile,
        JSON.stringify({
          phase: 'executing',
          strategy: 'A',
          strategyName: 'Spec Gap Filling',
          currentStep: 2,
          totalSteps: 3,
          gaps: {
            testsWithoutFeatures: 3,
            files: ['test1.ts', 'test2.ts', 'test3.ts'],
          },
        })
      );

      // When: I run "fspec reverse"
      const result = await reverse({ cwd: testDir });

      // Then: the command should detect the existing session
      expect(result.existingSessionDetected).toBe(true);

      // And: the command should exit with error code 1
      expect(result.exitCode).toBe(1);

      // And: the output should show "Existing reverse session detected"
      expect(result.message).toContain('Existing reverse session detected');

      // And: the output should show the current session phase "executing"
      expect(result.currentPhase).toBe('executing');

      // And: the output should show the current strategy and progress
      expect(result.currentStrategy).toBe('A (Spec Gap Filling)');
      expect(result.currentProgress).toBe('Step 2 of 3');

      // And: the output should suggest running "fspec reverse --continue"
      expect(result.suggestions).toContain('fspec reverse --continue');

      // And: the output should suggest running "fspec reverse --status"
      expect(result.suggestions).toContain('fspec reverse --status');

      // And: the output should suggest running "fspec reverse --reset"
      expect(result.suggestions).toContain('fspec reverse --reset');

      // And: the output should suggest running "fspec reverse --complete"
      expect(result.suggestions).toContain('fspec reverse --complete');

      // And: the command should NOT overwrite the existing session file
      const sessionAfter = JSON.parse(await fs.readFile(sessionFile, 'utf-8'));
      expect(sessionAfter.currentStep).toBe(2); // Unchanged

      // And: the command should emit a system-reminder about the conflict
      expect(result.systemReminder).toContain('Existing session detected');
      expect(result.systemReminder).toContain('DO NOT start new session');
    });
  });

  describe('Scenario: Reach final step and receive completion guidance', () => {
    it('should tell AI to run --complete (not --continue) on final step', async () => {
      // Given: I have a reverse session in "executing" phase
      // And: the session is on step 2 of 3
      await fs.writeFile(
        sessionFile,
        JSON.stringify({
          phase: 'executing',
          strategy: 'A',
          currentStep: 2,
          totalSteps: 3,
          gaps: { files: ['test1.ts', 'test2.ts', 'test3.ts'] },
        })
      );

      // And: I have completed the work for step 2

      // When: I run "fspec reverse --continue"
      const result = await reverse({ cwd: testDir, continue: true });

      // Then: the session should increment currentStep to 3
      const session = JSON.parse(await fs.readFile(sessionFile, 'utf-8'));
      expect(session.currentStep).toBe(3);

      // And: the command should emit a system-reminder with step 3 guidance
      expect(result.systemReminder).toContain('Step 3');

      // And: the guidance should reference the final test file to process
      expect(result.guidance).toContain('test3.ts');

      // And: the system-reminder should tell me to run "fspec reverse --complete" after completing step 3
      expect(result.systemReminder).toContain('fspec reverse --complete');

      // And: the system-reminder should NOT mention "fspec reverse --continue"
      expect(result.systemReminder).not.toContain('--continue');
    });
  });

  describe('Scenario: Detect feature files without tests and suggest Strategy B', () => {
    it('should suggest Strategy B when features exist without tests', async () => {
      // Given: I have a project with 2 feature files in "spec/features/"
      const featuresDir = join(testDir, 'spec', 'features');
      await fs.mkdir(featuresDir, { recursive: true });
      await fs.writeFile(join(featuresDir, 'auth.feature'), 'feature content');
      await fs.writeFile(join(featuresDir, 'user.feature'), 'feature content');

      // And: those feature files have no corresponding test files
      // (no test files exist)

      // And: no reverse session exists

      // When: I run "fspec reverse"
      const result = await reverse({ cwd: testDir });

      // Then: the command should analyze the project structure
      // And: the session should be in "gap-detection" phase
      expect(result.analysis).toBeDefined();

      // And: the output should show "2 feature files without tests"
      expect(result.gaps.featuresWithoutTests).toBe(2);

      // And: the output should suggest "Strategy B: Test Gap Filling"
      expect(result.suggestedStrategy).toBe('B');
      expect(result.strategyName).toBe('Test Gap Filling');

      // And: the guidance should mention creating test skeletons
      expect(result.guidance).toContain('test skeletons');

      // And: the guidance should mention using --skip-validation flag
      expect(result.guidance).toContain('--skip-validation');
    });
  });

  describe('Scenario: Detect unmapped coverage and suggest Strategy C', () => {
    it('should suggest Strategy C when tests and features exist but unmapped', async () => {
      // Given: I have a project with 5 feature files
      const featuresDir = join(testDir, 'spec', 'features');
      await fs.mkdir(featuresDir, { recursive: true });
      for (let i = 1; i <= 5; i++) {
        await fs.writeFile(
          join(featuresDir, `feature${i}.feature`),
          'feature content'
        );
      }

      // And: I have corresponding test files for those features
      const testFilesDir = join(testDir, 'src', '__tests__');
      await fs.mkdir(testFilesDir, { recursive: true });
      for (let i = 1; i <= 5; i++) {
        await fs.writeFile(
          join(testFilesDir, `feature${i}.test.ts`),
          'test content'
        );
      }

      // And: the coverage files show 5 scenarios without test mappings
      // (coverage files exist but have empty mappings)
      for (let i = 1; i <= 5; i++) {
        const coverageFile = join(featuresDir, `feature${i}.feature.coverage`);
        await fs.writeFile(
          coverageFile,
          JSON.stringify({
            scenarios: [{ name: `Scenario ${i}`, testMappings: [] }],
          })
        );
      }

      // And: no reverse session exists

      // When: I run "fspec reverse"
      const result = await reverse({ cwd: testDir });

      // Then: the command should analyze coverage files
      // And: the session should be in "gap-detection" phase
      expect(result.analysis.coverageAnalysis).toBeDefined();

      // And: the output should show "5 scenarios without coverage mappings"
      expect(result.gaps.unmappedScenarios).toBe(5);

      // And: the output should suggest "Strategy C: Coverage Mapping"
      expect(result.suggestedStrategy).toBe('C');
      expect(result.strategyName).toBe('Coverage Mapping');

      // And: the guidance should mention "Quick wins - no new files needed"
      expect(result.guidance).toContain('Quick wins');
      expect(result.guidance).toContain('no new files');

      // And: the guidance should estimate effort as "1 point total"
      expect(result.effortEstimate).toBe('1 point total');
    });
  });

  describe('Scenario: Detect raw implementation with no specs or tests and suggest Strategy D', () => {
    it('should suggest Strategy D when only implementation code exists', async () => {
      // Given: I have a project with 10 implementation files in "src/"
      const srcDir = join(testDir, 'src');
      await fs.mkdir(srcDir, { recursive: true });
      for (let i = 1; i <= 10; i++) {
        await fs.writeFile(
          join(srcDir, `module${i}.ts`),
          'implementation code'
        );
      }

      // And: the project has no feature files
      // And: the project has no test files
      // (no features or tests exist)

      // And: no reverse session exists

      // When: I run "fspec reverse"
      const result = await reverse({ cwd: testDir });

      // Then: the command should analyze implementation files
      expect(result.analysis.implementationFiles).toHaveLength(10);

      // And: the session should be in "gap-detection" phase
      // And: the output should show "10 implementation files without specs or tests"
      expect(result.gaps.rawImplementation).toBe(10);

      // And: the output should suggest "Strategy D: Full Reverse ACDD"
      expect(result.suggestedStrategy).toBe('D');
      expect(result.strategyName).toBe('Full Reverse ACDD');

      // And: the guidance should mention "Highest effort - analyze code, create features, tests, and work units"
      expect(result.guidance).toContain('Highest effort');
      expect(result.guidance).toContain('analyze code');
      expect(result.guidance).toContain(
        'create features, tests, and work units'
      );

      // And: the guidance should estimate effort range
      expect(result.effortEstimate).toMatch(/\d+-\d+ points/);
    });
  });

  describe('Scenario: Complete all steps and finalize session', () => {
    it('should validate work and delete session when completing', async () => {
      // Given: I have a reverse session in "executing" phase
      // And: I am on the final step (step 3 of 3)
      // And: I have completed all gap-filling work
      await fs.mkdir(join(testDir, 'spec'), { recursive: true });
      await fs.writeFile(
        sessionFile,
        JSON.stringify({
          phase: 'executing',
          strategy: 'A',
          currentStep: 3,
          totalSteps: 3,
          gaps: { files: ['test1.ts', 'test2.ts', 'test3.ts'] },
          completed: ['test1.ts', 'test2.ts', 'test3.ts'],
        })
      );

      // When: I run "fspec reverse --complete"
      const result = await reverse({ cwd: testDir, complete: true });

      // Then: the command should validate all work was completed
      expect(result.validationComplete).toBe(true);

      // And: the command should verify gaps are filled
      expect(result.gapsFilled).toBe(true);

      // And: the session file should be deleted
      await expect(fs.access(sessionFile)).rejects.toThrow();

      // And: the command should emit a system-reminder with completion summary
      expect(result.systemReminder).toContain('Session complete');
      expect(result.systemReminder).toContain('All gaps filled');

      // And: the output should show "✓ Reverse ACDD session complete"
      expect(result.message).toContain('✓ Reverse ACDD session complete');
    });
  });

  describe('Scenario: Preview analysis without creating session (dry-run mode)', () => {
    it('should analyze and suggest strategies without creating session file', async () => {
      // Given: I have a project with gaps (missing features or tests)
      const testFilesDir = join(testDir, 'src', '__tests__');
      await fs.mkdir(testFilesDir, { recursive: true });
      await fs.writeFile(join(testFilesDir, 'test.ts'), 'test content');

      // And: no reverse session exists

      // When: I run "fspec reverse --dry-run"
      const result = await reverse({ cwd: testDir, dryRun: true });

      // Then: the command should analyze the project structure
      expect(result.analysis).toBeDefined();

      // And: the command should detect gaps
      expect(result.gaps).toBeDefined();

      // And: the command should suggest strategies
      expect(result.suggestedStrategy).toBeDefined();

      // And: the command should NOT create "spec/.reverse-session.json"
      await expect(fs.access(sessionFile)).rejects.toThrow();

      // And: the output should show gap analysis results
      expect(result.analysis.summary).toBeDefined();

      // And: the output should show "Dry-run mode - no session created"
      expect(result.message).toContain('Dry-run mode - no session created');
    });
  });

  describe('Scenario: Handle large projects with pagination', () => {
    it('should paginate output when detecting 100+ gaps', async () => {
      // Given: I have a project with 150 test files without features
      const testFilesDir = join(testDir, 'src', '__tests__');
      await fs.mkdir(testFilesDir, { recursive: true });
      for (let i = 1; i <= 150; i++) {
        await fs.writeFile(
          join(testFilesDir, `test${i}.test.ts`),
          'test content'
        );
      }

      // And: no reverse session exists

      // When: I run "fspec reverse"
      const result = await reverse({ cwd: testDir });

      // Then: the command should analyze the project structure
      // And: the output should show "150 test files without features"
      expect(result.gaps.testsWithoutFeatures).toBe(150);

      // And: the output should paginate the detailed gap list
      expect(result.pagination).toBeDefined();
      expect(result.pagination.total).toBe(150);
      expect(result.pagination.perPage).toBeLessThan(150);

      // And: the output should show a summary with total counts
      expect(result.summary).toContain('150 test files');

      // And: the guidance should suggest "Use --strategy=A to narrow scope"
      expect(result.guidance).toContain('Use --strategy=A to narrow scope');
    });
  });
});
