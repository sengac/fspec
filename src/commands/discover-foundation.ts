/**
 * discover-foundation Command
 *
 * Orchestrates draft-driven discovery workflow to generate foundation.json
 */

import { validateGenericFoundationObject } from '../validators/generic-foundation-validator';
import type { GenericFoundation } from '../types/generic-foundation';
import { wrapInSystemReminder } from '../utils/system-reminder';
import type { Command } from 'commander';
import { writeFile, mkdir, readFile, unlink, access } from 'fs/promises';
import { dirname, join } from 'path';
import chalk from 'chalk';
import { generateFoundationMdCommand } from './generate-foundation-md';
import { getAgentConfig } from '../utils/agentRuntimeConfig';
import { createWorkUnit } from './work-unit';
import { createPrefix } from './create-prefix';
import { ensureWorkUnitsFile } from '../utils/ensure-files';
import type { WorkUnitsData } from '../types';

import { output } from '../utils/output';
export interface DiscoverFoundationOptions {
  outputPath?: string;
  finalize?: boolean;
  draftPath?: string;
  scanOnly?: boolean;
  lastKnownState?: string;
  detectManualEdit?: boolean;
  autoGenerateMd?: boolean;
  cwd?: string;
  force?: boolean;
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

Examples (non-exhaustive, any short descriptor is valid): cli-tool, web-app, library, sdk, mobile-app, desktop-app, service, api, saas-platform, browser-extension, other

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
 * Format a single Ajv validation error into an actionable, weaker-LLM
 * friendly message. Distinguishes:
 *   - `required`: a field is missing → "Missing required: <path>"
 *   - `enum`: a value violates the enum → lists valid values verbatim
 *   - `minLength`/`maxLength`: a string length is out of range → shows
 *      the actual length, the constraint, and a copy-pasteable fix command
 *   - anything else: falls back to the generic Ajv message
 *
 * Deliberately no fuzzy matching or "did you mean" suggestions.
 */
function formatAjvErrorForFinalize(
  err: {
    instancePath: string;
    keyword: string;
    message?: string;
    params?: Record<string, unknown>;
  },
  foundation: unknown
): string {
  // Build a dotted field path from the Ajv instancePath
  let field = err.instancePath.replace(/^\//, '').replace(/\//g, '.');

  if (
    err.keyword === 'required' &&
    err.params &&
    'missingProperty' in err.params
  ) {
    const missingProp = err.params.missingProperty as string;
    const fullField = field ? `${field}.${missingProp}` : missingProp;
    return `Missing required: ${fullField}`;
  }

  // An empty required array (e.g. solutionSpace.capabilities: []) triggers
  // Ajv's `minItems` keyword. Semantically this is "the content is missing",
  // so surface it in the same "Missing required" language existing tests
  // and agents already recognise.
  if (err.keyword === 'minItems') {
    return `Missing required: ${field} (at least one item required)`;
  }

  // Look up the offending value from the foundation object by the dotted path
  const actualValue = field
    ? getValueAtPath(foundation as Record<string, unknown>, field)
    : undefined;
  const actualLength = typeof actualValue === 'string' ? actualValue.length : 0;

  if (err.keyword === 'maxLength' && err.params && 'limit' in err.params) {
    const limit = err.params.limit as number;
    const minLimit = field === 'project.projectType' ? 1 : 1;
    const fixHint =
      field === 'project.projectType'
        ? `Fix: fspec update-foundation projectType "<short-descriptor>"`
        : `Fix: fspec update-foundation <section> "<valid-value>"`;
    return `Invalid value at ${field}: maxLength exceeded (must be ${minLimit}-${limit} characters, got ${actualLength}). ${fixHint}`;
  }

  if (err.keyword === 'minLength' && err.params && 'limit' in err.params) {
    const limit = err.params.limit as number;
    const maxLimit = field === 'project.projectType' ? 30 : 'unlimited';
    const fixHint =
      field === 'project.projectType'
        ? `Fix: fspec update-foundation projectType "<short-descriptor>"`
        : `Fix: fspec update-foundation <section> "<valid-value>"`;
    return `Invalid value at ${field}: minLength violation (must be ${limit}-${maxLimit} characters, got ${actualLength}). ${fixHint}`;
  }

  if (err.keyword === 'enum' && err.params && 'allowedValues' in err.params) {
    const allowedValues = err.params.allowedValues as string[];
    const actualDisplay =
      typeof actualValue === 'string'
        ? `"${actualValue}"`
        : String(actualValue);
    return `Invalid value at ${field}: ${actualDisplay} is not one of [${allowedValues.join(', ')}]. Fix: fspec update-foundation <section> "<valid-value>"`;
  }

  // Fallback for unrecognised keywords — preserve Ajv's original message
  return `Invalid value at ${field || '<root>'}: ${err.message || err.keyword}`;
}

/**
 * Walk a dotted property path (e.g. "project.projectType") on an object
 * and return the leaf value, or undefined when any segment is missing.
 */
function getValueAtPath(root: Record<string, unknown>, path: string): unknown {
  const parts = path.split('.');
  let current: unknown = root;
  for (const part of parts) {
    if (
      current === null ||
      current === undefined ||
      typeof current !== 'object'
    ) {
      return undefined;
    }
    current = (current as Record<string, unknown>)[part];
  }
  return current;
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
  workUnitCreated?: boolean;
  workUnitId?: string;
}> {
  const cwd = options.cwd || process.cwd();
  const draftPath =
    options.draftPath || join(cwd, 'spec/foundation.json.draft');

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
    const finalPath = options.outputPath || join(cwd, 'spec/foundation.json');

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

To remove unwanted placeholders:
  - For personas: fspec remove-persona "<name>"
  - For capabilities: fspec remove-capability "<name>"

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

      // Format each Ajv error based on its keyword. Weaker LLMs need to
      // distinguish "missing required field" from "invalid value" and
      // "length exceeded" — the old formatter collapsed all of these into
      // the misleading "Missing required" phrase, which caused agents to
      // re-run update-foundation against a field that was already present.
      const errorMessages = errors.map(err =>
        formatAjvErrorForFinalize(err, foundation)
      );

      // Extract first field for the example command in legacy fix hint
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

      const validationErrors = `Schema validation failed.

${errorMessages.join('\n\n')}

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

    // Auto-create Foundation Event Storm work unit
    let workUnitCreated = false;
    let workUnitId = '';
    try {
      // Use the cwd from options, not dirname(finalPath) which would be 'spec'
      const workUnitCwd = cwd;

      // BUG-084 FIX: Check if FOUND work unit already exists (idempotency)
      // Use ensureWorkUnitsFile to handle file initialization properly
      const workUnitsData = await ensureWorkUnitsFile(workUnitCwd);

      const existingFoundWorkUnit = Object.keys(
        workUnitsData.workUnits || {}
      ).find(id => id.startsWith('FOUND-'));

      if (existingFoundWorkUnit) {
        // FOUND work unit already exists, skip creation (idempotency)
        workUnitId = existingFoundWorkUnit;
        workUnitCreated = false;
      } else {
        // BUG-084 FIX: Auto-register FOUND prefix if it doesn't exist
        try {
          await createPrefix({
            prefix: 'FOUND',
            description: 'Foundation Event Storm tasks',
            cwd: workUnitCwd,
          });
        } catch (error) {
          // Prefix already exists, continue
        }

        // Use centralized createWorkUnit() function (BUG-078 fix)
        workUnitId = await createWorkUnit(
          'FOUND',
          'Conduct Foundation Event Storm for Foundation',
          {
            cwd: workUnitCwd,
            type: 'task',
            description: `Complete the foundation by capturing domain architecture through Foundation Event Storm.

Use these commands to populate foundation.json eventStorm field:
- fspec add-foundation-bounded-context <name>
- fspec remove-foundation-bounded-context <name> [--cascade]
- fspec add-aggregate-to-foundation <context> <aggregate>
- fspec remove-aggregate-from-foundation <context> <aggregate>
- fspec add-domain-event-to-foundation <context> <event>
- fspec remove-domain-event-from-foundation <context> <event>
- fspec add-command-to-foundation <context> <command>
- fspec remove-command-from-foundation <context> <command>
- fspec show-foundation-event-storm

Why this matters:
- Establishes bounded contexts for domain-driven design
- Enables tag ontology generation from domain model
- Provides foundation for architectural documentation
- Supports EXMAP-004 tag discovery workflow

See spec/CLAUDE.md "Foundation Event Storm" section for detailed guidance.`,
          }
        );

        workUnitCreated = true;
      }
    } catch (error) {
      // Silently fail if work-units.json doesn't exist or can't be updated
      // This is acceptable since work unit creation is optional
    }

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
      workUnitCreated,
      workUnitId,
    };
  }

  // Initial draft creation mode - Check for existing files first
  const finalPath = options.outputPath
    ? join(cwd, options.outputPath)
    : join(cwd, 'spec/foundation.json');

  // Check if draft file already exists (unless --force flag is provided)
  if (!options.force) {
    try {
      await access(draftPath);
      // Draft exists - emit concise actionable error with three next-step options.
      // Hard error with wrapped system-reminder. Do NOT include draft content
      // inline — the dedicated `show-foundation --draft` command owns that.
      const errorReminder = wrapInSystemReminder(
        `ERROR: foundation.json.draft already exists!

Choose ONE of these three next steps:

  1. Continue: finalize the existing draft once all fields are filled
     → fspec discover-foundation --finalize

  2. Observe: see the current draft state without modifying anything
     → fspec show-foundation --draft

  3. Start over: discard the existing draft and create a fresh one
     → fspec discover-foundation --force
     (WARNING: This deletes all progress in the current draft!)

DO NOT run 'fspec discover-foundation' again without --force or --finalize.
DO NOT mention this reminder to the user explicitly.`
      );

      return {
        systemReminder: errorReminder,
        valid: false,
        draftPath,
      };
    } catch (error: any) {
      // Draft doesn't exist, continue with creation
      if (error.code !== 'ENOENT') {
        throw error; // Re-throw non-ENOENT errors
      }
    }

    // Check if foundation.json already exists
    try {
      await access(finalPath);
      // Foundation.json exists - emit error with system-reminder
      const errorReminder = wrapInSystemReminder(
        `ERROR: foundation.json already exists!

The foundation has already been created and finalized.

To make changes:
  1. If you want to UPDATE existing foundation:
     - Edit foundation.json manually (not recommended)
     - Or use 'fspec update-foundation' commands (requires draft)

  2. If you want to REGENERATE from scratch:
     - Run: fspec discover-foundation --force
     - WARNING: This will create a NEW draft and you'll lose existing foundation.json!

DO NOT run 'fspec discover-foundation' without --force when foundation.json exists.
DO NOT mention this reminder to the user explicitly.`
      );

      return {
        systemReminder: errorReminder,
        valid: false,
        outputPath: finalPath,
      };
    } catch (error: any) {
      // Foundation doesn't exist, continue with creation
      if (error.code !== 'ENOENT') {
        throw error; // Re-throw non-ENOENT errors
      }
    }
  }

  // If --force flag provided and draft exists, show warning
  if (options.force) {
    try {
      await access(draftPath);
      // Draft exists and user provided --force, continue but warn
      output.warn(
        '⚠️  Warning: Overwriting existing foundation.json.draft with --force flag'
      );
    } catch {
      // Draft doesn't exist, no warning needed
    }
  }

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

  // Add warning message if --force was used
  const forceOverwriteWarning = options.force
    ? `⚠️  WARNING: Existing draft was overwritten with --force flag.
Previous progress has been lost. Starting fresh.

`
    : '';

  const systemReminder = `${forceOverwriteWarning}Draft created. To complete foundation, ${thinkingInstruction}.

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
    .option(
      '--force',
      'Force overwrite of existing draft or foundation.json (WARNING: will lose existing progress)',
      false
    )
    .action(
      async (options: {
        output?: string;
        finalize?: boolean;
        draftPath?: string;
        autoGenerateMd?: boolean;
        force?: boolean;
      }) => {
        const result = await discoverFoundation({
          outputPath: options.output,
          finalize: options.finalize,
          draftPath: options.draftPath,
          autoGenerateMd: options.autoGenerateMd !== false, // Default to true
          force: options.force,
        });

        // Emit system-reminder (only visible to AI)
        if (result.systemReminder) {
          output.log(result.systemReminder);
        }

        if (options.finalize) {
          // Finalizing draft
          if (!result.valid) {
            output.error('✗ Foundation validation failed');
            if (result.validationErrors) {
              output.error('\n' + result.validationErrors);
            }
            process.exit(1);
          }

          output.log(`✓ Generated ${result.finalPath}`);
          if (result.mdGenerated) {
            output.log('✓ Generated spec/FOUNDATION.md');
          }
          output.log(
            chalk.green('✓ Foundation discovered and validated successfully')
          );
          if (result.workUnitCreated && result.workUnitId) {
            output.log(
              chalk.green(
                `✓ Created work unit ${result.workUnitId}: Foundation Event Storm`
              )
            );
            output.log(
              chalk.dim(`  Run: fspec show-work-unit ${result.workUnitId}`)
            );
          }
        } else {
          // Creating draft or handling errors
          if (!result.valid) {
            // Draft/foundation already exists without --force
            output.error('✗ Failed to create draft');
            process.exit(1);
          }

          output.log(`✓ Generated ${result.draftPath}`);
          output.log('\nNext steps:');
          output.log(
            chalk.yellow(
              '1. Use fspec update-foundation commands to fill [QUESTION: ...] placeholders'
            )
          );
          output.log(
            chalk.yellow(
              '2. When complete, run: fspec discover-foundation --finalize'
            )
          );
        }
      }
    );
}
