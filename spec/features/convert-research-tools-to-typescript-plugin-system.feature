@done
@discovery
@p1
@cli
@research-tools
@plugin-system
@typescript
@RES-015
Feature: Convert Research Tools to TypeScript Plugin System
  """
  Tool building: 'fspec build-tool <name>' uses esbuild to transpile spec/research-tools/<name>.ts → <name>.js with bundling for node platform
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ARCHITECTURE: Hybrid plugin system - core tools (ast, perplexity, jira, confluence, stakeholder) bundled with fspec, custom tools loaded dynamically from spec/research-tools/
  #   2. INTERFACE: Standard TypeScript interface - export interface ResearchTool { name, description, execute(args), help() }
  #   3. DISCOVERY: Tool registry merges bundled tools (import) + dynamic tools (dynamic import from spec/research-tools/*.ts)
  #   4. ARGS: Use Commander.js .allowUnknownOption() to forward ALL args to tool.execute() - tools parse their own arguments
  #   5. MIGRATION: Convert existing bash scripts to TypeScript - ast.ts, perplexity.ts, jira.ts, confluence.ts, stakeholder.ts in src/research-tools/
  #   6. TESTING: Tools are testable TypeScript modules - unit tests verify execute() logic, integration tests verify fspec loads and runs tools correctly
  #   7. CONFIG: Each tool handles its own config resolution via resolveConfig(toolName) from config-resolution.ts - tools check required fields and throw clear errors
  #   8. Custom tools must be .js files. Provide 'fspec build-tool <name>' command that uses esbuild (already a dependency) to transpile spec/research-tools/<name>.ts → spec/research-tools/<name>.js. Users write TypeScript, run build command, fspec loads .js.
  #   9. Wrap tool errors in <system-reminder> tags for AI visibility. When tool.execute() throws, catch and wrap stderr/error message in system-reminder block so AI agents can see and self-correct. Example: <system-reminder>RESEARCH TOOL ERROR\n\nTool: ast\nError: {message}\n</system-reminder>
  #
  # EXAMPLES:
  #   1. User runs 'fspec research --tool=ast --query "find async functions"' → fspec loads bundled ast.ts → calls ast.execute({ query: 'find async functions' }) → returns JSON output
  #   2. User creates spec/research-tools/custom.ts with ResearchTool interface → runs 'fspec research --tool=custom --arg=value' → fspec dynamically imports custom.ts → executes tool
  #   3. User runs 'fspec research --tool=perplexity --help' → fspec loads perplexity.ts → calls perplexity.help() → displays tool-specific help with options
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should custom tools in spec/research-tools/ be transpiled on-the-fly (tsx/ts-node) or pre-compiled by user?
  #   A: true
  #
  #   Q: Should tool registry cache loaded tools or reload on every invocation?
  #   A: true
  #
  #   Q: When tool.execute() throws error, should fspec wrap it in system-reminder for AI visibility or pass through raw?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. No explicit caching - rely on Node.js module cache. Each CLI invocation is a new process, so caching between invocations is irrelevant. Within a single invocation, Node.js automatically caches dynamic imports. Keep it simple.
  #
  # ========================================
  Background: User Story
    As a developer using fspec on any platform (Windows/macOS/Linux)
    I want to use research tools without bash dependency
    So that research tools work cross-platform and integrate seamlessly with fspec's TypeScript ecosystem

  Scenario: Execute bundled AST research tool
    Given the AST tool is bundled with fspec at src/research-tools/ast.ts
    When I run 'fspec research --tool=ast --query "find async functions"'
    Then fspec should load the ast.ts module
    And call ast.execute(['--query', 'find async functions'])
    And return JSON output with matching functions

  Scenario: Build and execute custom research tool
    Given I have created spec/research-tools/custom.ts with ResearchTool interface
    When I run 'fspec build-tool custom'
    Then esbuild should transpile custom.ts to custom.js
    When I run 'fspec research --tool=custom --arg=value'
    Then fspec should dynamically import spec/research-tools/custom.js
    And execute the tool with forwarded arguments
