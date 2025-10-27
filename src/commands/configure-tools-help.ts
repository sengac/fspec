import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'configure-tools',
  description:
    'Configure test and quality check commands for platform-agnostic workflow. Required for ACDD validation phase.',
  usage: 'fspec configure-tools [options]',
  whenToUse:
    'Use this command during first-time setup or when changing test/quality tools. fspec is platform-agnostic and does NOT hardcode tool commands. You MUST configure tools for your specific platform (Node.js, Python, Rust, Go, etc.).',
  prerequisites: [
    'Identify your testing framework (<test-command>, pytest, cargo test, go test, etc.)',
    'Identify your quality check commands (linting, formatting, type checking)',
    'Run from project root directory',
  ],
  arguments: [],
  options: [
    {
      flag: '--test-command <command>',
      description:
        'Test command to run (e.g., "<test-command>", "pytest", "cargo test", "go test ./...")',
    },
    {
      flag: '--quality-commands <commands...>',
      description:
        'Quality check commands to run (e.g., "<quality-check-commands>", "black --check ." "mypy .", "cargo clippy" "cargo fmt --check")',
    },
    {
      flag: '--reconfigure',
      description: 'Re-detect tools and update existing configuration',
    },
  ],
  examples: [
    {
      command:
        'fspec configure-tools --test-command "<test-command>" --quality-commands "<quality-check-commands>"',
      description: 'Configure tools for your platform',
      output:
        '✓ Test command configured: <test-command>\n✓ Quality check commands configured: <quality-check-commands>\n✓ Configuration saved to spec/fspec-config.json',
    },
    {
      command:
        'fspec configure-tools --test-command "pytest" --quality-commands "black --check ." "mypy ."',
      description: 'Configure for Python project with pytest',
      output:
        '✓ Test command configured: pytest\n✓ Quality check commands configured: black --check ., mypy .\n✓ Configuration saved to spec/fspec-config.json',
    },
    {
      command:
        'fspec configure-tools --test-command "cargo test" --quality-commands "cargo clippy" "cargo fmt --check"',
      description: 'Configure for Rust project with cargo',
      output:
        '✓ Test command configured: cargo test\n✓ Quality check commands configured: cargo clippy, cargo fmt --check\n✓ Configuration saved to spec/fspec-config.json',
    },
    {
      command:
        'fspec configure-tools --test-command "go test ./..." --quality-commands "go fmt ./..." "go vet ./..."',
      description: 'Configure for Go project',
      output:
        '✓ Test command configured: go test ./...\n✓ Quality check commands configured: go fmt ./..., go vet ./...\n✓ Configuration saved to spec/fspec-config.json',
    },
    {
      command: 'fspec configure-tools --reconfigure',
      description: 'Re-detect and update tool configuration',
      output:
        'Detecting project type...\nFound: Node.js (package.json)\nRecommended test command: <test-command>\nRecommended quality commands: <quality-check-commands>\n[Interactive prompts to confirm...]',
    },
  ],
  commonErrors: [
    {
      error: 'NO TEST COMMAND CONFIGURED',
      fix: 'Run: fspec configure-tools --test-command "your-test-command"',
    },
    {
      error: 'No quality check commands configured',
      fix: 'Run: fspec configure-tools --quality-commands "lint-cmd" "format-cmd"',
    },
  ],
  typicalWorkflow:
    '1. Detect framework: Check package.json, Cargo.toml, go.mod, etc.\n2. Configure tools: fspec configure-tools --test-command <cmd> --quality-commands <cmds>\n3. Verify: cat spec/fspec-config.json\n4. Use in ACDD: Tools are used automatically during validating phase',
  relatedCommands: ['bootstrap', 'validate', 'check', 'board'],
  notes: [
    'Configuration persists in spec/fspec-config.json',
    'Commands are run during validating phase of ACDD workflow',
    'Platform-agnostic: works with any language/framework',
    'System reminders guide you if tools are not configured',
    'Use --reconfigure to update existing configuration',
  ],
};

export default config;
