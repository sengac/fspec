/**
 * Feature: spec/features/auto-create-big-picture-event-storming-work-unit-after-foundation-finalization.feature
 *
 * Tests for auto-creating Big Picture Event Storming work unit after foundation finalization
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, readFile } from 'fs/promises';
import { join } from 'path';
import {
  createTempTestDir,
  removeTempTestDir,
} from '../../test-helpers/temp-directory';
import { discoverFoundation } from '../discover-foundation';

describe('Feature: Auto-create Big Picture Event Storming work unit after foundation finalization', () => {
  let testDir: string;

  beforeEach(async () => {
    testDir = await createTempTestDir('auto-event-storm');

    // Create minimal work-units.json with states arrays
    const workUnitsPath = join(testDir, 'spec', 'work-units.json');
    await writeFile(
      workUnitsPath,
      JSON.stringify(
        {
          version: '2.0.0',
          workUnits: {},
          states: {
            backlog: [],
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

    // Create prefixes.json with FOUND prefix
    const prefixesPath = join(testDir, 'spec', 'prefixes.json');
    await writeFile(
      prefixesPath,
      JSON.stringify(
        { prefixes: { FOUND: { description: 'Foundation' } } },
        null,
        2
      )
    );

    // Create foundation draft with complete data (no placeholders)
    const draftPath = join(testDir, 'spec', 'foundation.json.draft');
    await writeFile(
      draftPath,
      JSON.stringify(
        {
          version: '2.0.0',
          project: {
            name: 'Test Project',
            vision: 'Test vision',
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
            overview: 'Test solution',
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
        },
        null,
        2
      )
    );
  });

  afterEach(async () => {
    await removeTempTestDir(testDir);
  });

  describe('Scenario: Work unit created when foundation finalized successfully', () => {
    it('should create work unit with FOUND prefix and Event Storming guidance', async () => {
      // @step Given foundation discovery has been completed
      // @step And all required foundation fields are populated
      // (Preconditions met by beforeEach - draft file created with complete data)

      // @step When I run "fspec discover-foundation --finalize"
      // @step And validation passes
      const result = await discoverFoundation({
        finalize: true,
        draftPath: join(testDir, 'spec', 'foundation.json.draft'),
        cwd: testDir,
      });

      // @step Then a new work unit should be created with FOUND prefix
      const workUnitsPath = join(testDir, 'spec', 'work-units.json');
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnits = JSON.parse(workUnitsContent);

      const foundWorkUnits = Object.keys(workUnits.workUnits).filter(id =>
        id.startsWith('FOUND-')
      );
      expect(foundWorkUnits.length).toBeGreaterThan(0);

      const workUnitId = foundWorkUnits[0];
      const workUnit = workUnits.workUnits[workUnitId];

      // @step And the work unit status should be "backlog"
      expect(workUnit.status).toBe('backlog');

      // @step And the work unit type should be "task"
      expect(workUnit.type).toBe('task');

      // @step And the work unit title should contain "Foundation Event Storm"
      expect(workUnit.title).toContain('Foundation Event Storm');

      // @step And the work unit description should include foundation Event Storm commands
      expect(workUnit.description).toContain('add-foundation-bounded-context');
      expect(workUnit.description).toContain('add-aggregate-to-foundation');
      expect(workUnit.description).toContain('add-domain-event-to-foundation');
      expect(workUnit.description).toContain('show-foundation-event-storm');

      // @step And the work unit description should reference CLAUDE.md documentation
      expect(workUnit.description).toContain('CLAUDE.md');

      // @step And console output should confirm work unit creation
      expect(result.workUnitCreated).toBe(true);
      expect(result.workUnitId).toBeTruthy();
    });
  });

  describe('Scenario: Work unit NOT created without finalize flag', () => {
    it('should not create work unit when --finalize flag is omitted', async () => {
      // @step Given foundation discovery has been completed
      // @step And all required foundation fields are populated
      // (Preconditions met by beforeEach)

      // @step When I run "fspec discover-foundation" without --finalize flag
      await discoverFoundation({
        force: true,
        cwd: testDir,
      });

      // @step Then NO work unit should be created
      const workUnitsPath = join(testDir, 'spec', 'work-units.json');
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnits = JSON.parse(workUnitsContent);

      const foundWorkUnits = Object.keys(workUnits.workUnits).filter(id =>
        id.startsWith('FOUND-')
      );

      expect(foundWorkUnits.length).toBe(0);

      // @step And work-units.json should remain unchanged
      expect(workUnits.workUnits).toEqual({});
    });
  });

  describe('Scenario: Work unit NOT created when validation fails', () => {
    it('should not create work unit when foundation validation fails', async () => {
      // @step Given foundation discovery has been completed
      // @step And foundation draft has validation errors
      const draftPath = join(testDir, 'spec', 'foundation.json.draft');
      await writeFile(
        draftPath,
        JSON.stringify(
          {
            version: '2.0.0',
            project: {
              name: 'Test Project',
              // Missing required fields: vision, projectType
            },
            // Missing required sections: problemSpace, solutionSpace, personas
          },
          null,
          2
        )
      );

      // @step When I run "fspec discover-foundation --finalize"
      // @step And validation fails
      let threwError = false;
      try {
        await discoverFoundation({
          finalize: true,
          draftPath: 'spec/foundation.json.draft',
          cwd: testDir,
        });
      } catch {
        threwError = true;
      }

      expect(threwError).toBe(true);

      // @step Then NO work unit should be created
      const workUnitsPath = join(testDir, 'spec', 'work-units.json');
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnits = JSON.parse(workUnitsContent);

      const foundWorkUnits = Object.keys(workUnits.workUnits).filter(id =>
        id.startsWith('FOUND-')
      );
      expect(foundWorkUnits.length).toBe(0);

      // @step And work-units.json should remain unchanged
      expect(workUnits.workUnits).toEqual({});

      // @step And error message should explain validation failure
      // (Error message checked by exit code 1)
    });
  });

  describe('Scenario: Work unit description contains Event Storming guidance', () => {
    it('should include all foundation Event Storm commands in description', async () => {
      // @step Given foundation has been finalized successfully
      // @step And a Big Picture Event Storming work unit was created
      await discoverFoundation({
        finalize: true,
        draftPath: join(testDir, 'spec', 'foundation.json.draft'),
        cwd: testDir,
      });

      // @step When AI agent reads the work unit description
      const workUnitsPath = join(testDir, 'spec', 'work-units.json');
      const workUnitsContent = await readFile(workUnitsPath, 'utf-8');
      const workUnits = JSON.parse(workUnitsContent);

      const foundWorkUnits = Object.keys(workUnits.workUnits).filter(id =>
        id.startsWith('FOUND-')
      );
      const workUnit = workUnits.workUnits[foundWorkUnits[0]];
      const description = workUnit.description;

      // @step Then the description should list "add-foundation-bounded-context" command
      expect(description).toContain('add-foundation-bounded-context');

      // @step And the description should list "add-aggregate-to-foundation" command
      expect(description).toContain('add-aggregate-to-foundation');

      // @step And the description should list "add-domain-event-to-foundation" command
      expect(description).toContain('add-domain-event-to-foundation');

      // @step And the description should list "show-foundation-event-storm" command
      expect(description).toContain('show-foundation-event-storm');

      // @step And the description should explain why Big Picture Event Storming matters
      expect(description.toLowerCase()).toContain('bounded context');

      // @step And the description should reference "spec/CLAUDE.md" for detailed guidance
      expect(description).toContain('CLAUDE.md');
    });
  });
});
