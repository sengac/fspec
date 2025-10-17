import { writeFile, readFile, access } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import type { Foundation } from '../types/foundation';
import { validateFoundationJson } from '../validators/json-schema';
import { generateFoundationMd } from '../generators/foundation-md';
import { ensureFoundationFile } from '../utils/ensure-files';
import { discoverFoundation } from './discover-foundation';

interface UpdateFoundationOptions {
  section: string;
  content: string;
  cwd?: string;
}

interface UpdateFoundationResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function updateFoundation(
  options: UpdateFoundationOptions
): Promise<UpdateFoundationResult> {
  const { section, content, cwd = process.cwd() } = options;

  // Validate inputs
  if (!section || section.trim().length === 0) {
    return {
      success: false,
      error: 'Section name cannot be empty',
    };
  }

  if (
    content === undefined ||
    content === null ||
    content.trim().length === 0
  ) {
    return {
      success: false,
      error: 'Section content cannot be empty',
    };
  }

  try {
    const draftPath = join(cwd, 'spec/foundation.json.draft');
    const foundationJsonPath = join(cwd, 'spec/foundation.json');
    const foundationMdPath = join(cwd, 'spec/FOUNDATION.md');

    // Check if draft exists - if yes, update draft instead of final foundation
    let isDraft = false;
    let targetPath = foundationJsonPath;

    try {
      await access(draftPath);
      isDraft = true;
      targetPath = draftPath;
    } catch {
      // Draft doesn't exist, use final foundation.json
    }

    // Load or create foundation file (draft or final)
    let foundationData: Foundation;
    if (isDraft) {
      const draftContent = await readFile(draftPath, 'utf-8');
      foundationData = JSON.parse(draftContent);
    } else {
      foundationData = await ensureFoundationFile(cwd);
    }

    // Update the JSON field based on section name
    const updated = updateJsonField(foundationData, section, content);
    if (!updated) {
      return {
        success: false,
        error: `Unknown section: "${section}". Use field names like: projectOverview, problemDefinition, etc.`,
      };
    }

    // Write updated file (draft or final)
    await writeFile(
      targetPath,
      JSON.stringify(foundationData, null, 2),
      'utf-8'
    );

    if (isDraft) {
      // When updating draft, don't validate or regenerate FOUNDATION.md
      // (those happen during finalize step)
      return {
        success: true,
        message: `Updated "${section}" in foundation.json.draft`,
      };
    } else {
      // When updating final foundation, validate and regenerate FOUNDATION.md
      const validation = await validateFoundationJson(foundationJsonPath);
      if (!validation.valid) {
        const errorMessages = validation.errors?.map(e => e.message).join(', ');
        return {
          success: false,
          error: `Updated foundation.json failed schema validation: ${errorMessages}`,
        };
      }

      // Regenerate FOUNDATION.md from JSON
      const markdown = await generateFoundationMd(foundationData);
      await writeFile(foundationMdPath, markdown, 'utf-8');

      return {
        success: true,
        message: `Updated "${section}" section in FOUNDATION.md`,
      };
    }
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

// Helper function to update JSON field based on section name
function updateJsonField(
  foundation: any,
  section: string,
  content: string
): boolean {
  // Map section names to JSON field paths (generic schema v2.0.0)
  switch (section) {
    // Project fields
    case 'projectName':
    case 'name':
      foundation.project = foundation.project || {};
      foundation.project.name = content;
      return true;

    case 'projectVision':
    case 'vision':
      foundation.project = foundation.project || {};
      foundation.project.vision = content;
      return true;

    case 'projectType':
      foundation.project = foundation.project || {};
      foundation.project.projectType = content;
      return true;

    // Problem space fields
    case 'problemTitle':
      foundation.problemSpace = foundation.problemSpace || {};
      foundation.problemSpace.primaryProblem =
        foundation.problemSpace.primaryProblem || {};
      foundation.problemSpace.primaryProblem.title = content;
      return true;

    case 'problemDefinition':
    case 'problemDescription':
      foundation.problemSpace = foundation.problemSpace || {};
      foundation.problemSpace.primaryProblem =
        foundation.problemSpace.primaryProblem || {};
      foundation.problemSpace.primaryProblem.description = content;
      return true;

    case 'problemImpact':
      if (!['high', 'medium', 'low'].includes(content)) {
        return false;
      }
      foundation.problemSpace = foundation.problemSpace || {};
      foundation.problemSpace.primaryProblem =
        foundation.problemSpace.primaryProblem || {};
      foundation.problemSpace.primaryProblem.impact = content;
      return true;

    // Solution space fields
    case 'solutionOverview':
    case 'projectOverview':
      foundation.solutionSpace = foundation.solutionSpace || {};
      foundation.solutionSpace.overview = content;
      return true;

    // Legacy mappings for backward compatibility
    case 'testingStrategy':
    case 'developmentTools':
    case 'architecturePattern':
    case 'painPoints':
    case 'methodology':
      // These were old schema fields - map to solutionSpace.overview
      foundation.solutionSpace = foundation.solutionSpace || {};
      foundation.solutionSpace.overview = content;
      return true;

    default:
      return false;
  }
}

export async function updateFoundationCommand(
  section: string,
  content: string
): Promise<void> {
  try {
    const result = await updateFoundation({
      section,
      content,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green('✓'), result.message);

    // Show different output based on whether we updated draft or final
    if (result.message?.includes('draft')) {
      console.log(chalk.gray('  Updated: spec/foundation.json.draft'));

      // IMPORTANT: Chain to next field during draft-driven discovery
      // Scan draft for next field
      const scanResult = await discoverFoundation({
        scanOnly: true,
        draftPath: 'spec/foundation.json.draft',
      });

      // Emit system-reminder for next field (visible to AI, stripped from user output)
      if (scanResult.systemReminder) {
        console.log(scanResult.systemReminder);
      }
    } else {
      console.log(chalk.gray('  Updated: spec/foundation.json'));
      console.log(chalk.gray('  Regenerated: spec/FOUNDATION.md'));
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerUpdateFoundationCommand(program: Command): void {
  program
    .command('update-foundation')
    .description('Update section content in FOUNDATION.md')
    .argument('<section>', 'Section name (e.g., "What We Are Building", "Why")')
    .argument('<content>', 'Section content (can be multi-line)')
    .action(updateFoundationCommand);
}
