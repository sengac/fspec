import { readFile } from 'fs/promises';
import chalk from 'chalk';
import type { Command } from 'commander';
import { join } from 'path';
import type { WorkUnitsData } from '../types';
import { showWorkUnit } from './show-work-unit';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';
import { getAgentConfig, formatAgentOutput } from '../utils/agentRuntimeConfig';

import { output } from '../utils/output';
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

/**
 * Build AI-driven deep code analysis system-reminder
 * Instructs AI to read implementation files, analyze code, and check FOUNDATION.md
 */
async function buildAIAnalysisReminder(
  workUnitId: string,
  workUnit: { type?: string; title: string },
  featureFile: string | null,
  coverageData: {
    scenarios?: Array<{
      name: string;
      testMappings?: Array<{
        file: string;
        implMappings?: Array<{
          file: string;
          lines: number[];
        }>;
      }>;
    }>;
  } | null,
  cwd: string,
  recommendations: Array<{
    recommendation: string;
    rationale: string;
    action: string;
  }>
): Promise<string> {
  const lines: string[] = [];

  // Include ACDD recommendations if present
  if (recommendations.length > 0) {
    lines.push('ACDD COMPLIANCE REVIEW');
    lines.push('');
    recommendations.forEach((rec, index) => {
      lines.push(`${index + 1}. **Recommendation:** ${rec.recommendation}`);
      lines.push(`   - **Rationale:** ${rec.rationale}`);
      lines.push(`   - **Action:** ${rec.action}`);
      lines.push('');
    });
  }

  lines.push('AI-DRIVEN DEEP CODE REVIEW');
  lines.push('');
  lines.push(`Work Unit: ${workUnitId} - ${workUnit.title}`);
  lines.push('');

  // Collect all implementation files from coverage
  const implFiles = new Set<string>();
  if (coverageData?.scenarios) {
    for (const scenario of coverageData.scenarios) {
      if (scenario.testMappings) {
        for (const mapping of scenario.testMappings) {
          if (mapping.implMappings) {
            for (const implMapping of mapping.implMappings) {
              implFiles.add(implMapping.file);
            }
          }
        }
      }
    }
  }

  // AST data gathering removed - now uses ast-grep NAPI
  // Implementation files will be analyzed through the astGrepSearch NAPI function if needed

  if (implFiles.size > 0) {
    lines.push('STEP 1: Read Implementation Files');
    lines.push('');
    lines.push(
      'Use the Read tool to examine the following implementation files:'
    );
    implFiles.forEach(file => {
      lines.push(`  - ${file}`);
    });
    lines.push('');
  }

  lines.push('STEP 2: Analyze Code for Quality Issues');
  lines.push('');
  lines.push(
    'Perform deep analysis to analyze the code you read. Look for bugs:'
  );
  lines.push('');
  lines.push('  • Bugs and Logic Errors:');
  lines.push('    - Off-by-one errors, null pointer exceptions');
  lines.push('    - Incorrect edge case handling');
  lines.push('    - Logic flaws in conditionals or loops');
  lines.push('');
  lines.push('  • Race Conditions:');
  lines.push('    - Async operations without proper locking');
  lines.push('    - File operations that could conflict');
  lines.push('    - Concurrent access to shared resources');
  lines.push('');
  lines.push('  • Anti-Patterns:');
  lines.push(
    '    - God functions (>100 lines, large functions that need refactoring)'
  );
  lines.push('    - duplicated code across multiple files');
  lines.push('    - Tight coupling between modules');
  lines.push('    - Magic numbers without constants');
  lines.push('');
  lines.push('  • Refactoring Opportunities:');
  lines.push('    - Similar code that could be extracted to shared utilities');
  lines.push('    - large functions that should be split');
  lines.push('    - Repeated validation logic that could be DRY');
  lines.push('');

  lines.push('STEP 3: Check FOUNDATION.md Alignment');
  lines.push('');
  lines.push(
    'Read FOUNDATION.md or CLAUDE.md and verify code follows project principles:'
  );
  lines.push('  - File size limits (e.g., keep files under 300 lines)');
  lines.push(
    '  - Architectural patterns (e.g., use gitoxide NAPI-RS bindings not child_process)'
  );
  lines.push('  - Coding standards (e.g., no any types, use ES6 imports)');
  lines.push('  - Project-specific conventions');
  lines.push('');

  lines.push('STEP 4: Report Findings');
  lines.push('');
  lines.push('After your analysis, report findings conversationally:');
  lines.push('  - List bugs found with file:line references');
  lines.push("  - Explain anti-patterns detected and why they're problematic");
  lines.push('  - Suggest specific refactoring with code examples if helpful');
  lines.push('  - Note FOUNDATION.md violations with exact principle violated');
  lines.push('');
  lines.push('Example:');
  lines.push(
    '  "I found a potential race condition in src/file-ops/save.ts:15-20.'
  );
  lines.push(
    '   Two async writeFile calls happen without synchronization, which could'
  );
  lines.push(
    '   corrupt the file if both execute simultaneously. Consider using a'
  );
  lines.push(
    '   file locking pattern or atomic writes as mentioned in FOUNDATION.md."'
  );
  lines.push('');

  lines.push(
    'NOTE: The static analysis above already caught basic issues (any types, etc.).'
  );
  lines.push(
    'Focus your analysis on deeper issues that require understanding context and logic.'
  );

  return lines.join('\n');
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

  const lines: string[] = [];
  const criticalIssues: CriticalIssue[] = [];
  const warnings: Warning[] = [];
  const recommendations: Recommendation[] = [];

  // Build review header
  lines.push(
    '================================================================================'
  );
  lines.push(`REVIEW: ${workUnitId} - ${workUnit.title}`);
  lines.push(
    '================================================================================'
  );
  lines.push('');

  // Step 2: Read Feature Files
  let featureFile: string | null = null;
  let featureContent: string | null = null;
  let gherkinDocument: Messages.GherkinDocument | null = null;

  if (
    workUnitDetails.linkedFeatures &&
    workUnitDetails.linkedFeatures.length > 0
  ) {
    featureFile = workUnitDetails.linkedFeatures[0].file;
    const featurePath = join(cwd, featureFile);
    featureContent = await readFile(featurePath, 'utf-8');

    // Parse Gherkin
    const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const matcher = new Gherkin.GherkinClassicTokenMatcher();
    const parser = new Gherkin.Parser(builder, matcher);

    try {
      gherkinDocument = parser.parse(featureContent);
    } catch {
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
    acddPassed.push(
      `Example Mapping completed (${workUnit.rules.length} rules, ${workUnit.examples?.length || 0} examples, ${workUnit.questions?.filter((q: { selected?: boolean }) => q.selected).length || 0} questions answered)`
    );
  } else if (workUnit.status !== 'backlog') {
    acddFailed.push('No Example Mapping data found (missing rules/examples)');
    recommendations.push({
      recommendation: 'Complete Example Mapping before specifying',
      rationale:
        'Example Mapping clarifies requirements and prevents building the wrong feature',
      action: `fspec add-rule ${workUnitId} "<rule>" and fspec add-example ${workUnitId} "<example>"`,
    });
  }

  // Check feature file creation during specifying phase
  if (featureFile && workUnit.stateHistory) {
    const specifyingEntry = workUnit.stateHistory.find(
      (h: { state: string }) => h.state === 'specifying'
    );
    if (specifyingEntry) {
      acddPassed.push('Feature file created during specifying phase');
    }
  } else if (
    workUnit.status !== 'backlog' &&
    workUnit.status !== 'specifying'
  ) {
    acddFailed.push('Feature file should be created during specifying phase');
  }

  // Check test coverage
  if (coverageData && coverageData.stats) {
    if (coverageData.stats.coveragePercent === 100) {
      acddPassed.push('All scenarios have test coverage (100%)');
    } else if (coverageData.stats.coveragePercent > 0) {
      acddFailed.push(
        `Incomplete test coverage (${coverageData.stats.coveragePercent}%)`
      );
      recommendations.push({
        recommendation: 'Add tests for uncovered scenarios',
        rationale: 'All acceptance criteria must have corresponding tests',
        action: `fspec show-coverage ${featureFile?.replace(/^spec\/features\//, '').replace(/\.feature$/, '')} to see uncovered scenarios`,
      });
    }
  }

  // Check temporal ordering
  if (workUnit.stateHistory && workUnit.stateHistory.length > 0) {
    acddPassed.push(
      `Temporal ordering verified (${workUnit.stateHistory.length} state transitions)`
    );
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
  lines.push('## Issues Found');
  lines.push('');
  lines.push('### 🔴 Critical Issues');
  if (criticalIssues.length > 0) {
    criticalIssues.forEach((issue, index) => {
      lines.push(`${index + 1}. **Issue:** ${issue.issue}`);
      if (issue.location) {
        lines.push(`   - **Location:** ${issue.location}`);
      }
      lines.push(`   - **Fix:** ${issue.fix}`);
      lines.push(`   - **Action:** ${issue.action}`);
      lines.push('');
    });
  } else {
    lines.push('No critical issues detected.');
    lines.push('');
  }

  lines.push('### 🟡 Warnings');
  if (warnings.length > 0) {
    warnings.forEach((warning, index) => {
      lines.push(`${index + 1}. **Issue:** ${warning.issue}`);
      if (warning.location) {
        lines.push(`   - **Location:** ${warning.location}`);
      }
      lines.push(`   - **Fix:** ${warning.fix}`);
      lines.push(`   - **Action:** ${warning.action}`);
      lines.push('');
    });
  } else {
    lines.push('No warnings detected.');
    lines.push('');
  }

  // Recommendations section (plain output, will be included in system-reminder later)
  if (recommendations.length > 0) {
    lines.push('## Recommendations');
    lines.push('');

    lines.push('**IMPORTANT:** ACDD COMPLIANCE REVIEW');
    lines.push('');

    recommendations.forEach((rec, index) => {
      lines.push(`${index + 1}. **Recommendation:** ${rec.recommendation}`);
      lines.push(`   - **Rationale:** ${rec.rationale}`);
      lines.push(`   - **Action:** ${rec.action}`);
      lines.push('');
    });
  }

  // ACDD Compliance section
  lines.push('## ACDD Compliance');
  lines.push('');
  if (acddPassed.length > 0) {
    lines.push('✅ **Passed:**');
    acddPassed.forEach(item => {
      lines.push(`- ${item}`);
    });
    lines.push('');
  }
  if (acddFailed.length > 0) {
    lines.push('❌ **Failed:**');
    acddFailed.forEach(item => {
      lines.push(`- ${item}`);
    });
    lines.push('');
  }

  // Coverage Analysis section
  lines.push('## Coverage Analysis');
  lines.push('');
  if (coverageData && coverageData.stats) {
    lines.push(`- **Total Scenarios:** ${coverageData.stats.totalScenarios}`);
    lines.push(
      `- **Covered Scenarios:** ${coverageData.stats.coveredScenarios} (${coverageData.stats.coveragePercent}%)`
    );

    if (coverageData.scenarios) {
      const uncovered = coverageData.scenarios
        .filter(s => !s.testMappings || s.testMappings.length === 0)
        .map(s => s.name);

      if (uncovered.length > 0) {
        lines.push('');
        lines.push('**Uncovered Scenarios:**');
        uncovered.forEach(name => {
          lines.push(`  - ${name}`);
        });
      }
    }
  } else {
    lines.push('- No coverage data available');
  }
  lines.push('');

  // Summary section
  lines.push('## Summary');
  lines.push('');

  let assessment = 'PASS';
  if (criticalIssues.length > 0) {
    assessment = 'CRITICAL ISSUES';
  } else if (warnings.length > 0 || acddFailed.length > 0) {
    assessment = 'NEEDS WORK';
  }

  lines.push(`**Overall Assessment:** ${assessment}`);
  lines.push('');

  // Show current state for in-progress work
  if (workUnit.status !== 'done') {
    lines.push(`**Current State:** ${workUnit.status}`);
    lines.push('');
  }

  lines.push('**Priority Actions:**');

  // Determine priority actions
  const priorityActions: string[] = [];
  if (criticalIssues.length > 0) {
    priorityActions.push(`Fix ${criticalIssues.length} critical issue(s)`);
  }
  if (acddFailed.length > 0) {
    priorityActions.push('Address ACDD compliance violations');
  }
  if (
    coverageData &&
    coverageData.stats &&
    coverageData.stats.coveragePercent < 100
  ) {
    priorityActions.push('Complete test coverage for all scenarios');
  }
  if (workUnit.status !== 'done') {
    priorityActions.push(`Continue ${workUnit.status} phase`);
  }

  if (priorityActions.length === 0) {
    priorityActions.push(
      'Work unit review complete - no critical actions needed'
    );
  }

  priorityActions.forEach((action, index) => {
    lines.push(`${index + 1}. ${action}`);
  });
  lines.push('');

  // Build AI-driven deep analysis system-reminder (includes ACDD recommendations)
  const systemReminder = await buildAIAnalysisReminder(
    workUnitId,
    workUnit,
    featureFile,
    coverageData,
    cwd,
    recommendations
  );

  lines.push(formatAgentOutput(agent, systemReminder));
  lines.push('');

  return {
    success: true,
    output: lines.join('\n'),
  };
}

// CLI command setup
export function registerReviewCommand(program: Command): void {
  program
    .command('review <work-unit-id>')
    .description(
      'Perform comprehensive review of work unit with ACDD compliance and quality checks'
    )
    .action(async (workUnitId: string) => {
      try {
        const result = await review(workUnitId);
        output.log(result.output);
      } catch (error) {
        if (error instanceof Error) {
          output.error('Error:', error.message);
          process.exit(1);
        }
        throw error;
      }
    });
}
