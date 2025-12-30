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
      command:
        'fspec research --tool=ast --pattern="async function $NAME" --lang=typescript --path=src/',
      description: 'Search codebase for async functions using AST analysis',
      output: `src/services/api.ts:15:1:async function fetchUser(id: string) {
src/services/auth.ts:42:1:async function validateToken(token: string) {
src/utils/cache.ts:8:1:async function getCacheValue(key: string) {`,
    },
    {
      command:
        'fspec research --tool=ast --pattern="function $NAME" --lang=typescript --path=src/auth.ts',
      description: 'Find all functions in a specific file',
      output: `src/auth.ts:12:1:function login(username: string, password: string) {
src/auth.ts:28:1:function logout() {
src/auth.ts:45:1:function validateCredentials(user: User) {`,
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
        'fspec research --tool=ast --refactor --pattern="const MyComponent" --lang=tsx --source=src/big-file.tsx --target=src/components/MyComponent.tsx',
      description: 'Move a component to its own file using AST refactoring',
      output: `{
  "success": true,
  "movedCode": "const MyComponent = () => { ... }",
  "sourceFile": "src/big-file.tsx",
  "targetFile": "src/components/MyComponent.tsx"
}`,
    },
    {
      command:
        'fspec research --tool=ast --pattern="interface $NAME" --lang=typescript --path=src/types/',
      description: 'Find all TypeScript interfaces in a directory',
      output: `src/types/user.ts:5:1:interface User {
src/types/user.ts:12:1:interface UserProfile {
src/types/auth.ts:3:1:interface AuthToken {
src/types/auth.ts:18:1:interface Session {`,
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
      error: 'Error: --pattern is required',
      cause: 'AST tool requires a pattern to search for',
      solution:
        'Provide pattern like --pattern="function $NAME" (use $NAME for single-node wildcard, $$$ARGS for multi-node)',
    },
    {
      error: 'Error: --lang is required',
      cause: 'AST tool requires a language to parse',
      solution:
        'Provide language like --lang=typescript (supports: typescript, tsx, javascript, rust, python, go, java, c, cpp, ruby, etc.)',
    },
    {
      error: 'Pattern matched N nodes. Refactor requires exactly 1 match.',
      cause: 'Refactor mode can only move one code block at a time',
      solution:
        'Make your pattern more specific to match exactly one code block',
    },
  ],
  commonPatterns: [
    {
      title: 'Find functions by pattern',
      description:
        'Search for function declarations using AST pattern matching',
      example:
        'fspec research --tool=ast --pattern="function $NAME" --lang=typescript --path=src/',
    },
    {
      title: 'Find async functions',
      description: 'Search for async function declarations',
      example:
        'fspec research --tool=ast --pattern="async function $NAME" --lang=typescript --path=src/',
    },
    {
      title: 'Find classes or interfaces',
      description: 'Search for class or interface definitions',
      example:
        'fspec research --tool=ast --pattern="class $NAME" --lang=typescript --path=src/',
    },
    {
      title: 'Refactor: Move code to new file',
      description: 'Extract a component or function to its own file',
      example:
        'fspec research --tool=ast --refactor --pattern="const MyComponent" --lang=tsx --source=src/big.tsx --target=src/MyComponent.tsx',
    },
    {
      title: 'Stakeholder questions during discovery',
      description: 'Send questions to Teams/Slack and attach responses',
      example:
        'fspec research --tool=stakeholder --platform=teams,slack --question="Need OAuth?" --work-unit=AUTH-001',
    },
  ],
};

export default config;
