/**
 * discover-foundation Command
 *
 * Orchestrates draft-driven discovery workflow to generate foundation.json
 */

import { validateGenericFoundationObject } from '../validators/generic-foundation-validator';
import type { GenericFoundation } from '../types/generic-foundation';
import { wrapInSystemReminder } from '../utils/system-reminder';
import type { Command } from 'commander';
import { writeFile, mkdir, readFile, unlink } from 'fs/promises';
import { dirname } from 'path';
import chalk from 'chalk';
import { generateFoundationMdCommand } from './generate-foundation-md';
import { getAgentConfig } from '../utils/agentRuntimeConfig';

export interface DiscoverFoundationOptions {
  outputPath?: string;
  finalize?: boolean;
  draftPath?: string;
  scanOnly?: boolean;
  lastKnownState?: string;
  detectManualEdit?: boolean;
  autoGenerateMd?: boolean;
  cwd?: string;
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
    {
      path: 'problemSpace.primaryProblem.title',
      value: draft.problemSpace?.primaryProblem?.title,
    },
    {
      path: 'problemSpace.primaryProblem.description',
      value: draft.problemSpace?.primaryProblem?.description,
    },
    { path: 'solutionSpace.overview', value: draft.solutionSpace?.overview },
    {
      path: 'solutionSpace.capabilities',
      value: draft.solutionSpace?.capabilities,
    },
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

    const valueStr =
      typeof field.value === 'string'
        ? field.value
        : JSON.stringify(field.value);
    const hasPlaceholder =
      valueStr.includes('[QUESTION:') || valueStr.includes('[DETECTED:');

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
 * Generate field-specific system-reminder with agent-aware guidance
 */
function generateFieldReminder(
  fieldPath: string,
  fieldNum: number,
  totalFields: number,
  cwd: string,
  detectedValue?: string
): string {
  // Detect current agent to provide appropriate guidance
  const agent = getAgentConfig(cwd);
  const reminders: Record<string, string> = {
    'project.name': `Field ${fieldNum}/${totalFields}: project.name

Analyze project configuration to determine project name. Confirm with human.

Run: fspec update-foundation projectName "<name>"`,

    'project.vision': `Field ${fieldNum}/${totalFields}: project.vision (elevator pitch)

${agent.supportsMetaCognition ? 'ULTRATHINK: Read ALL code, understand the system deeply.' : 'Think a lot about the entire codebase.'} What is the core PURPOSE?
Focus on WHY this exists, not HOW it works.

Ask human to confirm vision.

Run: fspec update-foundation projectVision "your vision"`,

    'project.projectType': `Field ${fieldNum}/${totalFields}: project.projectType

${detectedValue ? `[DETECTED: ${detectedValue}] ` : ''}Analyze codebase to determine project type. Verify with human.

Options: cli-tool, web-app, library, sdk, mobile-app, desktop-app, service, api, other

Run: fspec update-foundation projectType "<type>"`,

    'problemSpace.primaryProblem.title': `Field ${fieldNum}/${totalFields}: problemSpace.primaryProblem.title

CRITICAL: Think from USER perspective. WHO uses this (persona)?
WHAT problem do THEY face? WHY do they need this solution?

Analyze codebase to understand user pain, ask human. Requires title, description, impact.

Run: fspec update-foundation problemTitle "Problem Title"`,

    'problemSpace.primaryProblem.description': `Field ${fieldNum}/${totalFields}: problemSpace.primaryProblem.description

USER perspective: Describe the problem users face in detail.

Run: fspec update-foundation problemDefinition "Problem description"`,

    'solutionSpace.overview': `Field ${fieldNum}/${totalFields}: solutionSpace.overview

High-level solution approach. Focus on WHAT not HOW.

Run: fspec update-foundation solutionOverview "Solution overview"`,

    'solutionSpace.capabilities': `Field ${fieldNum}/${totalFields}: solutionSpace.capabilities

List 3-7 high-level abilities users have. Focus on WHAT not HOW.

Example: "Spec Validation" (WHAT), NOT "Uses Cucumber parser" (HOW)

Analyze user-facing functionality to identify capabilities.

Run: fspec add-capability "Capability Name" "Capability Description"
Run again for each capability (3-7 recommended)`,

    personas: `Field ${fieldNum}/${totalFields}: personas

Identify ALL user types from interactions.
CLI tools: who runs commands? Web apps: who uses UI + who calls API?

Analyze ALL user-facing code. Ask human about goals and pain points.

Run: fspec add-persona "Persona Name" "Persona Description" --goal "Primary goal"
Run again for each persona (repeat --goal for multiple goals)`,
  };

  const message =
    reminders[fieldPath] || `Field ${fieldNum}/${totalFields}: ${fieldPath}`;
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
  const cwd = options.cwd || process.cwd();
  const draftPath = options.draftPath || 'spec/foundation.json.draft';

  // Handle manual editing detection
  if (options.detectManualEdit && options.lastKnownState) {
    try {
      const currentContent = await readFile(draftPath, 'utf-8');
      if (currentContent !== options.lastKnownState) {
        // Manual edit detected - revert changes
        await writeFile(draftPath, options.lastKnownState, 'utf-8');

        const errorReminder =
          wrapInSystemReminder(`ERROR: CRITICAL: You manually edited foundation.json.draft

This violates the workflow. You MUST use:
  fspec update-foundation <section> "<value>"
  fspec add-capability "<name>" "<description>"
  fspec add-persona "<name>" "<description>" --goal "<goal>"

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
      if (
        scan.fieldPath === 'project.projectType' &&
        draft.project.projectType
      ) {
        const match = draft.project.projectType.match(
          /\[DETECTED:\s*([^\]]+)\]/
        );
        if (match) {
          detectedValue = match[1].trim();
        }
      }

      // Generate field-specific reminder
      const systemReminder = generateFieldReminder(
        scan.fieldPath,
        scan.fieldNumber,
        scan.totalFields,
        cwd,
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

    // Check if all placeholder fields are filled
    if (!allFieldsComplete) {
      const validationErrors = `Cannot finalize: draft still has unfilled placeholder fields.

Field '${scan.nextField}' still contains [QUESTION:] or [DETECTED:] placeholders.

Please fill all placeholder fields before finalizing:
  - For simple fields: fspec update-foundation <section> "<value>"
  - For capabilities: fspec add-capability "<name>" "<description>"
  - For personas: fspec add-persona "<name>" "<description>" --goal "<goal>"

Then re-run: fspec discover-foundation --finalize`;

      return {
        systemReminder: '',
        foundation,
        valid: false,
        validated: true,
        validationErrors,
      };
    }

    // Validate foundation
    const validation = validateGenericFoundationObject(foundation);

    if (!validation.valid) {
      const errors = validation.errors || [];
      const errorMessages = errors.map(err => {
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
      const firstField = errors[0]
        ? (() => {
            let field = errors[0].instancePath
              .replace(/^\//, '')
              .replace(/\//g, '.');
            if (errors[0].params && 'missingProperty' in errors[0].params) {
              const missingProp = errors[0].params.missingProperty as string;
              field = field ? `${field}.${missingProp}` : missingProp;
            }
            return field;
          })()
        : '<path>';

      const validationErrors = `Schema validation failed. ${errorMessages.join(', ')}

Fix by running appropriate commands:
  - For simple fields: fspec update-foundation <section> "<value>"
  - For capabilities: fspec add-capability "<name>" "<description>"
  - For personas: fspec add-persona "<name>" "<description>" --goal "<goal>"

Then re-run: fspec discover-foundation --finalize`;

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
      const mdResult = await generateFoundationMdCommand({
        cwd: dirname(dirname(finalPath)),
      });
      mdGenerated = mdResult.success;
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
    ? generateFieldReminder(
        scan.fieldPath,
        scan.fieldNumber,
        scan.totalFields,
        cwd
      )
    : '';

  // Detect agent for initial draft guidance
  const agent = getAgentConfig(cwd);
  const thinkingInstruction = agent.supportsMetaCognition
    ? 'you must ULTRATHINK the entire codebase'
    : 'you must think a lot about the entire codebase';

  const systemReminder = `Draft created. To complete foundation, ${thinkingInstruction}.

Analyze EVERYTHING: code structure, entry points, user interactions, documentation.
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
    .option('--finalize', 'Finalize foundation.json from edited draft file')
    .option(
      '--draft-path <path>',
      'Path to draft file (default: spec/foundation.json.draft)',
      'spec/foundation.json.draft'
    )
    .option(
      '--auto-generate-md',
      'Automatically generate FOUNDATION.md after finalization (default: true)',
      true
    )
    .action(
      async (options: {
        output?: string;
        finalize?: boolean;
        draftPath?: string;
        autoGenerateMd?: boolean;
      }) => {
        const result = await discoverFoundation({
          outputPath: options.output,
          finalize: options.finalize,
          draftPath: options.draftPath,
          autoGenerateMd: options.autoGenerateMd !== false, // Default to true
        });

        // Emit system-reminder (only visible to AI)
        if (result.systemReminder) {
          console.log(result.systemReminder);
        }

        if (options.finalize) {
          // Finalizing draft
          if (!result.valid) {
            console.error(chalk.red('✗ Foundation validation failed'));
            if (result.validationErrors) {
              console.error(chalk.yellow('\n' + result.validationErrors));
            }
            process.exit(1);
          }

          console.log(chalk.green(`✓ Generated ${result.finalPath}`));
          if (result.mdGenerated) {
            console.log(chalk.green('✓ Generated spec/FOUNDATION.md'));
          }
          console.log(
            chalk.green('✓ Foundation discovered and validated successfully')
          );
        } else {
          // Creating draft
          console.log(chalk.green(`✓ Generated ${result.draftPath}`));
          console.log(chalk.yellow('\nNext steps:'));
          console.log(
            chalk.yellow(
              '1. Use fspec update-foundation commands to fill [QUESTION: ...] placeholders'
            )
          );
          console.log(
            chalk.yellow(
              '2. When complete, run: fspec discover-foundation --finalize'
            )
          );
        }
      }
    );
}
