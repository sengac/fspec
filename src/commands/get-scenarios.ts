import { readFile, access } from 'fs/promises';
import { join } from 'path';
import chalk from 'chalk';
import { glob } from 'tinyglobby';
import * as Gherkin from '@cucumber/gherkin';
import * as Messages from '@cucumber/messages';

interface ScenarioInfo {
  feature: string;
  name: string;
  line: number;
}

interface GetScenariosOptions {
  tags?: string[];
  format?: 'text' | 'json';
  cwd?: string;
}

interface GetScenariosResult {
  success: boolean;
  scenarios: ScenarioInfo[];
  totalCount: number;
  message: string;
  warnings?: string[];
  error?: string;
}

export async function getScenarios(
  options: GetScenariosOptions = {}
): Promise<GetScenariosResult> {
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
      scenarios: [],
      totalCount: 0,
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
      scenarios: [],
      totalCount: 0,
      message: '',
      error: 'Error reading feature files',
    };
  }

  if (files.length === 0) {
    return {
      success: true,
      scenarios: [],
      totalCount: 0,
      message: 'No feature files found in spec/features/',
    };
  }

  const scenarios: ScenarioInfo[] = [];
  const warnings: string[] = [];

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
        warnings.push(`Skipping invalid file: ${file}`);
        continue;
      }

      if (!gherkinDocument.feature) {
        continue;
      }

      // Check if feature matches tags (if tags specified)
      if (tags.length > 0) {
        const featureTags = gherkinDocument.feature.tags.map(t => t.name);
        const matchesAllTags = tags.every(tag => featureTags.includes(tag));

        if (!matchesAllTags) {
          continue;
        }
      }

      // Extract scenarios from this feature
      for (const child of gherkinDocument.feature.children) {
        if (child.scenario && child.scenario.keyword === 'Scenario') {
          scenarios.push({
            feature: file,
            name: child.scenario.name,
            line: child.scenario.location.line,
          });
        }
      }
    } catch (error: any) {
      warnings.push(`Error reading ${file}: ${error.message}`);
    }
  }

  const totalCount = scenarios.length;

  let message = '';
  if (totalCount === 0 && tags.length > 0) {
    message = `No scenarios found matching tags: ${tags.join(', ')}`;
  } else if (totalCount === 0) {
    message = 'No scenarios found';
  } else if (tags.length > 0) {
    message = `Found ${totalCount} scenario${totalCount === 1 ? '' : 's'} matching tags: ${tags.join(', ')}`;
  } else {
    message = `Found ${totalCount} scenario${totalCount === 1 ? '' : 's'}`;
  }

  return {
    success: true,
    scenarios,
    totalCount,
    message,
    warnings: warnings.length > 0 ? warnings : undefined,
  };
}

export async function getScenariosCommand(options: {
  tag?: string | string[];
  format?: string;
}): Promise<void> {
  const tags = Array.isArray(options.tag)
    ? options.tag
    : options.tag
      ? [options.tag]
      : [];
  const format = (options.format as 'text' | 'json') || 'text';

  try {
    const result = await getScenarios({ tags, format });

    if (!result.success) {
      console.error(chalk.red('Error:'), result.error);
      process.exit(1);
    }

    if (result.warnings && result.warnings.length > 0) {
      for (const warning of result.warnings) {
        console.warn(chalk.yellow('⚠'), warning);
      }
    }

    if (format === 'json') {
      console.log(JSON.stringify(result.scenarios, null, 2));
    } else {
      console.log(chalk.blue(result.message));

      if (result.scenarios.length > 0) {
        console.log('');

        // Group by feature
        const byFeature = new Map<string, ScenarioInfo[]>();
        for (const scenario of result.scenarios) {
          if (!byFeature.has(scenario.feature)) {
            byFeature.set(scenario.feature, []);
          }
          byFeature.get(scenario.feature)!.push(scenario);
        }

        for (const [feature, scenarios] of byFeature.entries()) {
          console.log(chalk.bold.green(feature));
          for (const scenario of scenarios) {
            console.log(chalk.gray(`  ${scenario.line}:`), scenario.name);
          }
          console.log('');
        }
      }
    }

    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
