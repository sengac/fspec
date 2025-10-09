import { readFile, writeFile, access } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface Step {
  keyword: string;
  text: string;
}

interface Scenario {
  name: string;
  steps: Step[];
}

interface FeatureAC {
  name: string;
  tags: string[];
  description?: string;
  background?: string;
  scenarios: Scenario[];
}

interface ShowACOptions {
  tags?: string[];
  format?: 'text' | 'markdown' | 'json';
  output?: string;
  cwd?: string;
}

interface ShowACResult {
  success: boolean;
  features: FeatureAC[];
  totalScenarios: number;
  message: string;
  output?: string;
  error?: string;
}

export async function showAcceptanceCriteria(
  options: ShowACOptions = {}
): Promise<ShowACResult> {
  const cwd = options.cwd || process.cwd();
  const tags = options.tags || [];
  const format = options.format || 'text';

  // Check if spec/features exists
  const featuresDir = join(cwd, 'spec', 'features');

  try {
    await access(featuresDir);
  } catch {
    return {
      success: false,
      features: [],
      totalScenarios: 0,
      message: '',
      error: 'spec/features directory not found',
    };
  }

  let files: string[];
  try {
    files = await glob(['spec/features/**/*.feature'], {
      cwd,
      absolute: false,
    });
  } catch {
    return {
      success: false,
      features: [],
      totalScenarios: 0,
      message: '',
      error: 'Error reading feature files',
    };
  }

  if (files.length === 0) {
    return {
      success: true,
      features: [],
      totalScenarios: 0,
      message: 'No feature files found in spec/features/',
    };
  }

  const features: FeatureAC[] = [];
  let totalScenarios = 0;

  for (const file of files) {
    try {
      const content = await readFile(join(cwd, file), 'utf-8');

      const uuidFn = Messages.IdGenerator.uuid();
      const builder = new Gherkin.AstBuilder(uuidFn);
      const matcher = new Gherkin.GherkinClassicTokenMatcher();
      const parser = new Gherkin.Parser(builder, matcher);

      let gherkinDocument;
      try {
        gherkinDocument = parser.parse(content);
      } catch {
        continue;
      }

      if (!gherkinDocument.feature) {
        continue;
      }

      // Check if feature matches tags
      const featureTags = gherkinDocument.feature.tags.map(t => t.name);
      if (tags.length > 0) {
        const matchesAllTags = tags.every(tag => featureTags.includes(tag));
        if (!matchesAllTags) {
          continue;
        }
      }

      // Extract acceptance criteria
      const featureAC: FeatureAC = {
        name: gherkinDocument.feature.name,
        tags: featureTags,
        description: gherkinDocument.feature.description || undefined,
        scenarios: [],
      };

      // Extract background
      for (const child of gherkinDocument.feature.children) {
        if (child.background) {
          const backgroundSteps = child.background.steps
            .map(s => `${s.keyword}${s.text}`)
            .join('\n');
          featureAC.background =
            child.background.description || child.background.name
              ? `${child.background.name}\n${child.background.description || ''}\n${backgroundSteps}`
              : backgroundSteps;
        }
      }

      // Extract scenarios
      for (const child of gherkinDocument.feature.children) {
        if (child.scenario && child.scenario.keyword === 'Scenario') {
          const scenario: Scenario = {
            name: child.scenario.name,
            steps: child.scenario.steps.map(s => ({
              keyword: s.keyword.trim(),
              text: s.text,
            })),
          };
          featureAC.scenarios.push(scenario);
          totalScenarios++;
        }
      }

      features.push(featureAC);
    } catch {
      // Skip files with errors
    }
  }

  let message = '';
  if (features.length === 0 && tags.length > 0) {
    message = `No features found matching tags: ${tags.join(', ')}`;
  } else if (features.length === 0) {
    message = 'No features found';
  } else if (tags.length > 0) {
    message = `Showing acceptance criteria for ${totalScenarios} scenario${totalScenarios === 1 ? '' : 's'} from ${features.length} feature${features.length === 1 ? '' : 's'} matching tags: ${tags.join(', ')}`;
  } else {
    message = `Showing acceptance criteria for ${totalScenarios} scenario${totalScenarios === 1 ? '' : 's'} from ${features.length} feature${features.length === 1 ? '' : 's'}`;
  }

  // Generate output based on format
  let output = '';
  if (format === 'markdown') {
    output = generateMarkdown(features);
  } else if (format === 'json') {
    output = JSON.stringify(features, null, 2);
  } else {
    output = generateTextOutput(features);
  }

  // Write to file if output path specified
  if (options.output) {
    await writeFile(options.output, output, 'utf-8');
    const filename = options.output.split('/').pop();
    message = `Acceptance criteria written to ${filename}`;
  }

  return {
    success: true,
    features,
    totalScenarios,
    message,
    output,
  };
}

function generateMarkdown(features: FeatureAC[]): string {
  let md = '';

  for (const feature of features) {
    md += `# ${feature.name}\n\n`;

    if (feature.tags.length > 0) {
      md += `**Tags:** ${feature.tags.join(' ')}\n\n`;
    }

    if (feature.description) {
      md += `${feature.description}\n\n`;
    }

    if (feature.background) {
      md += `> **Background:**\n> ${feature.background.split('\n').join('\n> ')}\n\n`;
    }

    for (const scenario of feature.scenarios) {
      md += `## ${scenario.name}\n\n`;

      for (const step of scenario.steps) {
        md += `- **${step.keyword}** ${step.text}\n`;
      }

      md += '\n';
    }

    if (feature.scenarios.length === 0) {
      md += '_No scenarios defined_\n\n';
    }

    md += '---\n\n';
  }

  return md;
}

function generateTextOutput(features: FeatureAC[]): string {
  let output = '';

  for (const feature of features) {
    output += chalk.bold.blue(`\n${feature.name}\n`);
    output += chalk.gray('─'.repeat(feature.name.length)) + '\n';

    if (feature.tags.length > 0) {
      output += chalk.cyan(`Tags: ${feature.tags.join(' ')}\n`);
    }

    if (feature.description) {
      output += chalk.gray(`\n${feature.description}\n`);
    }

    if (feature.background) {
      output += chalk.yellow(`\nBackground:\n${feature.background}\n`);
    }

    for (const scenario of feature.scenarios) {
      output += chalk.bold.green(`\n  Scenario: ${scenario.name}\n`);

      for (const step of scenario.steps) {
        output += chalk.white(`    ${step.keyword} ${step.text}\n`);
      }
    }

    if (feature.scenarios.length === 0) {
      output += chalk.gray('\n  No scenarios defined\n');
    }

    output += '\n';
  }

  return output;
}

export async function showAcceptanceCriteriaCommand(options: {
  tag?: string | string[];
  format?: string;
  output?: string;
}): Promise<void> {
  const tags = Array.isArray(options.tag)
    ? options.tag
    : options.tag
      ? [options.tag]
      : [];
  const format = (options.format as 'text' | 'markdown' | 'json') || 'text';

  try {
    const result = await showAcceptanceCriteria({
      tags,
      format,
      output: options.output,
    });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    console.log(chalk.blue(result.message));

    if (!options.output) {
      console.log(result.output);
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
