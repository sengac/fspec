/**
 * Automated Discovery - Code Analysis Guidance
 *
 * This file contains guidance prompts for AI agents to discover project
 * information from existing codebases. These prompts guide AI to infer:
 * - Project type (web app, CLI, library, service, mobile, desktop, API)
 * - Personas from user-facing interactions
 * - Capabilities (WHAT, not HOW)
 * - Problems/pain points (WHY, not implementation details)
 *
 * IMPORTANT: This is GUIDANCE for AI, not code implementation.
 * fspec provides prompts; AI decides how to analyze code.
 */

/**
 * Guidance for discovering CLI tools from commander.js structure
 */
export const cliToolDiscoveryGuidance = {
  pattern: {
    files: ['package.json', 'src/**/*.ts', 'src/**/*.js'],
    indicators: [
      'commander',
      '.command(',
      '.option(',
      'program.parse',
      'bin field in package.json',
    ],
  },
  inference: {
    projectType: 'cli-tool',
    persona: 'Developer using CLI in terminal',
    goals: ['Automate tasks', 'Integrate with CI/CD', 'Run commands'],
  },
  prompt: `
Analyze the codebase for CLI tool patterns:

1. Check package.json for "bin" field (indicates CLI entry point)
2. Look for commander.js imports (.command, .option, program.parse)
3. Search for command definitions and subcommands
4. Identify command-line flags and options

Infer:
- Project Type: 'cli-tool'
- Persona: 'Developer using CLI in terminal'
- Capabilities: Extract from command names (e.g., 'validate', 'format', 'list-features')
- Problems: What manual tasks does this CLI automate?
`,
};

/**
 * Guidance for discovering web applications from Express and React
 */
export const webAppDiscoveryGuidance = {
  pattern: {
    files: ['package.json', 'src/**/*.tsx', 'src/**/*.jsx', 'src/**/*.ts'],
    indicators: [
      'express',
      'app.get(',
      'app.post(',
      'React.Component',
      'useState',
      'useEffect',
      'routes/',
    ],
  },
  inference: {
    projectType: 'web-app',
    personas: [
      {
        name: 'End User',
        source: 'UI components',
        goals: ['Complete tasks', 'Access features', 'View information'],
      },
      {
        name: 'API Consumer',
        source: 'API routes',
        goals: ['Access data programmatically', 'Build integrations'],
      },
    ],
  },
  prompt: `
Analyze the codebase for web application patterns:

1. Check for Express routes (app.get, app.post, router.use)
2. Look for React components (useState, useEffect, JSX)
3. Identify API endpoints (/api/*, REST patterns)
4. Find UI components (forms, dashboards, navigation)

Infer:
- Project Type: 'web-app'
- Persona 'End User' from: UI components, pages, forms
- Persona 'API Consumer' from: API routes, REST endpoints
- Capabilities: High-level features from routes and components
- Problems: What user needs does this web app solve?
`,
};

/**
 * Guidance for discovering libraries from package.json exports
 */
export const libraryDiscoveryGuidance = {
  pattern: {
    files: ['package.json', 'src/index.ts', 'README.md'],
    indicators: [
      'exports field in package.json',
      'main field in package.json',
      'public API exports',
      'index.ts with exports',
    ],
  },
  inference: {
    projectType: 'library',
    persona: 'Developer integrating library into their codebase',
    goals: [
      'Easy integration',
      'Type-safe API',
      'Good documentation',
      'Reusable functionality',
    ],
  },
  prompt: `
Analyze the codebase for library patterns:

1. Check package.json for "exports" or "main" field
2. Look for index.ts/index.js with public exports
3. Identify exported functions, classes, types
4. Check for TypeScript definitions (.d.ts files)

Infer:
- Project Type: 'library'
- Persona: 'Developer integrating library into their codebase'
- Capabilities: What functionality does the library provide?
- Problems: What developer pain points does this library solve?
`,
};

/**
 * Guidance for inferring capabilities (WHAT, not HOW)
 */
export const capabilityInferenceGuidance = {
  principle: 'Focus on WHAT the system does, not HOW it does it',
  examples: {
    good: [
      'User Authentication (WHAT)',
      'Data Visualization (WHAT)',
      'Real-time Updates (WHAT)',
      'User Interface (WHAT)',
    ],
    bad: [
      'Uses React hooks (HOW)',
      'JWT authentication with bcrypt (HOW)',
      'D3.js charting library (HOW)',
      'WebSocket connections (HOW)',
    ],
  },
  prompt: `
Extract high-level capabilities from code structure:

1. Look at routes, commands, or exported functions
2. Identify USER-FACING features (not implementation details)
3. Group granular features into broad capabilities (3-7 items)
4. Focus on WHAT the system does, not HOW

Example: React components → 'User Interface' (NOT 'Uses React hooks')
Example: Auth routes → 'User Authentication' (NOT 'JWT with bcrypt')
Example: WebSocket code → 'Real-time Updates' (NOT 'WebSocket connections')

Capabilities should answer: What can users DO with this system?
`,
};

/**
 * Guidance for inferring problems (WHY, not implementation details)
 */
export const problemInferenceGuidance = {
  principle: 'Focus on user needs (WHY), not technical implementation',
  examples: {
    good: [
      'Users need interactive web UI (WHY)',
      'Developers need to automate repetitive tasks (WHY)',
      'Teams need to collaborate in real-time (WHY)',
      'Users need secure access to protected features (WHY)',
    ],
    bad: [
      'Code needs React (HOW)',
      'System needs CLI tool (HOW)',
      'App needs WebSockets (HOW)',
      'Code needs JWT authentication (HOW)',
    ],
  },
  prompt: `
Infer user problems from code structure:

1. Look at what the code ENABLES users to do
2. Ask: WHY does this code exist? What problem does it solve?
3. Focus on USER NEEDS, not technical implementation choices
4. Frame problems from user/developer perspective

Example: React app → 'Users need interactive web UI' (NOT 'Code needs React')
Example: CLI tool → 'Developers need to automate tasks' (NOT 'System needs CLI')
Example: Auth code → 'Users need secure access' (NOT 'Code needs JWT')

Problems should answer: What user pain point is being solved?
`,
};

/**
 * Export all guidance for testing
 */
export const discoveryGuidance = {
  cliTools: cliToolDiscoveryGuidance,
  webApps: webAppDiscoveryGuidance,
  libraries: libraryDiscoveryGuidance,
  capabilities: capabilityInferenceGuidance,
  problems: problemInferenceGuidance,
};
