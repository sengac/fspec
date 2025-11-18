export function getBootstrapFoundationSection(): string {
  return `## Step 2: Bootstrap Foundation (REQUIRED for New Projects)

**CRITICAL**: If \`spec/foundation.json\` does not exist, you MUST bootstrap it using the AI-driven discovery feedback loop. This is ENFORCED by fspec commands.

\`\`\`bash
# AI runs discover-foundation to create draft with placeholders
fspec discover-foundation

# Finalize draft after all fields filled
fspec discover-foundation --finalize
\`\`\`

**What \`discover-foundation\` does:**

1. **Draft Creation** - AI runs \`fspec discover-foundation\` to create \`foundation.json.draft\`
   - Command creates draft with \`[QUESTION: text]\` placeholders for fields requiring input
   - Command creates draft with \`[DETECTED: value]\` for auto-detected fields to verify
   - Draft IS the guidance - defines structure and what needs to be filled

2. **ULTRATHINK Guidance** - Command emits initial system-reminder for AI
   - Instructs AI to analyze EVERYTHING: code structure, entry points, user interactions, documentation
   - Emphasizes understanding HOW system works, then determining WHY it exists and WHAT users can do
   - Guides AI field-by-field through discovery process

3. **Field-by-Field Prompting** - Command scans draft for FIRST unfilled field
   - Emits system-reminder with field-specific guidance (Field 1/N: project.name)
   - Includes exact command to run for simple fields: \`fspec update-foundation projectName "value"\`
   - For capabilities: \`fspec add-capability "name" "description"\` or \`fspec remove-capability "name"\`
   - For personas: \`fspec add-persona "name" "description" --goal "goal"\` or \`fspec remove-persona "name"\`
   - Provides language-agnostic guidance (not specific to JavaScript/TypeScript)

4. **AI Analysis and Update** - AI analyzes codebase, asks human, runs fspec command
   - AI examines code patterns to understand project structure
   - AI asks human for confirmation/clarification
   - AI runs: \`fspec update-foundation projectName "fspec"\`
   - NO manual editing allowed - command detects and reverts manual edits

5. **Automatic Chaining** - Command automatically re-scans draft after each update
   - Detects newly filled field
   - Identifies NEXT unfilled placeholder (Field 2/N: project.vision)
   - Emits system-reminder with guidance for next field
   - Repeats until all [QUESTION:] placeholders resolved

6. **Validation and Finalization** - AI runs \`fspec discover-foundation --finalize\`
   - Validates draft against JSON Schema
   - If valid: creates foundation.json, deletes draft, auto-generates FOUNDATION.md
   - If invalid: shows validation errors with exact field paths, prompts AI to fix and re-run

### Foundation Discovery Iteration

**Iteration fully supported** - update-foundation can re-update any field anytime with no restrictions:

**You CAN go back and fix mistakes:**
- Re-run \`fspec update-foundation\` on any field anytime
- No restrictions on updating already-filled fields
- Draft persists through multiple edits

**Review draft before finalization:**
- Run: \`cat spec/foundation.json.draft\` (or use file reader)
- No dedicated \`show-foundation-draft\` command yet

**Validation failure recovery:**
- If \`--finalize\` fails, draft persists (not deleted)
- Fix the errors using update-foundation commands
- Re-run \`--finalize\` when ready
- Draft only deleted on successful finalization

**Manual edit protection:**
- Direct file edits are detected and reverted (CLI-only enforcement)
- Always use CLI commands

**Why this is mandatory:**

- fspec commands check for foundation.json existence
- Foundation establishes project context (type, personas, capabilities)
- Ensures consistent WHY/WHAT focus (not HOW/implementation)
- Required for Example Mapping and work unit creation
- Provides context for all ACDD workflow steps

**When to skip:**

- ONLY if \`spec/foundation.json\` already exists

**See also:** \`spec/{{DOC_TEMPLATE}}\` section "Bootstrapping Foundation for New Projects" for complete guidance.

`;
}
