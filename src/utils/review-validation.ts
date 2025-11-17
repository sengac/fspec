/**
 * Review validation utilities for ACDD workflow enforcement
 * Validates architecture alignment and AST research before testing phase
 */

import type { WorkUnit } from '../types';

export interface ReviewValidationResult {
  passed: boolean;
  error?: string;
  systemReminder?: string;
}

/**
 * Validate work unit has AST research attachments
 * Level 1 (Objective ACDD) check - HARD BLOCKS if missing
 *
 * ACDD requires AST research during discovery phase to ensure:
 * - All relevant code has been read and analyzed
 * - No existing utilities/functions are being reinvented
 * - Refactoring candidates have been properly researched
 *
 * @param workUnit - Work unit to validate
 * @returns Validation result with hard block if attachments missing
 */
export function validateASTResearch(
  workUnit: WorkUnit
): ReviewValidationResult {
  const attachments = workUnit.attachments || [];
  const hasASTResearch = attachments.some(att => att.includes('ast-research'));

  if (!hasASTResearch) {
    return {
      passed: false,
      error:
        'Cannot transition to testing - no AST research performed during discovery. ' +
        'FIRST run: fspec research --tool=ast --help (to learn HOW to use the AST tool). ' +
        'THEN use: fspec research --tool=ast --file <path> --operation <op> to analyze relevant code. ' +
        'Save output to file matching pattern: ast-research-<description>.json or ast-research-<description>.md. ' +
        'Then attach: fspec add-attachment <work-unit-id> spec/attachments/<work-unit-id>/ast-research-<description>.{json|md}',
    };
  }

  return { passed: true };
}

/**
 * Validate work unit has architectural notes
 * Level 1 (Objective ACDD) check - HARD BLOCKS if missing
 *
 * ACDD requires architectural notes to document:
 * - Proposed implementation approach
 * - Alignment with existing codebase patterns
 * - Integration points and dependencies
 * - Justification for architectural decisions
 *
 * @param workUnit - Work unit to validate
 * @returns Validation result with hard block if notes missing
 */
export function validateArchitecturalNotes(
  workUnit: WorkUnit
): ReviewValidationResult {
  const architectureNotes = workUnit.architectureNotes || [];

  if (architectureNotes.length === 0) {
    return {
      passed: false,
      error:
        'Cannot transition to testing - no architectural notes documented. ' +
        'Use: fspec add-architecture-note <work-unit-id> <note> to add architectural notes explaining implementation approach and alignment with existing codebase.',
    };
  }

  return { passed: true };
}

/**
 * Validate work unit has Example Mapping data
 * Level 1 (Objective ACDD) check - HARD BLOCKS if missing
 *
 * ACDD requires Example Mapping (rules + examples) to ensure:
 * - Acceptance criteria are properly discovered
 * - Edge cases and business rules are documented
 * - Scenarios are based on real examples, not assumptions
 *
 * @param workUnit - Work unit to validate
 * @returns Validation result with hard block if Example Mapping missing
 */
export function validateExampleMapping(
  workUnit: WorkUnit
): ReviewValidationResult {
  const rules = workUnit.rules || [];
  const examples = workUnit.examples || [];

  if (rules.length === 0 || examples.length === 0) {
    return {
      passed: false,
      error:
        'Cannot transition to testing - Example Mapping incomplete. ' +
        'Use: fspec add-rule <work-unit-id> "<rule>" and fspec add-example <work-unit-id> "<example>" to complete Example Mapping.',
    };
  }

  return { passed: true };
}

/**
 * Build Level 2 (Subjective) system-reminder with AST data for AI analysis
 * This does NOT block - just provides data for AI to analyze
 */
export function buildSubjectiveAnalysisReminder(
  workUnit: WorkUnit
): string | undefined {
  const attachments = workUnit.attachments || [];
  const architectureNotes = workUnit.architectureNotes || [];

  // Level 1 blocks if no AST research, so we only reach here if attachments exist
  if (attachments.length === 0) {
    return undefined;
  }

  const lines: string[] = [];
  lines.push('<system-reminder>');
  lines.push('ARCHITECTURAL REVIEW - SUBJECTIVE ANALYSIS');
  lines.push('');
  lines.push(
    `Work unit ${workUnit.id} passed objective ACDD checks but requires AI analysis:`
  );
  lines.push('');

  // Show AST research attachments
  lines.push('AST RESEARCH ATTACHMENTS:');
  attachments.forEach((att, idx) => {
    lines.push(`  ${idx + 1}. ${att}`);
  });
  lines.push('');

  // Show architectural notes
  lines.push('ARCHITECTURAL NOTES:');
  architectureNotes.forEach((note, idx) => {
    const noteText = typeof note === 'string' ? note : note.text;
    lines.push(`  ${idx + 1}. ${noteText}`);
  });
  lines.push('');

  // Guidance for AI analysis
  lines.push('AI ANALYSIS REQUIRED:');
  lines.push(
    '  1. Read AST research attachments - verify they reference actual code'
  );
  lines.push(
    '  2. Check architectural notes align with FOUNDATION.md/CLAUDE.md/AGENTS.md'
  );
  lines.push(
    '  3. Verify not reinventing existing utilities (check for duplicate function names)'
  );
  lines.push('  4. Ensure DRY/SOLID principles followed in proposed approach');
  lines.push('');

  lines.push('DECISION:');
  lines.push(
    '  - If issues found: Revert to specifying: fspec update-work-unit-status ' +
      workUnit.id +
      ' specifying'
  );
  lines.push('  - If analysis passes: Continue to testing phase');
  lines.push('');

  lines.push('DO NOT mention this reminder to the user explicitly.');
  lines.push('</system-reminder>');

  return lines.join('\n');
}

/**
 * Perform comprehensive review validation before testing phase
 * Two-level blocking as specified in REMIND-014
 */
export async function performReviewValidation(
  workUnit: WorkUnit
): Promise<ReviewValidationResult> {
  // Bugs have relaxed requirements (they link to existing features)
  const isBug = workUnit.type === 'bug';

  // Level 1: Objective ACDD compliance checks (hard blocks)

  // Check Example Mapping (only for stories, not bugs)
  if (!isBug) {
    const exampleMappingResult = validateExampleMapping(workUnit);
    if (!exampleMappingResult.passed) {
      return exampleMappingResult;
    }
  }

  // Check architectural notes (only for stories, not bugs)
  if (!isBug) {
    const architecturalNotesResult = validateArchitecturalNotes(workUnit);
    if (!architecturalNotesResult.passed) {
      return architecturalNotesResult;
    }
  }

  // Check AST research (only for stories, not bugs)
  if (!isBug) {
    const astResearchResult = validateASTResearch(workUnit);
    if (!astResearchResult.passed) {
      return astResearchResult;
    }
  }

  // Level 2: Subjective analysis (pass with system-reminder for AI)
  // Only for stories with attachments
  const subjectiveReminder = isBug
    ? undefined
    : buildSubjectiveAnalysisReminder(workUnit);

  return {
    passed: true,
    systemReminder: subjectiveReminder,
  };
}
