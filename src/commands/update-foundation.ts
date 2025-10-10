import { readFile, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import type { Foundation } from '../types/foundation';
import { validateFoundationJson } from '../validators/json-schema';
import { generateFoundationMd } from '../generators/foundation-md';

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

// Minimal foundation.json template
const FOUNDATION_JSON_TEMPLATE: Foundation = {
  project: {
    name: 'Project',
    description: 'Project description',
    repository: 'https://github.com/user/repo',
    license: 'MIT',
    importantNote: 'Important project note',
  },
  whatWeAreBuilding: {
    projectOverview: 'Project overview',
    technicalRequirements: {
      coreTechnologies: [],
      architecture: {
        pattern: 'Architecture pattern',
        fileStructure: 'File structure',
        deploymentTarget: 'Deployment target',
        integrationModel: [],
      },
      developmentAndOperations: {
        developmentTools: 'Development tools',
        testingStrategy: 'Testing strategy',
        logging: 'Logging approach',
        validation: 'Validation approach',
        formatting: 'Formatting approach',
      },
      keyLibraries: [],
    },
    nonFunctionalRequirements: [],
  },
  whyWeAreBuildingIt: {
    problemDefinition: {
      primary: {
        title: 'Primary Problem',
        description: 'Description',
        points: [],
      },
      secondary: [],
    },
    painPoints: {
      currentState: 'Current state',
      specific: [],
    },
    stakeholderImpact: [],
    theoreticalSolutions: [],
    developmentMethodology: {
      name: 'Methodology',
      description: 'Description',
      steps: [],
      ensures: [],
    },
    successCriteria: [],
    constraintsAndAssumptions: {
      constraints: [],
      assumptions: [],
    },
  },
  architectureDiagrams: [],
  coreCommands: {
    categories: [],
  },
  featureInventory: {
    phases: [],
    tagUsageSummary: {
      phaseDistribution: [],
      componentDistribution: [],
      featureGroupDistribution: [],
      priorityDistribution: [],
      testingCoverage: [],
    },
  },
  notes: {
    developmentStatus: [],
  },
};

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
    const foundationJsonPath = join(cwd, 'spec/foundation.json');
    const foundationMdPath = join(cwd, 'spec/FOUNDATION.md');

    // Load or create foundation.json
    let foundationData: Foundation;

    if (existsSync(foundationJsonPath)) {
      const fileContent = await readFile(foundationJsonPath, 'utf-8');
      foundationData = JSON.parse(fileContent);
    } else {
      // Create spec directory and foundation.json from template
      await mkdir(join(cwd, 'spec'), { recursive: true });
      foundationData = JSON.parse(JSON.stringify(FOUNDATION_JSON_TEMPLATE));
    }

    // Update the JSON field based on section name
    const updated = updateJsonField(foundationData, section, content);
    if (!updated) {
      return {
        success: false,
        error: `Unknown section: "${section}". Use field names like: projectOverview, problemDefinition, etc.`,
      };
    }

    // Write updated foundation.json
    await writeFile(
      foundationJsonPath,
      JSON.stringify(foundationData, null, 2),
      'utf-8'
    );

    // Validate updated JSON against schema
    const validation = await validateFoundationJson(foundationJsonPath);
    if (!validation.valid) {
      return {
        success: false,
        error: `Updated foundation.json failed schema validation: ${validation.errors?.join(', ')}`,
      };
    }

    // Regenerate FOUNDATION.md from JSON
    const markdown = await generateFoundationMd(foundationData);
    await writeFile(foundationMdPath, markdown, 'utf-8');

    return {
      success: true,
      message: `Updated "${section}" section in FOUNDATION.md`,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

// Helper function to update JSON field based on section name
function updateJsonField(
  foundation: Foundation,
  section: string,
  content: string
): boolean {
  // Map section names to JSON field paths
  switch (section) {
    case 'projectOverview':
      foundation.whatWeAreBuilding.projectOverview = content;
      return true;

    case 'problemDefinition':
      foundation.whyWeAreBuildingIt.problemDefinition.primary.description =
        content;
      return true;

    case 'architecturePattern':
      foundation.whatWeAreBuilding.technicalRequirements.architecture.pattern =
        content;
      return true;

    case 'developmentTools':
      foundation.whatWeAreBuilding.technicalRequirements.developmentAndOperations.developmentTools =
        content;
      return true;

    case 'testingStrategy':
      foundation.whatWeAreBuilding.technicalRequirements.developmentAndOperations.testingStrategy =
        content;
      return true;

    case 'painPoints':
      foundation.whyWeAreBuildingIt.painPoints.currentState = content;
      return true;

    case 'methodology':
      foundation.whyWeAreBuildingIt.developmentMethodology.description =
        content;
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
    console.log(chalk.gray('  Updated: spec/foundation.json'));
    console.log(chalk.gray('  Regenerated: spec/FOUNDATION.md'));
    process.exit(0);
  } catch (error: any) {
    console.error(chalk.red('Error:'), error.message);
    process.exit(1);
  }
}
