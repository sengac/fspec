import { readFile, writeFile, access } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface AddArchitectureOptions {
  feature: string;
  text: string;
  cwd?: string;
}

interface AddArchitectureResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function addArchitecture(options: AddArchitectureOptions): Promise<AddArchitectureResult> {
  const { feature, text, cwd = process.cwd() } = options;

  // Validate architecture text
  if (!text || text.trim().length === 0) {
    return {
      success: false,
      error: 'Architecture text cannot be empty',
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
      const files = await glob(['spec/features/**/*.feature'], { cwd, absolute: false });
      const matchingFile = files.find((f) => {
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

    // Check if there's already a doc string after the Feature line
    let existingDocStringStart = -1;
    let existingDocStringEnd = -1;

    for (let i = featureLineIndex + 1; i < lines.length; i++) {
      const line = lines[i].trim();

      // Stop at Background, Scenario, or tag
      if (line.startsWith('Background:') || line.startsWith('Scenario:') || line.startsWith('@')) {
        break;
      }

      // Found opening doc string
      if (line === '"""' && existingDocStringStart === -1) {
        existingDocStringStart = i;
        continue;
      }

      // Found closing doc string
      if (line === '"""' && existingDocStringStart !== -1) {
        existingDocStringEnd = i;
        break;
      }
    }

    // Create the new doc string
    const docStringLines = ['  """'];
    const textLines = text.split('\n');
    for (const textLine of textLines) {
      docStringLines.push(`  ${textLine}`);
    }
    docStringLines.push('  """');

    // Insert or replace the doc string
    if (existingDocStringStart !== -1 && existingDocStringEnd !== -1) {
      // Replace existing doc string
      lines.splice(existingDocStringStart, existingDocStringEnd - existingDocStringStart + 1, ...docStringLines);
    } else {
      // Insert new doc string after Feature line
      lines.splice(featureLineIndex + 1, 0, ...docStringLines);
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
      message: `Added architecture documentation to ${feature}`,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function addArchitectureCommand(feature: string, text: string): Promise<void> {
  try {
    const result = await addArchitecture({
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
