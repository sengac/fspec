import { readFile, writeFile } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface UpdateStepOptions {
  feature: string;
  scenario: string;
  currentStep: string;
  text?: string;
  keyword?: string;
  cwd?: string;
}

interface UpdateStepResult {
  success: boolean;
  message?: string;
  error?: string;
}

export async function updateStep(options: UpdateStepOptions): Promise<UpdateStepResult> {
  const { feature, scenario, currentStep, text, keyword, cwd = process.cwd() } = options;

  // Validate that at least one update is specified
  if (!text && !keyword) {
    return {
      success: false,
      error: 'No updates specified. Use --text and/or --keyword',
    };
  }

  // Resolve feature file path
  let featurePath: string;
  if (feature.endsWith('.feature')) {
    featurePath = join(cwd, feature);
  } else if (feature.startsWith('spec/features/')) {
    featurePath = join(cwd, feature);
  } else {
    featurePath = join(cwd, 'spec/features', `${feature}.feature`);
  }

  // Read feature file
  let content: string;
  try {
    content = await readFile(featurePath, 'utf-8');
  } catch (error: any) {
    if (error.code === 'ENOENT') {
      return {
        success: false,
        error: `Feature file not found: ${featurePath}`,
      };
    }
    throw error;
  }

  // Parse Gherkin to find scenario and step
  const uuidFn = Messages.IdGenerator.uuid();
  const builder = new Gherkin.AstBuilder(uuidFn);
  const matcher = new Gherkin.GherkinClassicTokenMatcher();
  const parser = new Gherkin.Parser(builder, matcher);

  let gherkinDocument;
  try {
    gherkinDocument = parser.parse(content);
  } catch (error: any) {
    return {
      success: false,
      error: `Invalid Gherkin syntax: ${error.message}`,
    };
  }

  if (!gherkinDocument.feature) {
    return {
      success: false,
      error: 'Feature file does not contain a valid Feature',
    };
  }

  // Find the scenario
  const scenarioChild = gherkinDocument.feature.children.find(
    child => child.scenario && child.scenario.name === scenario
  );

  if (!scenarioChild || !scenarioChild.scenario) {
    return {
      success: false,
      error: `Scenario '${scenario}' not found in feature file`,
    };
  }

  // Find the step by matching text (with or without keyword)
  const stepToUpdate = scenarioChild.scenario.steps.find(s => {
    const stepText = s.text;
    const fullStepText = `${s.keyword}${s.text}`;
    return (
      stepText === currentStep ||
      fullStepText.trim() === currentStep.trim() ||
      `${s.keyword.trim()} ${stepText}` === currentStep.trim()
    );
  });

  if (!stepToUpdate) {
    return {
      success: false,
      error: `Step '${currentStep}' not found in scenario '${scenario}'`,
    };
  }

  // Get step location
  const stepLine = stepToUpdate.location.line;

  // Split content into lines
  const lines = content.split('\n');

  // Find the step line (convert to 0-indexed)
  const lineIndex = stepLine - 1;
  const stepLineContent = lines[lineIndex];

  // Parse current step to get indentation and keyword
  const stepMatch = stepLineContent.match(/^(\s*)(Given|When|Then|And|But)\s+(.+)$/);

  if (!stepMatch) {
    return {
      success: false,
      error: 'Could not parse step line',
    };
  }

  const indentation = stepMatch[1];
  const currentKeyword = stepMatch[2];
  const currentText = stepMatch[3];

  // Determine new keyword and text
  const newKeyword = keyword || currentKeyword;
  let newText: string;

  if (text) {
    // If text includes keyword, extract just the text part
    const textMatch = text.match(/^(?:Given|When|Then|And|But)\s+(.+)$/);
    if (textMatch) {
      newText = textMatch[1];
    } else {
      newText = text;
    }
  } else {
    newText = currentText;
  }

  // Create new step line
  const newStepLine = `${indentation}${newKeyword} ${newText}`;

  // Replace the line
  const newLines = [...lines];
  newLines[lineIndex] = newStepLine;

  const newContent = newLines.join('\n');

  // Validate the new content is still valid Gherkin
  try {
    const newBuilder = new Gherkin.AstBuilder(Messages.IdGenerator.uuid());
    const newMatcher = new Gherkin.GherkinClassicTokenMatcher();
    const newParser = new Gherkin.Parser(newBuilder, newMatcher);
    newParser.parse(newContent);
  } catch (error: any) {
    return {
      success: false,
      error: `Update would result in invalid Gherkin: ${error.message}`,
    };
  }

  // Write the updated content
  await writeFile(featurePath, newContent, 'utf-8');

  const fileName = featurePath.split('/').pop();
  return {
    success: true,
    message: `Successfully updated step in scenario '${scenario}' in ${fileName}`,
  };
}

export async function updateStepCommand(
  feature: string,
  scenario: string,
  currentStep: string,
  options: { text?: string; keyword?: string }
): Promise<void> {
  try {
    const result = await updateStep({
      feature,
      scenario,
      currentStep,
      text: options.text,
      keyword: options.keyword,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.green(`✓ ${result.message}`));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
