import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'research',
  description:
    'Execute research tools to answer questions during Example Mapping',
  usage: 'fspec research [--tool=<name>] [tool-specific-args...]',
  whenToUse:
    'Use during specifying phase when you have questions that require external research (AST analysis, stakeholder input, API queries, etc.). Research tools help answer red card questions during Example Mapping.',
  options: [
    {
      name: '--tool <name>',
      description:
        'Research tool to execute (ast, perplexity, confluence, jira, stakeholder). Omit to list available tools.',
      required: false,
    },
    {
      name: '--work-unit <id>',
      description:
        'Work unit context (forwarded to tool). Enables attaching results as attachments.',
      required: false,
    },
    {
      name: '[tool-specific-args]',
      description:
        'Additional arguments passed directly to the research tool. See tool --help for details.',
      required: false,
    },
  ],
  examples: [
    {
      command: 'fspec research',
      description: 'List all available research tools',
      output: `Available Research Tools:

  ast
    Usage: fspec research --tool=ast <args>

  stakeholder
    Usage: fspec research --tool=stakeholder <args>`,
    },
    {
      command: 'fspec research --tool=ast --query "find all async functions"',
      description: 'Search codebase for async functions using AST analysis',
      output: `{
  "matches": [
    {
      "file": "src/example.ts",
      "startLine": 10,
      "endLine": 15,
      "code": "async function fetchData() { ... }",
      "nodeType": "async_function"
    }
  ],
  "stats": {
    "filesScanned": 42,
    "matchCount": 1
  }
}`,
    },
    {
      command: 'fspec research --tool=ast --file "src/broken.ts"',
      description: 'Analyze specific file with AST parser',
      output: `{
  "partialAST": { "type": "program", "children": [...] },
  "errors": [
    { "line": 10, "message": "Unexpected token" }
  ]
}`,
    },
    {
      command:
        'fspec research --tool=stakeholder --platform=teams --question="Should we support OAuth2?" --work-unit=AUTH-001',
      description: 'Send question to stakeholders via Teams',
      output: `# Stakeholder Question for AUTH-001

**Platform:** teams
**Question:** Should we support OAuth2?

**Work Unit Context:**
- ID: AUTH-001
- Title: User Login
- Epic: user-management

**Rules:**
1. Password must be 8+ chars

Message sent successfully.

Attach research results to work unit? (y/n):`,
    },
    {
      command:
        'fspec research --tool=ast --query "functions with more than 5 parameters"',
      description: 'Find complex functions with many parameters',
      output: `{
  "matches": [
    {
      "file": "src/example.ts",
      "functionName": "complexFunc",
      "parameterCount": 6
    }
  ]
}`,
    },
  ],
  notes: [
    'Research tools are auto-discovered from spec/research-scripts/ directory',
    'Any executable file in that directory becomes a research tool',
    'Tools can be written in any language (shell, Node.js, Python, etc.)',
    'Use <tool> --help to see tool-specific options',
    'Results can be attached to work units during discovery',
    'Attachments are saved to spec/attachments/<work-unit-id>/',
  ],
  relatedCommands: [
    'add-question',
    'answer-question',
    'add-attachment',
    'generate-scenarios',
  ],
  typicalWorkflow: [
    'During Example Mapping, identify a question that needs research',
    'Add question: fspec add-question AUTH-001 "@human: Support OAuth?"',
    'Research the question: fspec research --tool=stakeholder --platform=teams --question="Support OAuth?" --work-unit=AUTH-001',
    'When prompted, attach results to work unit (y)',
    'Review attached research in spec/attachments/AUTH-001/',
    'Answer question based on research: fspec answer-question AUTH-001 0 --answer "Yes" --add-to rule',
    'Generate scenarios: fspec generate-scenarios AUTH-001',
  ],
  commonErrors: [
    {
      error: 'Error: Research tool not found: xyz',
      cause: 'Tool does not exist in spec/research-scripts/',
      solution:
        "Run 'fspec research' to list available tools, or create custom tool",
    },
    {
      error: 'Error: FSPEC_TEST_MODE not set',
      cause: 'Tool requires test mode but environment variable not configured',
      solution: 'Set FSPEC_TEST_MODE=1 or run tool in production mode',
    },
    {
      error: 'Permission denied',
      cause: 'Research script not executable',
      solution: 'chmod +x spec/research-scripts/<tool-name>',
    },
  ],
  commonPatterns: [
    {
      title: 'AST-based code search',
      description: 'Find code patterns using AST instead of regex',
      example:
        'fspec research --tool=ast --query "find all exported functions"',
    },
    {
      title: 'Stakeholder questions during discovery',
      description: 'Send questions to Teams/Slack and attach responses',
      example:
        'fspec research --tool=stakeholder --platform=teams,slack --question="Need OAuth?" --work-unit=AUTH-001',
    },
    {
      title: 'Analyze broken code',
      description: 'Use AST to identify syntax errors in specific files',
      example: 'fspec research --tool=ast --file "src/problematic.ts"',
    },
  ],
};

export default config;
