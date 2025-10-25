import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'suggest-dependencies',
  description:
    'Auto-suggest dependency relationships based on work unit metadata (sequential IDs, build/test pairs, infrastructure patterns)',
  usage: 'fspec suggest-dependencies [options]',
  whenToUse:
    'Use this command when setting up new work units or auditing existing relationships. Especially valuable after bulk work unit creation to quickly establish dependencies based on naming conventions and sequential IDs. Reduces manual relationship mapping effort.',
  prerequisites: ['spec/work-units.json exists with multiple work units'],
  arguments: [],
  options: [
    {
      flag: '--output <format>',
      description: 'Output format: json or text',
      defaultValue: 'text',
    },
  ],
  examples: [
    {
      command: 'fspec suggest-dependencies',
      description: 'Get dependency suggestions based on patterns',
      output:
        'Found 5 dependency suggestion(s):\n\n1. AUTH-002 → AUTH-001 (dependsOn)\n   ● sequential IDs in AUTH prefix suggest AUTH-002 depends on AUTH-001\n   Confidence: MEDIUM\n\n2. TEST-AUTH-001 → BUILD-AUTH-001 (dependsOn)\n   ● test work depends on build work: "Test authentication" depends on "Build authentication"\n   Confidence: HIGH\n\n3. FEAT-001 → SCHEMA-001 (dependsOn)\n   ● infrastructure work (schema/migration) should complete before feature work: "Add user features" depends on "Database schema setup"\n   Confidence: HIGH\n\nTo apply a suggestion: fspec add-dependency <from-id> --depends-on=<to-id>',
    },
    {
      command: 'fspec suggest-dependencies --output json',
      description: 'Output suggestions as JSON for automation',
      output:
        '{\n  "suggestions": [\n    {\n      "from": "AUTH-002",\n      "to": "AUTH-001",\n      "type": "dependsOn",\n      "reason": "sequential IDs in AUTH prefix suggest AUTH-002 depends on AUTH-001",\n      "confidence": "medium"\n    },\n    {\n      "from": "TEST-AUTH-001",\n      "to": "BUILD-AUTH-001",\n      "type": "dependsOn",\n      "reason": "test work depends on build work",\n      "confidence": "high"\n    }\n  ]\n}',
    },
    {
      command: 'fspec suggest-dependencies | grep HIGH',
      description: 'Show only high-confidence suggestions',
      output:
        '2. TEST-AUTH-001 → BUILD-AUTH-001 (dependsOn)\n   ● test work depends on build work: "Test authentication" depends on "Build authentication"\n   Confidence: HIGH',
    },
  ],
  commonErrors: [
    {
      error: 'Error: No dependency suggestions found',
      fix: 'No patterns detected (sequential IDs, build/test pairs, infrastructure keywords). Suggestions require consistent naming conventions.',
    },
    {
      error: 'Error: work-units.json not found',
      fix: 'Run: fspec init to create work-units.json file',
    },
    {
      error: 'Error: Invalid work-units.json format',
      fix: 'Check for JSON syntax errors in spec/work-units.json',
    },
  ],
  typicalWorkflow:
    '1. Create work units with consistent naming → 2. Run suggestions: fspec suggest-dependencies → 3. Review suggestions (prioritize HIGH confidence) → 4. Apply dependencies: fspec add-dependency <from-id> --depends-on=<to-id> → 5. Verify: fspec show-work-unit <id>',
  commonPatterns: [
    {
      pattern: 'Bulk Dependency Setup',
      example:
        '# Create work units with sequential IDs\nfspec create-story AUTH "Setup auth infrastructure"\nfspec create-story AUTH "Add login endpoint"\nfspec create-story AUTH "Add logout endpoint"\n\n# Get suggestions\nfspec suggest-dependencies\n\n# Apply HIGH confidence suggestions automatically\nfspec add-dependency AUTH-002 --depends-on AUTH-001\nfspec add-dependency AUTH-003 --depends-on AUTH-001',
    },
    {
      pattern: 'Test/Build Pattern Recognition',
      example:
        '# Create build and test work units\nfspec create-task BUILD "Build authentication module"\nfspec create-task TEST "Test authentication module"\n\n# Get suggestions (will detect test depends on build)\nfspec suggest-dependencies\n\n# Apply suggested dependency\nfspec add-dependency TEST-001 --depends-on BUILD-001',
    },
    {
      pattern: 'Infrastructure-First Pattern',
      example:
        '# Create schema and feature work\nfspec create-task SCHEMA "Database schema setup"\nfspec create-story ADD "Add user management features"\n\n# Get suggestions (will detect feature depends on schema)\nfspec suggest-dependencies\n\n# Apply infrastructure dependency\nfspec add-dependency ADD-001 --depends-on SCHEMA-001',
    },
  ],
  relatedCommands: [
    'add-dependency',
    'query-bottlenecks',
    'query-orphans',
    'remove-dependency',
    'show-work-unit',
    'validate-dependencies',
  ],
  notes: [
    'Suggestion Rules:',
    '  1. Sequential IDs: AUTH-002 depends on AUTH-001 (same prefix)',
    '  2. Build/Test pairs: "Test X" depends on "Build X"',
    '  3. Infrastructure-first: Features depend on schema/migration work',
    '  4. Same epic: Suggests relatesTo (low confidence)',
    '  5. Circular dependencies are automatically filtered out',
    'Confidence levels:',
    '  HIGH: Build/test pairs, infrastructure patterns (apply immediately)',
    '  MEDIUM: Sequential IDs (review before applying)',
    'Existing dependencies are excluded from suggestions',
    'Specific patterns (build/test, infrastructure) override generic sequential suggestions',
  ],
};

export default config;
