export function getToolConfigurationSection(): string {
  return `## Tool Configuration: Platform-Agnostic Workflow

**CRITICAL**: fspec is platform-agnostic and does NOT hardcode test or quality check commands. You MUST configure tools for your specific platform (Node.js, Python, Rust, Go, etc.).

### First Time Setup

When you first move a work unit to \`validating\` status, fspec will emit a system-reminder if tools are not configured:

\`\`\`
**IMPORTANT:** NO TEST COMMAND CONFIGURED

No test command configured. Use Read/Glob tools to detect test framework, then run:

  fspec configure-tools --test-command <cmd>

If no test tools detected, search for current best practices:
  Query: "best <platform> testing tools 2025"
\`\`\`

### Configuring Tools

**Step 1: Detect Framework**

Use Read or Glob tools to detect what testing and quality tools exist:

\`\`\`bash
# Check for Node.js
cat package.json | grep -E "test|vitest|jest"

# Check for Python
ls | grep -E "pytest|unittest"

# Check for Rust
ls | grep "Cargo.toml"
\`\`\`

**Step 2: Configure Commands**

Once you know the platform, configure the commands:

\`\`\`bash
# Node.js with npm/Vitest
fspec configure-tools --test-command "npm test" \\
  --quality-commands "npm run format" "npx tsc --noEmit"

# Python with pytest
fspec configure-tools --test-command "pytest" \\
  --quality-commands "black --check ." "mypy ."

# Rust with cargo
fspec configure-tools --test-command "cargo test" \\
  --quality-commands "cargo clippy" "cargo fmt --check"

# Go
fspec configure-tools --test-command "go test ./..." \\
  --quality-commands "go fmt ./..." "go vet ./..."
\`\`\`

**Step 3: Verify Configuration**

Check that tools are configured:

\`\`\`bash
cat spec/fspec-config.json
\`\`\`

Should contain:

\`\`\`json
{
  "agent": "claude",
  "tools": {
    "test": {
      "command": "npm test"
    },
    "qualityCheck": {
      "commands": ["npm run format", "npx tsc --noEmit"]
    }
  }
}
\`\`\`

### Using Configured Tools

After configuration, when you move work units to \`validating\`, fspec will tell you what to run:

\`\`\`
**IMPORTANT:** RUN TESTS

Run tests: npm test

**IMPORTANT:** RUN QUALITY CHECKS

Run quality checks: npm run format && npx tsc --noEmit
\`\`\`

Then run those commands as instructed.

### Reconfiguring Tools

If tools change (switching test frameworks, adding new quality checks):

\`\`\`bash
# Re-detect and reconfigure
fspec configure-tools --reconfigure

# Or directly update
fspec configure-tools --test-command "new-test-command"
\`\`\`

### Important Notes

- **Platform Detection**: fspec does NOT detect your platform automatically - YOU must configure it
- **One-Time Setup**: Configuration persists in \`spec/fspec-config.json\` and is used for all work units
- **System-Reminders**: If config is missing, you'll be prompted during workflow transitions
- **No Hardcoding**: Documentation uses \`<test-command>\` and \`<quality-check-commands>\` placeholders - these refer to YOUR configured commands

`;
}
