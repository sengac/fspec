import { readFile, writeFile, access } from 'fs/promises';
import type { Command } from 'commander';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface AddBackgroundOptions {
  feature: string;
  text: string;
  cwd?: string;
}

interface AddBackgroundResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function addBackground(
  options: AddBackgroundOptions
): Promise<AddBackgroundResult> {
  const { feature, text, cwd = process.cwd() } = options;

  // Validate background text
  if (!text || text.trim().length === 0) {
    return {
      success: false,
      error: 'Background text cannot be empty',
    };
  }

  try {
    // Find the feature file
    let featurePath: string;

    // Check if it's a direct path
    if (feature.endsWith('.feature')) {
      featurePath = join(cwd, feature);
      try {
        await access(featurePath);
      } catch {
        return {
          success: false,
          error: `Feature file not found: ${feature}`,
        };
      }
    } else {
      // Search for the feature file by name
      const files = await glob(['spec/features/**/*.feature'], {
        cwd,
        absolute: false,
      });
      const matchingFile = files.find(f => {
        const basename = f.split('/').pop()?.replace('.feature', '');
        return basename === feature;
      });

      if (!matchingFile) {
        return {
          success: false,
          error: `Feature file not found: ${feature}`,
        };
      }

      featurePath = join(cwd, matchingFile);
    }

    // Read the feature file
    let content = await readFile(featurePath, 'utf-8');

    // Parse to validate current Gherkin
    const builder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const matcher = new Gherkin.GherkinClassicTokenMatcher();
    const parser = new Gherkin.Parser(builder, matcher);

    try {
      parser.parse(content);
    } catch (error: any) {
      return {
        success: false,
        error: `Invalid Gherkin syntax in feature file: ${error.message}`,
      };
    }

    // Split content into lines
    const lines = content.split('\n');

    // Find the Feature line
    let featureLineIndex = -1;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].trim().startsWith('Feature:')) {
        featureLineIndex = i;
        break;
      }
    }

    if (featureLineIndex === -1) {
      return {
        success: false,
        error: 'No Feature line found in file',
      };
    }

    // Find the end of the doc string (if it exists)
    let docStringEndIndex = featureLineIndex;
    let inDocString = false;

    for (let i = featureLineIndex + 1; i < lines.length; i++) {
      const line = lines[i].trim();

      // Stop at Background, Scenario, or tag
      if (
        !inDocString &&
        (line.startsWith('Background:') ||
          line.startsWith('Scenario:') ||
          line.startsWith('@'))
      ) {
        break;
      }

      // Track doc string boundaries
      if (line === '"""') {
        if (!inDocString) {
          inDocString = true;
        } else {
          docStringEndIndex = i;
          inDocString = false;
          break;
        }
      }
    }

    // Check if there's already a Background section
    let existingBackgroundStart = -1;
    let existingBackgroundEnd = -1;

    for (let i = docStringEndIndex + 1; i < lines.length; i++) {
      const line = lines[i].trim();

      // Stop at Scenario or tag (but not if we're inside Background)
      if (
        existingBackgroundStart === -1 &&
        (line.startsWith('Scenario:') || line.startsWith('@'))
      ) {
        break;
      }

      // Found Background line
      if (line.startsWith('Background:')) {
        existingBackgroundStart = i;
        continue;
      }

      // If we're in a Background section, find where it ends
      if (existingBackgroundStart !== -1) {
        // Background ends when we hit a Scenario, tag, or another feature element
        if (
          line.startsWith('Scenario:') ||
          line.startsWith('@') ||
          line.startsWith('Feature:')
        ) {
          existingBackgroundEnd = i - 1;
          // Skip back over blank lines
          while (
            existingBackgroundEnd > existingBackgroundStart &&
            lines[existingBackgroundEnd].trim() === ''
          ) {
            existingBackgroundEnd--;
          }
          break;
        }
      }
    }

    // If Background section started but didn't end (goes to end of file)
    if (existingBackgroundStart !== -1 && existingBackgroundEnd === -1) {
      existingBackgroundEnd = lines.length - 1;
      while (
        existingBackgroundEnd > existingBackgroundStart &&
        lines[existingBackgroundEnd].trim() === ''
      ) {
        existingBackgroundEnd--;
      }
    }

    // Create the new Background section
    const backgroundLines = ['  Background: User Story'];
    const textLines = text.split('\n');
    for (const textLine of textLines) {
      backgroundLines.push(`    ${textLine}`);
    }

    // Insert or replace the Background
    if (existingBackgroundStart !== -1 && existingBackgroundEnd !== -1) {
      // Replace existing Background
      lines.splice(
        existingBackgroundStart,
        existingBackgroundEnd - existingBackgroundStart + 1,
        ...backgroundLines,
        '' // Add blank line after Background
      );
    } else {
      // Insert new Background after doc string (or after Feature line if no doc string)
      lines.splice(docStringEndIndex + 1, 0, '', ...backgroundLines, '');
    }

    // Join lines back
    const newContent = lines.join('\n');

    // Validate the new content
    try {
      parser.parse(newContent);
    } catch (error: any) {
      return {
        success: false,
        error: `Generated invalid Gherkin: ${error.message}`,
      };
    }

    // Write the updated content
    await writeFile(featurePath, newContent, 'utf-8');

    return {
      success: true,
      message: `Added background to ${feature}`,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function addBackgroundCommand(
  feature: string,
  text: string
): Promise<void> {
  try {
    const result = await addBackground({
      feature,
      text,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green('✓'), result.message);
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}

export function registerAddBackgroundCommand(program: Command): void {
  program
    .command('add-background')
    .description(
      'Add or update Background (user story) section in a feature file'
    )
    .argument(
      '<feature>',
      'Feature file name or path (e.g., "login" or "spec/features/login.feature")'
    )
    .argument('<text>', 'User story text (As a... I want to... So that...)')
    .action(addBackgroundCommand);
}
