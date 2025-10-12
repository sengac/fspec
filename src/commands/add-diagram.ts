import { readFile, writeFile, mkdir } from 'fs/promises';
import { join } from 'path';
import { existsSync } from 'fs';
import chalk from 'chalk';
import { JSDOM } from 'jsdom';
import type { Foundation } from '../types/foundation';
import { validateFoundationJson } from '../validators/json-schema';
import { generateFoundationMd } from '../generators/foundation-md';

interface AddDiagramOptions {
  section: string;
  title: string;
  code: string;
  cwd?: string;
}

interface AddDiagramResult {
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

/**
 * Validate Mermaid diagram syntax using mermaid.parse() with jsdom
 */
async function validateMermaidSyntax(code: string): Promise<{
  valid: boolean;
  error?: string;
}> {
  try {
    // Create a jsdom instance with a minimal HTML document
    const dom = new JSDOM('<!DOCTYPE html><html><body></body></html>', {
      runScripts: 'dangerously',
      resources: 'usable',
    });

    // Set up global DOM objects for mermaid
    const { window } = dom;
    const originalWindow = global.window;
    const originalDocument = global.document;

    global.window = window as any;
    global.document = window.document as any;
    Object.defineProperty(global, 'navigator', {
      value: window.navigator,
      configurable: true,
    });

    // Dynamically import mermaid after setting up the DOM
    const mermaid = (await import('mermaid')).default;

    // Initialize mermaid with minimal config
    mermaid.initialize({
      startOnLoad: false,
      securityLevel: 'strict',
    });

    // Use mermaid.parse() to validate syntax
    await mermaid.parse(code);

    // Clean up globals
    if (originalWindow) {
      global.window = originalWindow;
    } else {
      delete (global as any).window;
    }
    if (originalDocument) {
      global.document = originalDocument;
    } else {
      delete (global as any).document;
    }
    delete (global as any).navigator;

    return { valid: true };
  } catch (error: any) {
    // Clean up globals even on error
    delete (global as any).window;
    delete (global as any).document;
    delete (global as any).navigator;

    return {
      valid: false,
      error: error.message || 'Unknown Mermaid syntax error',
    };
  }
}

export async function addDiagram(
  options: AddDiagramOptions
): Promise<AddDiagramResult> {
  const { section, title, code, cwd = process.cwd() } = options;

  // Validate inputs
  if (!section || section.trim().length === 0) {
    return {
      success: false,
      error: 'Section name cannot be empty',
    };
  }

  if (!title || title.trim().length === 0) {
    return {
      success: false,
      error: 'Diagram title cannot be empty',
    };
  }

  if (!code || code.trim().length === 0) {
    return {
      success: false,
      error: 'Diagram code cannot be empty',
    };
  }

  // Validate Mermaid syntax
  const validation = await validateMermaidSyntax(code);
  if (!validation.valid) {
    return {
      success: false,
      error: `Invalid Mermaid syntax: ${validation.error}`,
    };
  }

  try {
    const foundationJsonPath = join(cwd, 'spec/foundation.json');
    const foundationMdPath = join(cwd, 'spec/FOUNDATION.md');

    // Load or create foundation.json
    let foundationData: Foundation;

    if (existsSync(foundationJsonPath)) {
      const content = await readFile(foundationJsonPath, 'utf-8');
      foundationData = JSON.parse(content);
    } else {
      // Create spec directory and foundation.json from template
      await mkdir(join(cwd, 'spec'), { recursive: true });
      foundationData = JSON.parse(JSON.stringify(FOUNDATION_JSON_TEMPLATE));
    }

    // Find existing diagram with same title or add new one
    const existingIndex = foundationData.architectureDiagrams.findIndex(
      d => d.title === title
    );

    const newDiagram = {
      section,
      title,
      mermaidCode: code,
    };

    if (existingIndex !== -1) {
      // Replace existing diagram
      foundationData.architectureDiagrams[existingIndex] = newDiagram;
    } else {
      // Add new diagram
      foundationData.architectureDiagrams.push(newDiagram);
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
      message:
        existingIndex !== -1
          ? `Updated diagram "${title}"`
          : `Added diagram "${title}"`,
    };
  } catch (error: any) {
    return {
      success: false,
      error: error.message,
    };
  }
}

export async function addDiagramCommand(
  section: string,
  title: string,
  code: string
): Promise<void> {
  try {
    const result = await addDiagram({
      section,
      title,
      code,
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
