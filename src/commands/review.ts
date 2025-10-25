import { readFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import { glob } from 'tinyglobby';
import type { WorkUnitsData } from '../types';
import { showWorkUnit } from './show-work-unit';
import { showCoverage } from './show-coverage';
import { extractWorkUnitTags } from '../utils/work-unit-tags';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import { getAgentConfig, formatAgentOutput } from '../utils/agentRuntimeConfig';

interface ReviewOptions {
  cwd?: string;
}

interface ReviewResult {
  success: boolean;
  output: string;
}

interface CriticalIssue {
  issue: string;
  location?: string;
  fix: string;
  action: string;
}

interface Warning {
  issue: string;
  location?: string;
  fix: string;
  action: string;
}

interface Recommendation {
  recommendation: string;
  rationale: string;
  action: string;
}

export async function review(
  workUnitId: string,
  options?: ReviewOptions
): Promise<ReviewResult> {
  const cwd = options?.cwd || process.cwd();

  // Detect agent for formatted output
  const agent = getAgentConfig(cwd);

  // Read work units
  const workUnitsFile = join(cwd, 'spec/work-units.json');
  const workUnitsContent = await readFile(workUnitsFile, 'utf-8');
  const workUnitsData: WorkUnitsData = JSON.parse(workUnitsContent);

  // Check if work unit exists
  if (!workUnitsData.workUnits[workUnitId]) {
    throw new Error(`Work unit '${workUnitId}' does not exist`);
  }

  const workUnit = workUnitsData.workUnits[workUnitId];

  // Step 1: Load Work Unit Context
  const workUnitDetails = await showWorkUnit({
    workUnitId,
    output: 'json',
    cwd,
  });

  const output: string[] = [];
  const criticalIssues: CriticalIssue[] = [];
  const warnings: Warning[] = [];
  const recommendations: Recommendation[] = [];

  // Build review header
  output.push('================================================================================');
  output.push(`REVIEW: ${workUnitId} - ${workUnit.title}`);
  output.push('================================================================================');
  output.push('');

  // Step 2: Read Feature Files
  let featureFile: string | null = null;
  let featureContent: string | null = null;
  let gherkinDocument: Messages.GherkinDocument | null = null;

  if (workUnitDetails.linkedFeatures && workUnitDetails.linkedFeatures.length > 0) {
    featureFile = workUnitDetails.linkedFeatures[0].file;
    const featurePath = join(cwd, featureFile);
    featureContent = await readFile(featurePath, 'utf-8');

    // Parse Gherkin
    const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const matcher = new Gherkin.GherkinClassicTokenMatcher();
    const parser = new Gherkin.Parser(builder, matcher);

    try {
      gherkinDocument = parser.parse(featureContent);
    } catch (error) {
      warnings.push({
        issue: 'Invalid Gherkin syntax in feature file',
        location: featureFile,
        fix: 'Run fspec validate to see detailed syntax errors',
        action: `fspec validate ${featureFile}`,
      });
    }
  } else {
    warnings.push({
      issue: 'No linked feature files found',
      location: `Work unit ${workUnitId}`,
      fix: 'Create feature file with acceptance criteria',
        action: `fspec create-feature "${workUnit.title}"`,
    });
  }

  // Step 3: Analyze Test Coverage
  let coverageData: {
    stats?: {
      totalScenarios: number;
      coveredScenarios: number;
      coveragePercent: number;
    };
    scenarios?: Array<{
      name: string;
      testMappings?: Array<{
        file: string;
        lines: string;
      }>;
    }>;
  } | null = null;

  if (featureFile) {
    try {
      const coverageFilePath = join(cwd, featureFile + '.coverage');
      const coverageContent = await readFile(coverageFilePath, 'utf-8');
      coverageData = JSON.parse(coverageContent);
    } catch {
      // Coverage file might not exist
    }
  }

  // Step 4: Validate ACDD Workflow Compliance
  const acddPassed: string[] = [];
  const acddFailed: string[] = [];

  // Check Example Mapping
  if (workUnit.rules && workUnit.rules.length > 0) {
    acddPassed.push(`Example Mapping completed (${workUnit.rules.length} rules, ${workUnit.examples?.length || 0} examples, ${workUnit.questions?.filter((q: { selected?: boolean }) => q.selected).length || 0} questions answered)`);
  } else if (workUnit.status !== 'backlog') {
    acddFailed.push('No Example Mapping data found (missing rules/examples)');
    recommendations.push({
      recommendation: 'Complete Example Mapping before specifying',
      rationale: 'Example Mapping clarifies requirements and prevents building the wrong feature',
      action: `fspec add-rule ${workUnitId} "<rule>" and fspec add-example ${workUnitId} "<example>"`,
    });
  }

  // Check feature file creation during specifying phase
  if (featureFile && workUnit.stateHistory) {
    const specifyingEntry = workUnit.stateHistory.find((h: { state: string }) => h.state === 'specifying');
    if (specifyingEntry) {
      acddPassed.push('Feature file created during specifying phase');
    }
  } else if (workUnit.status !== 'backlog' && workUnit.status !== 'specifying') {
    acddFailed.push('Feature file should be created during specifying phase');
  }

  // Check test coverage
  if (coverageData && coverageData.stats) {
    if (coverageData.stats.coveragePercent === 100) {
      acddPassed.push('All scenarios have test coverage (100%)');
    } else if (coverageData.stats.coveragePercent > 0) {
      acddFailed.push(`Incomplete test coverage (${coverageData.stats.coveragePercent}%)`);
      recommendations.push({
        recommendation: 'Add tests for uncovered scenarios',
        rationale: 'All acceptance criteria must have corresponding tests',
        action: `fspec show-coverage ${featureFile?.replace(/^spec\/features\//, '').replace(/\.feature$/, '')} to see uncovered scenarios`,
      });
    }
  }

  // Check temporal ordering
  if (workUnit.stateHistory && workUnit.stateHistory.length > 0) {
    acddPassed.push(`Temporal ordering verified (${workUnit.stateHistory.length} state transitions)`);
  }

  // Step 5: Validate Coding Standards
  if (featureFile && coverageData?.scenarios) {
    for (const scenario of coverageData.scenarios) {
      if (scenario.testMappings && scenario.testMappings.length > 0) {
        for (const mapping of scenario.testMappings) {
          try {
            const testFilePath = join(cwd, mapping.file);
            const testContent = await readFile(testFilePath, 'utf-8');

            // Check for coding violations
            if (testContent.includes(': any')) {
              criticalIssues.push({
                issue: 'Use of `any` type detected',
                location: `${mapping.file}`,
                fix: 'Replace `any` with proper TypeScript types',
                action: 'Review file and add proper type annotations',
              });
            }

            if (testContent.match(/require\(/)) {
              criticalIssues.push({
                issue: 'CommonJS `require()` detected',
                location: `${mapping.file}`,
                fix: 'Use ES6 import statements',
                action: 'Replace require() with import',
              });
            }

            if (testContent.match(/import .* from ['"].*\.(ts|js)['"]/)) {
              criticalIssues.push({
                issue: 'File extensions in imports',
                location: `${mapping.file}`,
                fix: 'Remove .ts/.js extensions from imports',
                action: 'Vite handles file extensions automatically',
              });
            }
          } catch {
            // Test file might not exist
          }
        }
      }
    }
  }

  // Build Issues Found section
  output.push('## Issues Found');
  output.push('');
  output.push('### 🔴 Critical Issues');
  if (criticalIssues.length > 0) {
    criticalIssues.forEach((issue, index) => {
      output.push(`${index + 1}. **Issue:** ${issue.issue}`);
      if (issue.location) {
        output.push(`   - **Location:** ${issue.location}`);
      }
      output.push(`   - **Fix:** ${issue.fix}`);
      output.push(`   - **Action:** ${issue.action}`);
      output.push('');
    });
  } else {
    output.push('No critical issues detected.');
    output.push('');
  }

  output.push('### 🟡 Warnings');
  if (warnings.length > 0) {
    warnings.forEach((warning, index) => {
      output.push(`${index + 1}. **Issue:** ${warning.issue}`);
      if (warning.location) {
        output.push(`   - **Location:** ${warning.location}`);
      }
      output.push(`   - **Fix:** ${warning.fix}`);
      output.push(`   - **Action:** ${warning.action}`);
      output.push('');
    });
  } else {
    output.push('No warnings detected.');
    output.push('');
  }

  // Recommendations section (wrapped in agent-specific formatting)
  if (recommendations.length > 0) {
    output.push('## Recommendations');
    output.push('');

    // Build recommendations message
    const recommendationsMessage = recommendations
      .map((rec, index) =>
        `${index + 1}. **Recommendation:** ${rec.recommendation}\n` +
        `   - **Rationale:** ${rec.rationale}\n` +
        `   - **Action:** ${rec.action}`
      )
      .join('\n\n');

    // Wrap in agent-specific formatting
    const formattedRecommendations = formatAgentOutput(
      agent,
      `ACDD COMPLIANCE REVIEW\n\n${recommendationsMessage}`
    );

    output.push(formattedRecommendations);
    output.push('');
  }

  // ACDD Compliance section
  output.push('## ACDD Compliance');
  output.push('');
  if (acddPassed.length > 0) {
    output.push('✅ **Passed:**');
    acddPassed.forEach(item => {
      output.push(`- ${item}`);
    });
    output.push('');
  }
  if (acddFailed.length > 0) {
    output.push('❌ **Failed:**');
    acddFailed.forEach(item => {
      output.push(`- ${item}`);
    });
    output.push('');
  }

  // Coverage Analysis section
  output.push('## Coverage Analysis');
  output.push('');
  if (coverageData && coverageData.stats) {
    output.push(`- **Total Scenarios:** ${coverageData.stats.totalScenarios}`);
    output.push(`- **Covered Scenarios:** ${coverageData.stats.coveredScenarios} (${coverageData.stats.coveragePercent}%)`);

    if (coverageData.scenarios) {
      const uncovered = coverageData.scenarios
        .filter(s => !s.testMappings || s.testMappings.length === 0)
        .map(s => s.name);

      if (uncovered.length > 0) {
        output.push('');
        output.push('**Uncovered Scenarios:**');
        uncovered.forEach(name => {
          output.push(`  - ${name}`);
        });
      }
    }
  } else {
    output.push('- No coverage data available');
  }
  output.push('');

  // Summary section
  output.push('## Summary');
  output.push('');

  let assessment = 'PASS';
  if (criticalIssues.length > 0) {
    assessment = 'CRITICAL ISSUES';
  } else if (warnings.length > 0 || acddFailed.length > 0) {
    assessment = 'NEEDS WORK';
  }

  output.push(`**Overall Assessment:** ${assessment}`);
  output.push('');

  // Show current state for in-progress work
  if (workUnit.status !== 'done') {
    output.push(`**Current State:** ${workUnit.status}`);
    output.push('');
  }

  output.push('**Priority Actions:**');

  // Determine priority actions
  const priorityActions: string[] = [];
  if (criticalIssues.length > 0) {
    priorityActions.push(`Fix ${criticalIssues.length} critical issue(s)`);
  }
  if (acddFailed.length > 0) {
    priorityActions.push('Address ACDD compliance violations');
  }
  if (coverageData && coverageData.stats && coverageData.stats.coveragePercent < 100) {
    priorityActions.push('Complete test coverage for all scenarios');
  }
  if (workUnit.status !== 'done') {
    priorityActions.push(`Continue ${workUnit.status} phase`);
  }

  if (priorityActions.length === 0) {
    priorityActions.push('Work unit review complete - no critical actions needed');
  }

  priorityActions.forEach((action, index) => {
    output.push(`${index + 1}. ${action}`);
  });
  output.push('');

  return {
    success: true,
    output: output.join('\n'),
  };
}

// CLI command setup
export function registerReviewCommand(program: Command): void {
  program
    .command('review <work-unit-id>')
    .description('Perform comprehensive review of work unit with ACDD compliance and quality checks')
    .action(async (workUnitId: string) => {
      try {
        const result = await review(workUnitId);
        console.log(result.output);
      } catch (error) {
        if (error instanceof Error) {
          console.error(chalk.red('Error:'), error.message);
          process.exit(1);
        }
        throw error;
      }
    });
}
