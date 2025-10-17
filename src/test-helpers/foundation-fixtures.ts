/**
 * Test fixture helpers for generic foundation schema v2.0.0
 */

export interface GenericFoundation {
  version: string;
  project: {
    name: string;
    vision: string;
    projectType: string;
    repository?: string;
    license?: string;
  };
  problemSpace: {
    primaryProblem: {
      title: string;
      description: string;
      impact: 'high' | 'medium' | 'low';
    };
  };
  solutionSpace: {
    overview: string;
    capabilities: Array<{
      name: string;
      description: string;
    }>;
  };
  personas?: Array<{
    name: string;
    description: string;
    goals?: string[];
  }>;
  architectureDiagrams?: Array<{
    title: string;
    mermaidCode: string;
    description?: string;
  }>;
}

/**
 * Creates a minimal valid generic foundation fixture
 */
export function createMinimalFoundation(
  overrides?: Partial<GenericFoundation>
): GenericFoundation {
  return {
    version: '2.0.0',
    project: {
      name: 'Test Project',
      vision: 'A test project for validation',
      projectType: 'cli-tool',
    },
    problemSpace: {
      primaryProblem: {
        title: 'Test Problem',
        description: 'Test problem description',
        impact: 'high',
      },
    },
    solutionSpace: {
      overview: 'Test solution overview',
      capabilities: [
        {
          name: 'Test Capability',
          description: 'Test capability description',
        },
      ],
    },
    ...overrides,
  };
}

/**
 * Creates a complete valid generic foundation fixture with all optional fields
 */
export function createCompleteFoundation(): GenericFoundation {
  return {
    version: '2.0.0',
    project: {
      name: 'fspec',
      vision: 'CLI tool for managing Gherkin specs with ACDD',
      projectType: 'cli-tool',
      repository: 'https://github.com/sengac/fspec',
      license: 'MIT',
    },
    problemSpace: {
      primaryProblem: {
        title: 'Specification Management',
        description: 'AI agents need structured workflow for specifications',
        impact: 'high',
      },
    },
    solutionSpace: {
      overview: 'Gherkin-based specification management',
      capabilities: [
        {
          name: 'Gherkin Validation',
          description: 'Validate feature files using official Cucumber parser',
        },
        {
          name: 'Work Unit Management',
          description: 'Track work through Kanban states',
        },
      ],
    },
    personas: [
      {
        name: 'Developer using CLI',
        description: 'Uses fspec in terminal',
        goals: ['Manage specifications', 'Track work progress'],
      },
    ],
    architectureDiagrams: [
      {
        title: 'System Architecture',
        mermaidCode: 'graph TB\n  CLI-->Parser\n  Parser-->Validator',
        description: 'High-level system architecture',
      },
    ],
  };
}

/**
 * Creates a generic foundation with architecture diagrams for testing diagram features
 */
export function createFoundationWithDiagrams(): GenericFoundation {
  return {
    version: '2.0.0',
    project: {
      name: 'Test Project',
      vision: 'A test project with diagrams',
      projectType: 'cli-tool',
    },
    problemSpace: {
      primaryProblem: {
        title: 'Test Problem',
        description: 'Test problem description',
        impact: 'high',
      },
    },
    solutionSpace: {
      overview: 'Test solution overview',
      capabilities: [
        {
          name: 'Test Capability',
          description: 'Test capability description',
        },
      ],
    },
    architectureDiagrams: [
      {
        title: 'fspec System Context',
        mermaidCode: 'graph TB\n  AI[AI Agent]\n  FSPEC[fspec CLI]',
        description: 'System context diagram',
      },
    ],
  };
}
