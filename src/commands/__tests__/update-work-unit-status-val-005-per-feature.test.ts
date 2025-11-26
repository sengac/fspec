/**
 * Feature: spec/features/val-005-per-feature-validation.feature
 *
 * BUG-093: VAL-005 1:1 validation checks across entire work unit instead of per-feature
 *
 * This test file validates that VAL-005 checks 1:1 mapping PER FEATURE,
 * not across all features in the work unit.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdtemp, rm, writeFile, mkdir } from 'fs/promises';
import { tmpdir } from 'os';
import { join } from 'path';
import { updateWorkUnitStatus } from '../update-work-unit-status';

describe('Feature: VAL-005 1:1 validation checks across entire work unit instead of per-feature', () => {
  let testDir: string;
  let specDir: string;
  let workUnitsFile: string;
  let featuresDir: string;

  beforeEach(async () => {
    testDir = await mkdtemp(join(tmpdir(), 'fspec-bug-093-'));
    specDir = join(testDir, 'spec');
    workUnitsFile = join(specDir, 'work-units.json');
    featuresDir = join(specDir, 'features');

    await mkdir(specDir, { recursive: true });
    await mkdir(featuresDir, { recursive: true });
    await mkdir(join(testDir, 'src/__tests__'), { recursive: true });

    await writeFile(
      workUnitsFile,
      JSON.stringify(
        {
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
  });

  afterEach(async () => {
    await rm(testDir, { recursive: true, force: true });
  });

  describe('Scenario: Single feature with single test file passes validation', () => {
    it('should pass validation when 1 feature has 1 test file', async () => {
      // @step Given a work unit with 1 feature file linked to 1 test file
      const featureContent = `@VTEST-001
Feature: Test Feature One

  Scenario: Test scenario
    Given a precondition
    When an action
    Then a result
`;
      await writeFile(join(featuresDir, 'feature-one.feature'), featureContent);

      const coverageContent = {
        scenarios: [
          {
            name: 'Test scenario',
            testMappings: [
              { file: 'src/__tests__/feature-one.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-one.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      const testContent = `// Feature: spec/features/feature-one.feature
describe('Test scenario', () => {
  it('test', () => {
    // @step Given a precondition
    // @step When an action
    // @step Then a result
  });
});
`;
      await writeFile(
        join(testDir, 'src/__tests__/feature-one.test.ts'),
        testContent
      );

      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'VTEST-001': {
                id: 'VTEST-001',
                type: 'story',
                title: 'Test Feature One',
                status: 'testing',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                stateHistory: [
                  {
                    state: 'testing',
                    timestamp: new Date(Date.now() - 1000).toISOString(),
                  },
                ],
              },
            },
            states: {
              backlog: [],
              specifying: [],
              testing: ['VTEST-001'],
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

      // @step When VAL-005 validation runs during status transition to implementing
      // @step Then the validation passes
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'VTEST-001',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).resolves.not.toThrow();
    });
  });

  describe('Scenario: Single feature with multiple test files fails validation', () => {
    it('should fail validation when 1 feature has 2 test files', async () => {
      // @step Given a work unit with 1 feature file linked to 2 test files
      const featureContent = `@VTEST-002
Feature: Test Feature Two

  Scenario: Scenario A
    Given step A
    When action A
    Then result A

  Scenario: Scenario B
    Given step B
    When action B
    Then result B
`;
      await writeFile(join(featuresDir, 'feature-two.feature'), featureContent);

      const coverageContent = {
        scenarios: [
          {
            name: 'Scenario A',
            testMappings: [
              { file: 'src/__tests__/test-a.test.ts', lines: '10-15' },
            ],
          },
          {
            name: 'Scenario B',
            testMappings: [
              { file: 'src/__tests__/test-b.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-two.feature.coverage'),
        JSON.stringify(coverageContent, null, 2)
      );

      const testContentA = `// Feature: spec/features/feature-two.feature
describe('Scenario A', () => {
  it('test', () => {
    // @step Given step A
    // @step When action A
    // @step Then result A
  });
});
`;
      await writeFile(
        join(testDir, 'src/__tests__/test-a.test.ts'),
        testContentA
      );

      const testContentB = `// Feature: spec/features/feature-two.feature
describe('Scenario B', () => {
  it('test', () => {
    // @step Given step B
    // @step When action B
    // @step Then result B
  });
});
`;
      await writeFile(
        join(testDir, 'src/__tests__/test-b.test.ts'),
        testContentB
      );

      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'VTEST-002': {
                id: 'VTEST-002',
                type: 'story',
                title: 'Test Feature Two',
                status: 'testing',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                stateHistory: [
                  {
                    state: 'testing',
                    timestamp: new Date(Date.now() - 1000).toISOString(),
                  },
                ],
              },
            },
            states: {
              backlog: [],
              specifying: [],
              testing: ['VTEST-002'],
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

      // @step When VAL-005 validation runs during status transition to implementing
      // @step Then the validation fails with error about multiple test files for single feature
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'VTEST-002',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/Multiple test files detected/);
    });
  });

  describe('Scenario: Multiple features each with single test file passes validation', () => {
    it('should pass validation when 3 features each have 1 test file (BUG-093 FIX)', async () => {
      // @step Given a work unit with 3 feature files each linked to exactly 1 test file

      // Feature 1
      const feature1 = `@VTEST-003
Feature: Feature Alpha

  Scenario: Alpha scenario
    Given alpha precondition
    When alpha action
    Then alpha result
`;
      await writeFile(join(featuresDir, 'feature-alpha.feature'), feature1);

      const coverage1 = {
        scenarios: [
          {
            name: 'Alpha scenario',
            testMappings: [
              { file: 'src/__tests__/alpha.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-alpha.feature.coverage'),
        JSON.stringify(coverage1, null, 2)
      );

      const test1 = `// Feature: spec/features/feature-alpha.feature
describe('Alpha scenario', () => {
  it('test', () => {
    // @step Given alpha precondition
    // @step When alpha action
    // @step Then alpha result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/alpha.test.ts'), test1);

      // Feature 2
      const feature2 = `@VTEST-003
Feature: Feature Beta

  Scenario: Beta scenario
    Given beta precondition
    When beta action
    Then beta result
`;
      await writeFile(join(featuresDir, 'feature-beta.feature'), feature2);

      const coverage2 = {
        scenarios: [
          {
            name: 'Beta scenario',
            testMappings: [
              { file: 'src/__tests__/beta.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-beta.feature.coverage'),
        JSON.stringify(coverage2, null, 2)
      );

      const test2 = `// Feature: spec/features/feature-beta.feature
describe('Beta scenario', () => {
  it('test', () => {
    // @step Given beta precondition
    // @step When beta action
    // @step Then beta result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/beta.test.ts'), test2);

      // Feature 3
      const feature3 = `@VTEST-003
Feature: Feature Gamma

  Scenario: Gamma scenario
    Given gamma precondition
    When gamma action
    Then gamma result
`;
      await writeFile(join(featuresDir, 'feature-gamma.feature'), feature3);

      const coverage3 = {
        scenarios: [
          {
            name: 'Gamma scenario',
            testMappings: [
              { file: 'src/__tests__/gamma.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-gamma.feature.coverage'),
        JSON.stringify(coverage3, null, 2)
      );

      const test3 = `// Feature: spec/features/feature-gamma.feature
describe('Gamma scenario', () => {
  it('test', () => {
    // @step Given gamma precondition
    // @step When gamma action
    // @step Then gamma result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/gamma.test.ts'), test3);

      // Work unit with all 3 features
      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'VTEST-003': {
                id: 'VTEST-003',
                type: 'story',
                title: 'Multi-Feature Work Unit',
                status: 'testing',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                stateHistory: [
                  {
                    state: 'testing',
                    timestamp: new Date(Date.now() - 1000).toISOString(),
                  },
                ],
              },
            },
            states: {
              backlog: [],
              specifying: [],
              testing: ['VTEST-003'],
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

      // @step When VAL-005 validation runs during status transition to implementing
      // @step Then the validation passes because each feature has exactly one test file
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'VTEST-003',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).resolves.not.toThrow();
    });
  });

  describe('Scenario: Multiple features where one has multiple test files fails validation', () => {
    it('should fail when one of 3 features has 2 test files', async () => {
      // @step Given a work unit with 3 feature files where one feature has 2 test files

      // Feature 1 - has 1 test file (OK)
      const feature1 = `@VTEST-004
Feature: Feature Delta

  Scenario: Delta scenario
    Given delta precondition
    When delta action
    Then delta result
`;
      await writeFile(join(featuresDir, 'feature-delta.feature'), feature1);

      const coverage1 = {
        scenarios: [
          {
            name: 'Delta scenario',
            testMappings: [
              { file: 'src/__tests__/delta.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-delta.feature.coverage'),
        JSON.stringify(coverage1, null, 2)
      );

      const test1 = `// Feature: spec/features/feature-delta.feature
describe('Delta scenario', () => {
  it('test', () => {
    // @step Given delta precondition
    // @step When delta action
    // @step Then delta result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/delta.test.ts'), test1);

      // Feature 2 - has 2 test files (BAD - should trigger error)
      const feature2 = `@VTEST-004
Feature: Feature Epsilon

  Scenario: Epsilon scenario 1
    Given epsilon1 precondition
    When epsilon1 action
    Then epsilon1 result

  Scenario: Epsilon scenario 2
    Given epsilon2 precondition
    When epsilon2 action
    Then epsilon2 result
`;
      await writeFile(join(featuresDir, 'feature-epsilon.feature'), feature2);

      const coverage2 = {
        scenarios: [
          {
            name: 'Epsilon scenario 1',
            testMappings: [
              { file: 'src/__tests__/epsilon1.test.ts', lines: '10-15' },
            ],
          },
          {
            name: 'Epsilon scenario 2',
            testMappings: [
              { file: 'src/__tests__/epsilon2.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-epsilon.feature.coverage'),
        JSON.stringify(coverage2, null, 2)
      );

      const test2a = `// Feature: spec/features/feature-epsilon.feature
describe('Epsilon scenario 1', () => {
  it('test', () => {
    // @step Given epsilon1 precondition
    // @step When epsilon1 action
    // @step Then epsilon1 result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/epsilon1.test.ts'), test2a);

      const test2b = `// Feature: spec/features/feature-epsilon.feature
describe('Epsilon scenario 2', () => {
  it('test', () => {
    // @step Given epsilon2 precondition
    // @step When epsilon2 action
    // @step Then epsilon2 result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/epsilon2.test.ts'), test2b);

      // Feature 3 - has 1 test file (OK)
      const feature3 = `@VTEST-004
Feature: Feature Zeta

  Scenario: Zeta scenario
    Given zeta precondition
    When zeta action
    Then zeta result
`;
      await writeFile(join(featuresDir, 'feature-zeta.feature'), feature3);

      const coverage3 = {
        scenarios: [
          {
            name: 'Zeta scenario',
            testMappings: [
              { file: 'src/__tests__/zeta.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-zeta.feature.coverage'),
        JSON.stringify(coverage3, null, 2)
      );

      const test3 = `// Feature: spec/features/feature-zeta.feature
describe('Zeta scenario', () => {
  it('test', () => {
    // @step Given zeta precondition
    // @step When zeta action
    // @step Then zeta result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/zeta.test.ts'), test3);

      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'VTEST-004': {
                id: 'VTEST-004',
                type: 'story',
                title: 'Multi-Feature With One Bad',
                status: 'testing',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                stateHistory: [
                  {
                    state: 'testing',
                    timestamp: new Date(Date.now() - 1000).toISOString(),
                  },
                ],
              },
            },
            states: {
              backlog: [],
              specifying: [],
              testing: ['VTEST-004'],
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

      // @step When VAL-005 validation runs during status transition to implementing
      // @step Then the validation fails identifying the specific feature with multiple test files
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'VTEST-004',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/Multiple test files detected/);

      // After fix, error should mention the specific feature file
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'VTEST-004',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/feature-epsilon\.feature/);
    });
  });

  describe('Scenario: Multiple features where one has no test files fails validation', () => {
    it('should fail when one of 3 features has 0 test files', async () => {
      // @step Given a work unit with 3 feature files where one feature has 0 test files

      // Feature 1 - has 1 test file (OK)
      const feature1 = `@VTEST-005
Feature: Feature Eta

  Scenario: Eta scenario
    Given eta precondition
    When eta action
    Then eta result
`;
      await writeFile(join(featuresDir, 'feature-eta.feature'), feature1);

      const coverage1 = {
        scenarios: [
          {
            name: 'Eta scenario',
            testMappings: [
              { file: 'src/__tests__/eta.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-eta.feature.coverage'),
        JSON.stringify(coverage1, null, 2)
      );

      const test1 = `// Feature: spec/features/feature-eta.feature
describe('Eta scenario', () => {
  it('test', () => {
    // @step Given eta precondition
    // @step When eta action
    // @step Then eta result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/eta.test.ts'), test1);

      // Feature 2 - has 0 test files (BAD - should trigger error)
      const feature2 = `@VTEST-005
Feature: Feature Theta

  Scenario: Theta scenario
    Given theta precondition
    When theta action
    Then theta result
`;
      await writeFile(join(featuresDir, 'feature-theta.feature'), feature2);

      // Coverage file with no test mappings
      const coverage2 = {
        scenarios: [
          {
            name: 'Theta scenario',
            testMappings: [],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-theta.feature.coverage'),
        JSON.stringify(coverage2, null, 2)
      );

      // Feature 3 - has 1 test file (OK)
      const feature3 = `@VTEST-005
Feature: Feature Iota

  Scenario: Iota scenario
    Given iota precondition
    When iota action
    Then iota result
`;
      await writeFile(join(featuresDir, 'feature-iota.feature'), feature3);

      const coverage3 = {
        scenarios: [
          {
            name: 'Iota scenario',
            testMappings: [
              { file: 'src/__tests__/iota.test.ts', lines: '10-15' },
            ],
          },
        ],
      };
      await writeFile(
        join(featuresDir, 'feature-iota.feature.coverage'),
        JSON.stringify(coverage3, null, 2)
      );

      const test3 = `// Feature: spec/features/feature-iota.feature
describe('Iota scenario', () => {
  it('test', () => {
    // @step Given iota precondition
    // @step When iota action
    // @step Then iota result
  });
});
`;
      await writeFile(join(testDir, 'src/__tests__/iota.test.ts'), test3);

      await writeFile(
        workUnitsFile,
        JSON.stringify(
          {
            workUnits: {
              'VTEST-005': {
                id: 'VTEST-005',
                type: 'story',
                title: 'Multi-Feature With One Missing',
                status: 'testing',
                createdAt: new Date().toISOString(),
                updatedAt: new Date().toISOString(),
                stateHistory: [
                  {
                    state: 'testing',
                    timestamp: new Date(Date.now() - 1000).toISOString(),
                  },
                ],
              },
            },
            states: {
              backlog: [],
              specifying: [],
              testing: ['VTEST-005'],
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

      // @step When VAL-005 validation runs during status transition to implementing
      // @step Then the validation fails identifying the specific feature with no test files
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'VTEST-005',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/No test files/);

      // After fix, error should mention the specific feature file
      await expect(
        updateWorkUnitStatus({
          workUnitId: 'VTEST-005',
          status: 'implementing',
          cwd: testDir,
          skipTemporalValidation: true,
        })
      ).rejects.toThrow(/feature-theta\.feature/);
    });
  });
});
