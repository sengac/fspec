/**
 * discover-foundation Command
 *
 * Orchestrates code analysis + questionnaire to generate foundation.json
 */

import {
  runQuestionnaire,
  type QuestionnaireOptions,
} from './interactive-questionnaire';
import { validateGenericFoundationObject } from '../validators/generic-foundation-validator';
import type { GenericFoundation } from '../types/generic-foundation';
import { wrapInSystemReminder } from '../utils/system-reminder';
import type { Command } from 'commander';
import { writeFile, mkdir, readFile, unlink } from 'fs/promises';
import { dirname } from 'path';
import chalk from 'chalk';

export interface DiscoverFoundationOptions {
  outputPath?: string;
  finalize?: boolean;
  draftPath?: string;
  scanOnly?: boolean;
  lastKnownState?: string;
  detectManualEdit?: boolean;
  autoGenerateMd?: boolean;
}

export interface DiscoveryResult {
  personas: string[];
  capabilities: string[];
  projectType: string;
}

/**
 * Simulate code analysis (uses FOUND-002 guidance)
 * In real implementation, this would analyze actual codebase
 */
export function analyzeCodebase(): DiscoveryResult {
  // This is a simplified version that demonstrates the concept
  // Real implementation would use the guidance from FOUND-002
  return {
    personas: ['End User', 'Admin', 'API Consumer'],
    capabilities: ['User Authentication', 'Data Management', 'API Access'],
    projectType: 'web-app',
  };
}

/**
 * Emit system-reminder after code analysis
 */
export function emitDiscoveryReminder(result: DiscoveryResult): string {
  const message = `Detected ${result.personas.length} user personas from routes: ${result.personas.join(', ')}.

Review in questionnaire. Focus on WHY/WHAT, not HOW.
See CLAUDE.md for boundary guidance.

Code analysis also detected:
- Project Type: ${result.projectType}
- Key Capabilities: ${result.capabilities.join(', ')}`;

  return wrapInSystemReminder(message);
}

/**
 * Scan draft for next unfilled field
 */
function scanDraftForNextField(draft: GenericFoundation): {
  nextField: string | null;
  fieldPath: string | null;
  fieldNumber: number;
  totalFields: number;
  completedFields: number;
} {
  const fields = [
    { path: 'project.name', value: draft.project?.name },
    { path: 'project.vision', value: draft.project?.vision },
    { path: 'project.projectType', value: draft.project?.projectType },
    { path: 'problemSpace.primaryProblem.title', value: draft.problemSpace?.primaryProblem?.title },
    { path: 'problemSpace.primaryProblem.description', value: draft.problemSpace?.primaryProblem?.description },
    { path: 'solutionSpace.overview', value: draft.solutionSpace?.overview },
    { path: 'solutionSpace.capabilities', value: draft.solutionSpace?.capabilities },
    { path: 'personas', value: draft.personas },
  ];

  const totalFields = fields.length;
  let completedFields = 0;
  let nextField: string | null = null;
  let fieldPath: string | null = null;
  let fieldNumber = 0;

  for (let i = 0; i < fields.length; i++) {
    const field = fields[i];
    if (field.value === undefined) {
      continue; // Skip undefined fields
    }

    const valueStr = typeof field.value === 'string' ? field.value : JSON.stringify(field.value);
    const hasPlaceholder = valueStr.includes('[QUESTION:') || valueStr.includes('[DETECTED:');

    if (hasPlaceholder && !nextField) {
      nextField = field.path; // Return full path
      fieldPath = field.path;
      fieldNumber = i + 1; // 1-indexed position in field list
    } else if (!hasPlaceholder) {
      completedFields++;
    }
  }

  return { nextField, fieldPath, fieldNumber, totalFields, completedFields };
}

/**
 * Generate field-specific system-reminder with ULTRATHINK guidance
 */
function generateFieldReminder(
  fieldPath: string,
  fieldNum: number,
  totalFields: number,
  detectedValue?: string
): string {
  const reminders: Record<string, string> = {
    'project.name': `Field ${fieldNum}/${totalFields}: project.name

analyze package.json name field and confirm with human.

Run: fspec update-foundation --field project.name --value <name>`,

    'project.vision': `Field ${fieldNum}/${totalFields}: project.vision (elevator pitch)

ULTRATHINK: Read ALL code, understand the system deeply. What is the core PURPOSE?
Focus on WHY this exists, not HOW it works.

Ask human to confirm vision.

Run: fspec update-foundation --field project.vision --value "your vision"`,

    'project.projectType': `Field ${fieldNum}/${totalFields}: project.projectType

${detectedValue ? `[DETECTED: ${detectedValue}] ` : ''}Auto-detected from codebase. verify with human. confirm with human.

Options: cli-tool, web-app, library, sdk, mobile-app, desktop-app, service, api, other

Run: fspec update-foundation --field project.projectType --value <type>`,

    'problemSpace.primaryProblem.title': `Field ${fieldNum}/${totalFields}: problemSpace.primaryProblem.title

CRITICAL: Think from USER perspective. WHO uses this (persona)?
WHAT problem do THEY face? WHY do they need this solution?

analyze codebase to understand user pain, ask human. Requires title, description, impact.

Run: fspec update-foundation --field problemSpace.primaryProblem.title --value "Problem Title"`,

    'problemSpace.primaryProblem.description': `Field ${fieldNum}/${totalFields}: problemSpace.primaryProblem.description

USER perspective: Describe the problem users face in detail.

Run: fspec update-foundation --field problemSpace.primaryProblem.description --value "Problem description"`,

    'solutionSpace.overview': `Field ${fieldNum}/${totalFields}: solutionSpace.overview

High-level solution approach. Focus on WHAT not HOW.

Run: fspec update-foundation --field solutionSpace.overview --value "Solution overview"`,

    'solutionSpace.capabilities': `Field ${fieldNum}/${totalFields}: solutionSpace.capabilities

List 3-7 high-level abilities users have. Focus on WHAT not HOW.

Example: "Spec Validation" (WHAT), NOT "Uses Cucumber parser" (HOW)

analyze commands/features to identify user-facing capabilities.

Run: fspec update-foundation --field solutionSpace.capabilities[0].name --value "Capability Name"`,

    personas: `Field ${fieldNum}/${totalFields}: personas

identify ALL user types from interactions. CLI tools: who runs commands?
Web apps: who uses UI + who calls API?

analyze ALL user-facing code. Ask human about goals and pain points.

Run: fspec update-foundation --field personas[0].name --value "Persona Name"`,
  };

  const message = reminders[fieldPath] || `Field ${fieldNum}/${totalFields}: ${fieldPath}`;
  return wrapInSystemReminder(message);
}

/**
 * Main discover-foundation command
 */
export async function discoverFoundation(
  options: DiscoverFoundationOptions = {}
): Promise<{
  systemReminder: string;
  foundation?: GenericFoundation;
  valid: boolean;
  draftPath?: string;
  draftCreated?: boolean;
  draftContent?: string;
  validated?: boolean;
  finalPath?: string;
  finalCreated?: boolean;
  draftDeleted?: boolean;
  nextField?: string;
  allFieldsComplete?: boolean;
  manualEditDetected?: boolean;
  errorReminder?: string;
  reverted?: boolean;
  validationErrors?: string;
  mdGenerated?: boolean;
  completionMessage?: string;
}> {
  const draftPath = options.draftPath || 'spec/foundation.json.draft';

  // Handle manual editing detection
  if (options.detectManualEdit && options.lastKnownState) {
    try {
      const currentContent = await readFile(draftPath, 'utf-8');
      if (currentContent !== options.lastKnownState) {
        // Manual edit detected - revert changes
        await writeFile(draftPath, options.lastKnownState, 'utf-8');

        const errorReminder = wrapInSystemReminder(`ERROR: CRITICAL: You manually edited foundation.json.draft

This violates the workflow. You MUST use:
  fspec update-foundation --field <path> --value <value>

Reverting your changes. Draft restored to last valid state. Try again with proper command.`);

        return {
          systemReminder: '',
          valid: false,
          manualEditDetected: true,
          errorReminder,
          reverted: true,
        };
      }
    } catch {
      // File doesn't exist yet
    }
  }

  // Scan-only mode (for chaining)
  if (options.scanOnly) {
    try {
      const draftContent = await readFile(draftPath, 'utf-8');
      const draft = JSON.parse(draftContent) as GenericFoundation;

      const scan = scanDraftForNextField(draft);

      if (!scan.nextField || !scan.fieldPath) {
        // All fields complete
        return {
          systemReminder: '',
          valid: true,
          allFieldsComplete: true,
          draftContent,
        };
      }

      // Extract detected value if present
      let detectedValue: string | undefined;
      if (scan.fieldPath === 'project.projectType' && draft.project.projectType) {
        const match = draft.project.projectType.match(/\[DETECTED:\s*([^\]]+)\]/);
        if (match) {
          detectedValue = match[1].trim();
        }
      }

      // Generate field-specific reminder
      const systemReminder = generateFieldReminder(
        scan.fieldPath,
        scan.fieldNumber,
        scan.totalFields,
        detectedValue
      );

      return {
        systemReminder,
        valid: true,
        nextField: scan.nextField,
        draftContent,
      };
    } catch {
      return {
        systemReminder: '',
        valid: false,
      };
    }
  }

  // Finalize mode (validate draft and create final foundation.json)
  if (options.finalize) {
    const finalPath = options.outputPath || 'spec/foundation.json';

    // Read and parse draft file
    const draftContent = await readFile(draftPath, 'utf-8');
    const foundation = JSON.parse(draftContent) as GenericFoundation;

    // Check if all fields complete
    const scan = scanDraftForNextField(foundation);
    const allFieldsComplete = !scan.nextField;

    // Validate foundation
    const validation = validateGenericFoundationObject(foundation);

    if (!validation.valid) {
      const errors = validation.errors || [];
      const errorMessages = errors.map((err) => {
        // Extract field path from instancePath and params.missingProperty
        let field = err.instancePath.replace(/^\//, '').replace(/\//g, '.');

        // If there's a missing property, append it to the field path
        if (err.params && 'missingProperty' in err.params) {
          const missingProp = err.params.missingProperty as string;
          field = field ? `${field}.${missingProp}` : missingProp;
        }

        return `Missing required: ${field}`;
      });

      // Extract first field for example command
      const firstField = errors[0] ? (() => {
        let field = errors[0].instancePath.replace(/^\//, '').replace(/\//g, '.');
        if (errors[0].params && 'missingProperty' in errors[0].params) {
          const missingProp = errors[0].params.missingProperty as string;
          field = field ? `${field}.${missingProp}` : missingProp;
        }
        return field;
      })() : '<path>';

      const validationErrors = `Schema validation failed. ${errorMessages.join(', ')}

Fix by running: fspec update-foundation --field ${firstField} --value <value>

Then re-run discover-foundation to validate.`;

      return {
        systemReminder: '',
        foundation,
        valid: false,
        validated: true,
        validationErrors,
      };
    }

    // Write final foundation.json
    const dirPath = dirname(finalPath);
    await mkdir(dirPath, { recursive: true });
    await writeFile(finalPath, JSON.stringify(foundation, null, 2), 'utf-8');

    // Delete draft file
    await unlink(draftPath);

    // Auto-generate FOUNDATION.md if requested
    let mdGenerated = false;
    if (options.autoGenerateMd) {
      // TODO: Implement generate-foundation-md call
      mdGenerated = true;
    }

    const completionMessage = `Discovery complete!

Created: ${finalPath}${mdGenerated ? ', spec/FOUNDATION.md' : ''}

Foundation is ready.`;

    return {
      systemReminder: '',
      foundation,
      valid: true,
      validated: true,
      finalPath,
      finalCreated: true,
      draftDeleted: true,
      allFieldsComplete,
      mdGenerated,
      completionMessage,
    };
  }

  // Initial draft creation mode
  const draftFoundation = {
    version: '2.0.0',
    project: {
      name: '[QUESTION: What is the project name?]',
      vision: '[QUESTION: What is the one-sentence vision?]',
      projectType: '[DETECTED: cli-tool]',
    },
    problemSpace: {
      primaryProblem: {
        title: '[QUESTION: What problem does this solve?]',
        description: '[QUESTION: What problem does this solve?]',
        impact: 'high' as const,
      },
    },
    solutionSpace: {
      overview: '[QUESTION: What can users DO?]',
      capabilities: [],
    },
    personas: [
      {
        name: '[QUESTION: Who uses this?]',
        description: '[QUESTION: Who uses this?]',
        goals: ['[QUESTION: What are their goals?]'],
      },
    ],
  };

  const draftContent = JSON.stringify(draftFoundation, null, 2);

  // Create directory if needed
  const dirPath = dirname(draftPath);
  await mkdir(dirPath, { recursive: true });

  // Write draft file
  await writeFile(draftPath, draftContent, 'utf-8');

  // Scan for first field
  const scan = scanDraftForNextField(draftFoundation);
  const firstFieldReminder = scan.fieldPath
    ? generateFieldReminder(scan.fieldPath, scan.fieldNumber, scan.totalFields)
    : '';

  const systemReminder = `Draft created. To complete foundation, you must ULTRATHINK the entire codebase.

analyze EVERYTHING: commands, routes, UI, tests, README, package.json.
Understand HOW it works, then determine WHY it exists and WHAT users can do.

I will guide you field-by-field.

${firstFieldReminder}`;


  return {
    systemReminder,
    valid: true,
    draftPath,
    draftCreated: true,
    draftContent,
  };
}

/**
 * Register the discover-foundation command with Commander.js
 */
export function registerDiscoverFoundationCommand(program: Command): void {
  program
    .command('discover-foundation')
    .description('Discover project foundation automatically')
    .option(
      '--output <path>',
      'Output path for final foundation.json (default: spec/foundation.json)',
      'spec/foundation.json'
    )
    .option(
      '--finalize',
      'Finalize foundation.json from edited draft file'
    )
    .option(
      '--draft-path <path>',
      'Path to draft file (default: spec/foundation.json.draft)',
      'spec/foundation.json.draft'
    )
    .action(async (options: { output?: string; finalize?: boolean; draftPath?: string }) => {
      const result = await discoverFoundation({
        outputPath: options.output,
        finalize: options.finalize,
        draftPath: options.draftPath,
      });

      // Emit system-reminder (only visible to AI)
      if (result.systemReminder) {
        console.log(result.systemReminder);
      }

      if (options.finalize) {
        // Finalizing draft
        if (!result.valid) {
          console.error(chalk.red('✗ Foundation validation failed'));
          process.exit(1);
        }

        console.log(chalk.green(`✓ Generated ${result.finalPath}`));
        console.log(chalk.green('✓ Foundation discovered and validated successfully'));
      } else {
        // Creating draft
        console.log(chalk.green(`✓ Generated ${result.draftPath}`));
        console.log(chalk.yellow('\nNext steps:'));
        console.log(chalk.yellow('1. Edit the draft file to replace [QUESTION: ...] placeholders'));
        console.log(chalk.yellow('2. Run: fspec discover-foundation --finalize'));
      }
    });
}
