# Help System Architecture Analysis - BUG-074

## Executive Summary

This document proposes a standardized, DRY (Don't Repeat Yourself) help system architecture that unifies the help structure across both fspec commands and research tools. Currently, research tools use ad-hoc string formatting in their `help()` methods, while fspec commands use a structured `CommandHelpConfig` approach with the `help-formatter` utility. This inconsistency violates DRY principles and makes maintenance difficult.

**Primary Issue (BUG-074)**: `fspec research --tool=ast --help` fails because the `--help` flag is being checked AFTER tool validation, causing "Research tool 'ast' not found" error instead of displaying help.

---

## Current State Analysis

### Research Tool Plugin System

fspec supports **two types of research tools**:

1. **Bundled Tools** (in `src/research-tools/`)
   - Shipped with fspec (ast, jira, perplexity, confluence, stakeholder)
   - TypeScript source files compiled during build
   - Registered in `BUNDLED_TOOLS` map in `registry.ts`

2. **Custom Tools** (in `spec/research-tools/`)
   - User-created plugins loaded dynamically at runtime
   - JavaScript files (`.js`) loaded via dynamic `import()`
   - Must implement `ResearchTool` interface
   - Must export as `default` or named `tool` export

**Loading Order** (`registry.ts:40-92`):
```typescript
export async function getResearchTool(toolName: string, cwd: string): Promise<ResearchTool> {
  // 1. Check bundled tools first
  const bundledTool = BUNDLED_TOOLS.get(toolName);
  if (bundledTool) return bundledTool;

  // 2. Try loading custom tool from spec/research-tools/<toolName>.js
  const customToolPath = join(cwd, 'spec', 'research-tools', `${toolName}.js`);
  const module = await import(customToolPath);

  // 3. Validate tool implements interface
  const tool = module.default || module.tool;
  if (!tool.name || !tool.description || !tool.execute || !tool.help) {
    throw new Error('Custom tool does not implement ResearchTool interface');
  }

  return tool;
}
```

**Key Point**: Both bundled and custom tools use the SAME `ResearchTool` interface, meaning **the proposed help system works for both equally**.

---

### Command Help System (Well-Structured)

**Location**: `src/utils/help-formatter.ts`

**Interface**: `CommandHelpConfig`
```typescript
export interface CommandHelpConfig {
  name: string;
  description: string;
  usage?: string;
  arguments?: CommandArgument[];
  options?: CommandOption[];
  examples?: CommandExample[];
  relatedCommands?: string[];
  whenToUse?: string;
  whenNotToUse?: string;
  prerequisites?: string[];
  commonPatterns?: string[] | CommonPattern[];
  typicalWorkflow?: string;
  commonErrors?: Array<{ error: string; fix: string }>;
  notes?: string[];
}
```

**Formatter**: `formatCommandHelp(config: CommandHelpConfig): string`

**Example Usage** (`src/commands/add-rule-help.ts`):
```typescript
const config: CommandHelpConfig = {
  name: 'add-rule',
  description: 'Add a business rule to a work unit during Example Mapping',
  usage: 'fspec add-rule <workUnitId> <rule>',
  whenToUse: 'Use during specifying phase when capturing business rules...',
  arguments: [
    { name: 'workUnitId', description: 'Work unit ID', required: true },
    { name: 'rule', description: 'Business rule description', required: true },
  ],
  examples: [
    {
      command: 'fspec add-rule AUTH-001 "Email must be valid format"',
      description: 'Add business rule',
      output: '✓ Rule added successfully',
    },
  ],
  relatedCommands: ['add-question', 'add-example', 'generate-scenarios'],
};
```

**Benefits**:
- ✅ Structured, type-safe configuration
- ✅ Consistent formatting across all commands
- ✅ AI-optimized sections (WHEN TO USE, PREREQUISITES, COMMON ERRORS)
- ✅ DRY - formatter logic centralized
- ✅ Easy to maintain and extend

---

### Research Tool Help System (Unstructured)

**Location**: `src/research-tools/types.ts`

**Interface**: `ResearchTool`
```typescript
export interface ResearchTool {
  name: string;
  description: string;
  execute(args: string[]): Promise<string>;
  help(): string;  // ⚠️ Returns hand-crafted string
}
```

**Current Implementation** (`src/research-tools/ast.ts`):
```typescript
help(): string {
  return `AST RESEARCH TOOL

Research code structure using AST parsing during Example Mapping.

USAGE
  ast --query <query> [options]
  ast --file <path> [options]

OPTIONS
  --query <query>     Natural language query for pattern detection (required if no --file)
  --file <path>       Specific file to analyze (required if no --query)
  --format <type>     Output format: json, markdown, text (default: json)
  --language <lang>   Language filter: typescript, python, go, rust, etc.
  --help              Show this help message

QUERY EXAMPLES
  ast --query "find all async functions"
  ast --query "functions with more than 5 parameters"
  ast --query "classes implementing interface UserRepository"

FILE EXAMPLES
  ast --file "src/broken.ts"
  ast --file "src/auth/login.ts"

FEATURES
  - AST parsing using tree-sitter (supports 40+ languages)
  - Pattern detection across TypeScript, JavaScript, Python, Go, Rust, Java, C++
  - Error-tolerant parsing (analyzes incomplete or broken code)

EXIT CODES
  0  Success
  1  Missing required flag (--query or --file)
  2  File not found or parsing error
  3  Invalid query or unsupported language`;
}
```

**Problems**:
- ❌ Hand-crafted string formatting in EVERY tool (violates DRY)
- ❌ No type safety for help structure
- ❌ Inconsistent formatting across tools (some use chalk, some don't)
- ❌ Missing AI-optimized sections (WHEN TO USE, PREREQUISITES, COMMON ERRORS)
- ❌ Hard to maintain - changes require editing raw strings
- ❌ No validation of help structure completeness
- ❌ Duplicated formatting logic in each tool (jira.ts, ast.ts, perplexity.ts, etc.)

---

### The BUG-074 Root Cause

**Problem Flow**:
```
1. User runs: fspec research --tool=ast --help
2. research.ts:246 - getResearchTool(options.tool, cwd) is called
3. getResearchTool() throws: "Research tool 'ast' not found"
4. Error occurs BEFORE --help flag is checked (line 249)
```

**Current Code** (`src/commands/research.ts:246-252`):
```typescript
// Load and execute tool
const tool = await getResearchTool(options.tool, cwd);  // ⚠️ THROWS HERE

// Check if --help is requested
if (forwardedArgs.includes('--help') || forwardedArgs.includes('-h')) {
  console.log(tool.help());
  return;
}
```

**Why It Fails**:
- `getResearchTool()` validates tool existence before `--help` check
- If tool name is invalid (or registry issue), it throws immediately
- `--help` flag never gets processed

---

## Proposed Solution Architecture

### Phase 1: Unified Help Configuration System

#### 1.1 Create `ResearchToolHelpConfig` Interface

**Location**: `src/utils/help-formatter.ts` (extend existing file)

```typescript
/**
 * Help configuration for research tools
 * Subset of CommandHelpConfig tailored for research tools
 */
export interface ResearchToolHelpConfig {
  name: string;
  description: string;
  usage?: string;
  whenToUse?: string;
  whenNotToUse?: string;
  prerequisites?: string[];
  options?: CommandOption[];
  examples?: CommandExample[];
  commonErrors?: Array<{ error: string; fix: string }>;
  exitCodes?: Array<{ code: number; description: string }>;
  configuration?: {
    required: boolean;
    location: string;
    example: string;
    instructions?: string;
  };
  features?: string[];
  notes?: string[];
}
```

**Why This Design?**:
- ✅ Subset of `CommandHelpConfig` - reuses existing types
- ✅ Adds research-tool-specific fields: `exitCodes`, `configuration`, `features`
- ✅ Removes command-specific fields: `arguments` (tools use `options` only)
- ✅ Type-safe, structured approach

#### 1.2 Create `formatResearchToolHelp()` Utility

**Location**: `src/utils/help-formatter.ts`

```typescript
/**
 * Format research tool help configuration into display string
 * Uses similar structure to formatCommandHelp but optimized for tools
 */
export function formatResearchToolHelp(config: ResearchToolHelpConfig): string {
  const lines: string[] = [];

  // Header
  lines.push('');
  lines.push(chalk.bold.cyan(`${config.name.toUpperCase()} RESEARCH TOOL`));
  lines.push(chalk.dim(config.description));
  lines.push('');

  // When to use (AI-optimized)
  if (config.whenToUse) {
    lines.push(chalk.bold('WHEN TO USE'));
    lines.push(`  ${config.whenToUse}`);
    lines.push('');
  }

  // When NOT to use (AI-optimized)
  if (config.whenNotToUse) {
    lines.push(chalk.bold('WHEN NOT TO USE'));
    lines.push(`  ${config.whenNotToUse}`);
    lines.push('');
  }

  // Prerequisites (AI-optimized)
  if (config.prerequisites && config.prerequisites.length > 0) {
    lines.push(chalk.bold('PREREQUISITES'));
    config.prerequisites.forEach(prereq => {
      lines.push(`  • ${prereq}`);
    });
    lines.push('');
  }

  // Usage
  lines.push(chalk.bold('USAGE'));
  const usageLine = config.usage || `fspec research --tool=${config.name} [options]`;
  lines.push(`  ${chalk.cyan(usageLine)}`);
  lines.push('');

  // Options
  if (config.options && config.options.length > 0) {
    lines.push(chalk.bold('OPTIONS'));
    config.options.forEach(opt => {
      lines.push(`  ${chalk.green(opt.flag)}`);
      lines.push(`    ${opt.description}`);
      if (opt.defaultValue) {
        lines.push(`    ${chalk.dim(`Default: ${opt.defaultValue}`)}`);
      }
    });
    lines.push('');
  }

  // Configuration (research tool specific)
  if (config.configuration) {
    lines.push(chalk.bold('CONFIGURATION'));
    if (config.configuration.required) {
      lines.push(chalk.red('  Required'));
    }
    lines.push(`  ${chalk.dim('Location:')} ${config.configuration.location}`);
    if (config.configuration.instructions) {
      lines.push(`  ${config.configuration.instructions}`);
    }
    if (config.configuration.example) {
      lines.push('');
      lines.push(chalk.dim('  Example:'));
      lines.push(`  ${chalk.dim(config.configuration.example)}`);
    }
    lines.push('');
  }

  // Examples
  if (config.examples && config.examples.length > 0) {
    lines.push(chalk.bold('EXAMPLES'));
    config.examples.forEach((example, index) => {
      if (example.description) {
        lines.push(`  ${chalk.dim(`${index + 1}. ${example.description}`)}`);
      }
      lines.push(`  ${chalk.cyan(`$ fspec research --tool=${config.name} ${example.command}`)}`);
      if (example.output) {
        lines.push(`  ${chalk.dim(example.output)}`);
      }
      if (index < config.examples!.length - 1) {
        lines.push('');
      }
    });
    lines.push('');
  }

  // Features (research tool specific)
  if (config.features && config.features.length > 0) {
    lines.push(chalk.bold('FEATURES'));
    config.features.forEach(feature => {
      lines.push(`  • ${feature}`);
    });
    lines.push('');
  }

  // Common Errors (AI-optimized)
  if (config.commonErrors && config.commonErrors.length > 0) {
    lines.push(chalk.bold('COMMON ERRORS'));
    config.commonErrors.forEach(err => {
      lines.push(`  ${chalk.red('✗')} ${chalk.bold(err.error)}`);
      lines.push(`    ${chalk.green('Fix:')} ${err.fix}`);
      lines.push('');
    });
  }

  // Exit Codes (research tool specific)
  if (config.exitCodes && config.exitCodes.length > 0) {
    lines.push(chalk.bold('EXIT CODES'));
    config.exitCodes.forEach(exit => {
      const codeColor = exit.code === 0 ? chalk.green : chalk.yellow;
      lines.push(`  ${codeColor(exit.code.toString())}  ${exit.description}`);
    });
    lines.push('');
  }

  // Notes
  if (config.notes && config.notes.length > 0) {
    lines.push(chalk.bold('NOTES'));
    config.notes.forEach(note => {
      lines.push(`  • ${note}`);
    });
    lines.push('');
  }

  return lines.join('\n');
}
```

**Why This Design?**:
- ✅ DRY - Single formatter for all research tools
- ✅ Consistent with existing `formatCommandHelp()` structure
- ✅ Adds research-tool-specific sections (CONFIGURATION, FEATURES, EXIT CODES)
- ✅ AI-optimized sections match command help system
- ✅ Colored output using chalk for readability

---

### Phase 2: Refactor Research Tool Interface

#### 2.1 Update `ResearchTool` Interface

**Location**: `src/research-tools/types.ts`

**Before**:
```typescript
export interface ResearchTool {
  name: string;
  description: string;
  execute(args: string[]): Promise<string>;
  help(): string;  // ⚠️ Hand-crafted string
}
```

**After**:
```typescript
import type { ResearchToolHelpConfig } from '../utils/help-formatter';

export interface ResearchTool {
  name: string;
  description: string;
  execute(args: string[]): Promise<string>;

  /**
   * Get help configuration for this tool
   * Returns structured configuration for standardized help display
   */
  getHelpConfig(): ResearchToolHelpConfig;
}
```

**Why This Design?**:
- ✅ Structured config instead of strings
- ✅ Clean interface - no deprecated methods
- ✅ Type-safe help configuration
- ✅ Enforces completeness via TypeScript compiler

---

### Phase 3: Implementation Strategy

#### 3.1 Create Helper Function for Tool Help Display

**Location**: `src/utils/help-formatter.ts`

```typescript
/**
 * Display research tool help
 * Works for BOTH bundled tools (src/research-tools/) AND custom tools (spec/research-tools/)
 */
export function displayResearchToolHelp(tool: ResearchTool): void {
  const config = tool.getHelpConfig();
  const helpText = formatResearchToolHelp(config);
  console.log(helpText);
}
```

**Why This Works for Custom Tools:**

Custom tools are loaded via dynamic `import()` and MUST implement the `ResearchTool` interface (validated in `registry.ts:75-79`). Since both bundled and custom tools use the same interface:

1. ✅ Custom tools use `getHelpConfig()` method (required by interface)
2. ✅ Helper function works identically for bundled and custom tools
3. ✅ No special handling needed for custom vs bundled tools
4. ✅ Consistent formatting across all tools

#### 3.2 Refactor AST Tool (Example)

**Location**: `src/research-tools/ast.ts`

**Before**:
```typescript
help(): string {
  return `AST RESEARCH TOOL
...hand-crafted multi-line string...`;
}
```

**After**:
```typescript
import type { ResearchToolHelpConfig } from '../utils/help-formatter';

getHelpConfig(): ResearchToolHelpConfig {
  return {
    name: 'ast',
    description: 'AST code analysis tool for pattern detection and deep code analysis',
    usage: 'fspec research --tool=ast [options]',
    whenToUse: 'Use during Example Mapping to understand code structure, find patterns, or analyze existing implementations before writing specifications.',
    prerequisites: [
      'Codebase must contain TypeScript, JavaScript, Python, Go, Rust, or Java files',
      'Tree-sitter parsers are bundled (no additional setup required)',
    ],
    options: [
      {
        flag: '--query <query>',
        description: 'Natural language query for pattern detection (required if no --file)',
      },
      {
        flag: '--file <path>',
        description: 'Specific file to analyze (required if no --query)',
      },
      {
        flag: '--format <type>',
        description: 'Output format',
        defaultValue: 'json',
      },
      {
        flag: '--language <lang>',
        description: 'Language filter: typescript, python, go, rust, etc.',
      },
    ],
    examples: [
      {
        command: '--query "find all async functions"',
        description: 'Find all async function definitions',
        output: '{ "query": "...", "matches": [...] }',
      },
      {
        command: '--query "functions with more than 5 parameters"',
        description: 'Detect functions with high parameter count',
      },
      {
        command: '--file "src/auth/login.ts"',
        description: 'Analyze specific file structure',
      },
    ],
    features: [
      'AST parsing using tree-sitter (supports 40+ languages)',
      'Pattern detection across TypeScript, JavaScript, Python, Go, Rust, Java, C++',
      'Error-tolerant parsing (analyzes incomplete or broken code)',
      'Natural language queries converted to AST patterns',
    ],
    commonErrors: [
      {
        error: 'At least one of --query or --file is required',
        fix: 'Provide either --query "your query" or --file "path/to/file"',
      },
      {
        error: 'File not found or parsing error',
        fix: 'Check file path exists and is a valid source file',
      },
    ],
    exitCodes: [
      { code: 0, description: 'Success' },
      { code: 1, description: 'Missing required flag (--query or --file)' },
      { code: 2, description: 'File not found or parsing error' },
      { code: 3, description: 'Invalid query or unsupported language' },
    ],
  };
}
```

**Benefits**:
- ✅ Structured, type-safe configuration
- ✅ DRY - no duplicated formatting logic
- ✅ AI-optimized sections (WHEN TO USE, COMMON ERRORS)
- ✅ Consistent with command help system
- ✅ Easy to extend (add new sections to config)
- ✅ Clean interface - no deprecated methods

---

### Phase 4: Fix BUG-074 (--help Flag Handling)

#### 4.1 Move --help Check BEFORE Tool Loading

**Location**: `src/commands/research.ts`

**Before** (lines 246-252):
```typescript
// Load and execute tool
const tool = await getResearchTool(options.tool, cwd);  // ⚠️ THROWS HERE

// Check if --help is requested
if (forwardedArgs.includes('--help') || forwardedArgs.includes('-h')) {
  console.log(tool.help());
  return;
}
```

**After**:
```typescript
// Check if --help is requested BEFORE loading tool
if (forwardedArgs.includes('--help') || forwardedArgs.includes('-h')) {
  try {
    const tool = await getResearchTool(options.tool, cwd);
    displayResearchToolHelp(tool);
    return;
  } catch (error: any) {
    // If tool not found, show helpful error with available tools
    console.error(chalk.red(`Research tool '${options.tool}' not found\n`));
    console.error('Available research tools:');
    const toolsWithStatus = listResearchTools();
    for (const tool of toolsWithStatus) {
      console.log(`  ${tool.statusIndicator} ${tool.name} - ${tool.description}`);
    }
    console.error(`\nTry: fspec research --tool=<name> --help`);
    process.exit(1);
  }
}

// Load and execute tool
const tool = await getResearchTool(options.tool, cwd);
```

**Why This Design?**:
- ✅ Fixes BUG-074 - `--help` works even if tool has issues
- ✅ Shows helpful error with available tools if tool not found
- ✅ Separates help display from tool execution
- ✅ Better UX - users can see help before fixing config issues

---

## Implementation Checklist

### Phase 1: Foundation (1-2 hours)
- [ ] Create `ResearchToolHelpConfig` interface in `help-formatter.ts`
- [ ] Implement `formatResearchToolHelp()` function in `help-formatter.ts`
- [ ] Add unit tests for `formatResearchToolHelp()`
- [ ] Create `displayResearchToolHelp()` helper function in `help-formatter.ts`

### Phase 2: Interface Updates (15 min)
- [ ] Update `ResearchTool` interface to replace `help()` with `getHelpConfig()` method
- [ ] Update TypeScript types

### Phase 3: Tool Refactoring (2-3 hours)
- [ ] Refactor `ast.ts` to use `getHelpConfig()`
- [ ] Refactor `jira.ts` to use `getHelpConfig()`
- [ ] Refactor `perplexity.ts` to use `getHelpConfig()`
- [ ] Refactor `confluence.ts` to use `getHelpConfig()`
- [ ] Refactor `stakeholder.ts` to use `getHelpConfig()`

### Phase 4: Bug Fix (30 min)
- [ ] Move `--help` check BEFORE `getResearchTool()` in `research.ts`
- [ ] Add error handling for tool not found during help display
- [ ] Update error messages to be more helpful

### Phase 5: Testing (1 hour)
- [ ] Test `fspec research --tool=ast --help` (BUG-074 fix)
- [ ] Test help display for all bundled research tools
- [ ] Test error handling when tool not found
- [ ] Verify consistent formatting across all tools
- [ ] Test with custom tool (create simple test tool in spec/research-tools/)

### Phase 6: Documentation (30 min)
- [ ] Update research tool development guide
- [ ] Document `ResearchToolHelpConfig` structure
- [ ] Add examples for creating custom research tools
- [ ] Update CLAUDE.md with new help system

---

## Custom Tool Support (spec/research-tools/)

### How Custom Tools Work

Custom research tools are user-created plugins that extend fspec's research capabilities:

**Location**: `spec/research-tools/<tool-name>.js`

**Requirements**:
- Must be JavaScript (`.js` files) - TypeScript source files must be compiled first
- Must implement `ResearchTool` interface
- Must export as `default` or named `tool` export

**Example Custom Tool** (`spec/research-tools/github.js`):

```javascript
// Custom GitHub research tool
export const tool = {
  name: 'github',
  description: 'Search GitHub issues and pull requests',

  async execute(args) {
    // Tool implementation
    return JSON.stringify({ results: [] });
  },

  getHelpConfig() {
    return {
      name: 'github',
      description: 'Search GitHub issues and pull requests',
      usage: 'fspec research --tool=github [options]',
      whenToUse: 'Use during Example Mapping to research existing issues or PRs',
      options: [
        { flag: '--repo <name>', description: 'Repository name (owner/repo)' },
        { flag: '--query <text>', description: 'Search query' },
      ],
      examples: [
        {
          command: '--repo facebook/react --query "hooks"',
          description: 'Search React repo for hooks-related issues',
        },
      ],
      exitCodes: [
        { code: 0, description: 'Success' },
        { code: 1, description: 'Missing required arguments' },
      ],
    };
  }
};
```

### Benefits for Custom Tool Authors

With the proposed help system, custom tool authors get:

✅ **Consistent Formatting**:
- Custom tools get same professional formatting as bundled tools
- No need to manually format help strings
- AI-optimized sections (WHEN TO USE, PREREQUISITES, etc.)

✅ **Type Safety** (if using TypeScript):
- Import types from fspec: `import type { ResearchToolHelpConfig } from 'fspec/utils/help-formatter'`
- TypeScript compiler validates help config structure
- Autocomplete for all help sections

✅ **Simplified Interface**:
- Single method to implement: `getHelpConfig()`
- No confusion between old/new approaches
- Clean, modern API

---

## Benefits Summary

### Before (Current State)
- ❌ Hand-crafted help strings in every tool
- ❌ Duplicated formatting logic
- ❌ Inconsistent structure across tools
- ❌ No type safety for help content
- ❌ Missing AI-optimized sections
- ❌ BUG-074: `--help` fails for research tools

### After (Proposed Solution)
- ✅ Structured, type-safe help configuration
- ✅ DRY - single formatter for all tools
- ✅ Consistent formatting across all tools
- ✅ AI-optimized sections (WHEN TO USE, PREREQUISITES, COMMON ERRORS)
- ✅ Easy to maintain and extend
- ✅ BUG-074 fixed: `--help` works correctly
- ✅ Better error messages when tools not found

---

## Alternative Approaches Considered

### Alternative 1: Keep String-Based Help
**Rejected** because:
- Violates DRY principle
- No type safety
- Inconsistent formatting
- Hard to maintain
- Every tool duplicates formatting logic

### Alternative 2: Single HelpConfig for Both Commands and Tools
**Rejected** because:
- Commands and tools have different help needs
- Would require many optional fields
- Less type-safe
- Harder to understand
- Pollutes interface with irrelevant fields

### Alternative 3: Separate Help Files (Like Commands)
**Rejected** because:
- Research tools are plugins - help should be co-located with tool
- Adds unnecessary file overhead
- Breaks tool encapsulation
- Custom tools would need multiple files

### Alternative 4: Keep help() but Validate Format
**Rejected** because:
- Still allows inconsistent formatting
- No type safety
- Runtime validation instead of compile-time
- Doesn't solve DRY problem

---

## Risk Analysis

### Low Risk
- Breaking changes to `ResearchTool` interface
  - **Mitigation**: This is greenfield - no existing custom tools to break
  - **Impact**: None (interface not yet used externally)

### Low Risk
- Implementation takes longer than expected
  - **Mitigation**: Clear phases, can implement incrementally
  - **Impact**: Can complete one bundled tool at a time

### Low Risk
- Custom tool authors struggle with new interface
  - **Mitigation**: Comprehensive documentation and examples
  - **Impact**: Minimal, interface is straightforward and type-safe

---

## Success Metrics

- ✅ BUG-074 fixed: `fspec research --tool=ast --help` works
- ✅ All research tools use `getHelpConfig()`
- ✅ No duplicated formatting code
- ✅ Help output consistent across all tools
- ✅ Type safety enforced by TypeScript compiler
- ✅ Tests pass for all help displays

---

## Conclusion

This proposal provides a comprehensive, DRY solution to standardize the help system across fspec commands and research tools. By introducing `ResearchToolHelpConfig` and `formatResearchToolHelp()`, we:

1. **Fix BUG-074** by moving `--help` check before tool loading
2. **Eliminate code duplication** by centralizing formatting logic
3. **Improve maintainability** through type-safe, structured configuration
4. **Enhance AI optimization** with consistent WHEN TO USE and COMMON ERRORS sections
5. **Ensure consistency** across all tools and commands

The migration path is clear, backward compatible, and can be implemented incrementally. Total estimated time: **6-8 hours** for complete implementation and testing.

---

**Date**: 2025-01-12
**Author**: Claude (AI Analysis)
**Status**: Proposal - Pending Review
**Related Issue**: BUG-074
