/**
 * Feature: spec/features/discover-foundation-finalize-does-not-auto-create-foundation-event-storm-work-unit.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Tests ensure discover-foundation --finalize auto-creates FOUND prefix and Foundation Event Storm work unit.
 *
 * CRITICAL: Tests use isolated temporary directories and NEVER write to actual spec/work-units.json
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { discoverFoundation } from '../discover-foundation';
import { mkdir, writeFile, rm, readFile } from 'fs/promises';
import { join } from 'path';
import type { GenericFoundation } from '../../types/generic-foundation';
import type { WorkUnitsData } from '../../types/work-unit';

describe('Feature: BUG-084 - discover-foundation --finalize does not auto-create Foundation Event Storm work unit', () => {
  let testDir: string;
  let draftPath: string;
  let finalPath: string;
  let workUnitsPath: string;
  let prefixesPath: string;

  // Minimal valid foundation draft (no placeholders)
  const completedDraft: GenericFoundation = {
    version: '2.0.0',
    project: {
      name: 'Test Project',
      vision: 'Test vision for foundation event storm',
      projectType: 'cli-tool',
    },
    problemSpace: {
      primaryProblem: {
        title: 'Test Problem',
        description: 'Test problem description',
        impact: 'high',
      },
    },
    solutionSpace: {
      overview: 'Test solution overview',
      capabilities: [
        {
          name: 'Test Capability',
          description: 'Test capability description',
        },
      ],
    },
    personas: [
      {
        name: 'Test Persona',
        description: 'Test persona description',
        goals: ['Test goal'],
      },
    ],
    eventStorm: {
      level: 'big_picture',
      items: [],
      nextItemId: 1,
    },
    architectureDiagrams: [],
  };

  beforeEach(async () => {
    // Create isolated temp directory for tests (CRITICAL: never write to actual spec/)
    testDir = join(process.cwd(), `test-temp-${Date.now()}`);
    draftPath = join(testDir, 'spec/foundation.json.draft');
    finalPath = join(testDir, 'spec/foundation.json');
    workUnitsPath = join(testDir, 'spec/work-units.json');
    prefixesPath = join(testDir, 'spec/prefixes.json');
    await mkdir(join(testDir, 'spec'), { recursive: true });

    // Create minimal prefixes.json structure (EMPTY prefixes)
    const emptyPrefixesData = {
      prefixes: {},
    };
    await writeFile(
      prefixesPath,
      JSON.stringify(emptyPrefixesData, null, 2),
      'utf-8'
    );

    // Create minimal epics.json structure (EMPTY epics)
    const epicsPath = join(testDir, 'spec/epics.json');
    const emptyEpicsData = {
      epics: {},
    };
    await writeFile(
      epicsPath,
      JSON.stringify(emptyEpicsData, null, 2),
      'utf-8'
    );

    // Create minimal work-units.json structure with EMPTY work units
    const emptyWorkUnitsData: WorkUnitsData = {
      prefixes: {},
      workUnits: {},
      epics: {},
      states: {
        backlog: [],
        specifying: [],
        testing: [],
        implementing: [],
        validating: [],
        done: [],
        blocked: [],
      },
    };
    await writeFile(
      workUnitsPath,
      JSON.stringify(emptyWorkUnitsData, null, 2),
      'utf-8'
    );
  });

  afterEach(async () => {
    // Clean up isolated test directory
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: First finalize auto-creates FOUND-001 work unit in isolated test directory', () => {
    it('should create FOUND-001 work unit with title "Foundation Event Storm"', async () => {
      // @step Given I am in an isolated temporary test directory
      // (already set up in beforeEach)

      // @step And I have a completed foundation.json.draft file
      await writeFile(
        draftPath,
        JSON.stringify(completedDraft, null, 2),
        'utf-8'
      );

      // @step And the FOUND prefix does not exist yet
      const beforeData = JSON.parse(
        await readFile(workUnitsPath, 'utf-8')
      ) as WorkUnitsData;
      expect(beforeData.prefixes['FOUND']).toBeUndefined();

      // @step When I run "fspec discover-foundation --finalize"
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        cwd: testDir,
      });

      // @step Then the command should succeed
      expect(result.valid).toBe(true);
      expect(result.finalCreated).toBe(true);

      // @step And a work unit "FOUND-001" should be created with title "Foundation Event Storm"
      const afterData = JSON.parse(
        await readFile(workUnitsPath, 'utf-8')
      ) as WorkUnitsData;
      const foundWorkUnit = afterData.workUnits['FOUND-001'];
      expect(foundWorkUnit).toBeDefined();
      expect(foundWorkUnit?.title).toContain('Foundation Event Storm');

      // @step And the work unit should have status "backlog"
      expect(foundWorkUnit?.status).toBe('backlog');

      // @step And the work unit description should mention Event Storm workflow
      expect(foundWorkUnit?.description).toContain('Event Storm');
      expect(foundWorkUnit?.description).toContain(
        'add-foundation-bounded-context'
      );

      // @step And the test MUST NOT write to the actual spec/work-units.json file
      // (verified by using isolated testDir, not process.cwd())
      expect(workUnitsPath).toContain('test-temp-');
      expect(workUnitsPath).not.toContain(
        join(process.cwd(), 'spec/work-units.json')
      );
    });
  });

  describe("Scenario: FOUND prefix is auto-registered if it doesn't exist", () => {
    it('should auto-register FOUND prefix with description "Foundation Event Storm tasks"', async () => {
      // @step Given I am in an isolated temporary test directory
      // (already set up in beforeEach)

      // @step And I have a completed foundation.json.draft file
      await writeFile(
        draftPath,
        JSON.stringify(completedDraft, null, 2),
        'utf-8'
      );

      // @step And the FOUND prefix does not exist in prefixes
      const beforeData = JSON.parse(
        await readFile(workUnitsPath, 'utf-8')
      ) as WorkUnitsData;
      expect(beforeData.prefixes['FOUND']).toBeUndefined();

      // @step When I run "fspec discover-foundation --finalize"
      const result = await discoverFoundation({
        finalize: true,
        draftPath,
        cwd: testDir,
      });

      // @step Then the FOUND prefix should be registered
      const afterPrefixesData = JSON.parse(
        await readFile(prefixesPath, 'utf-8')
      );
      expect(afterPrefixesData.prefixes['FOUND']).toBeDefined();

      // @step And the prefix description should be "Foundation Event Storm tasks"
      expect(afterPrefixesData.prefixes['FOUND']?.description).toContain(
        'Foundation Event Storm'
      );

      // @step And the test MUST NOT write to the actual spec/work-units.json file
      expect(workUnitsPath).toContain('test-temp-');
      expect(workUnitsPath).not.toContain(
        join(process.cwd(), 'spec/work-units.json')
      );
    });
  });

  describe('Scenario: Running finalize twice does NOT create duplicate work units (idempotency)', () => {
    it('should not create duplicate FOUND work units on second finalize', async () => {
      // @step Given I am in an isolated temporary test directory
      // (already set up in beforeEach)

      // @step And I have already run "fspec discover-foundation --finalize" once successfully
      await writeFile(
        draftPath,
        JSON.stringify(completedDraft, null, 2),
        'utf-8'
      );

      const firstResult = await discoverFoundation({
        finalize: true,
        draftPath,
        cwd: testDir,
      });
      expect(firstResult.valid).toBe(true);

      // @step And work unit "FOUND-001" already exists
      const afterFirstData = JSON.parse(
        await readFile(workUnitsPath, 'utf-8')
      ) as WorkUnitsData;
      expect(afterFirstData.workUnits['FOUND-001']).toBeDefined();

      // Recreate draft for second finalization
      await writeFile(
        draftPath,
        JSON.stringify(completedDraft, null, 2),
        'utf-8'
      );

      // @step When I run "fspec discover-foundation --finalize" again
      const secondResult = await discoverFoundation({
        finalize: true,
        draftPath,
        cwd: testDir,
      });

      // @step Then the command should succeed
      expect(secondResult.valid).toBe(true);

      // @step And NO new work unit should be created
      const afterSecondData = JSON.parse(
        await readFile(workUnitsPath, 'utf-8')
      ) as WorkUnitsData;

      // @step And FOUND-001 should still be the only FOUND work unit
      const foundWorkUnits = Object.keys(afterSecondData.workUnits).filter(id =>
        id.startsWith('FOUND-')
      );
      expect(foundWorkUnits).toHaveLength(1);
      expect(foundWorkUnits[0]).toBe('FOUND-001');

      // @step And the test MUST NOT write to the actual spec/work-units.json file
      expect(workUnitsPath).toContain('test-temp-');
      expect(workUnitsPath).not.toContain(
        join(process.cwd(), 'spec/work-units.json')
      );
    });
  });
});
