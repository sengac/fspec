export function getUpdatingSpecsSection(): string {
  return `## Updating Specifications

### When to Update Feature Files

1. **New Feature**: Create new \`.feature\` file with all scenarios
2. **Feature Enhancement**: Add new scenarios to existing feature file
3. **Bug Fix**: Add scenario that reproduces the bug, then fix code
4. **Architecture Change**: Update architecture notes in doc strings
5. **Deprecated Behavior**: Mark scenario with \`@deprecated\` tag and add replacement

### Change Process (ACDD - Acceptance Criteria Driven Development)

1. **Update Feature File**: Modify \`.feature\` file with new/changed scenarios
2. **Update Tags**: Add/modify tags using \`fspec register-tag\` (updates spec/tags.json)
3. **Write/Update Tests**: Create tests for new scenarios BEFORE implementation
4. **Format**: Run \`fspec format\` to format feature files
5. **Validate**: Run \`fspec validate\` and \`fspec validate-tags\` to ensure correctness
6. **Implement**: Write code to make tests pass
7. **Verify**: Run \`npm test\` to ensure all tests pass
8. **Build**: Run \`npm run build\` to ensure TypeScript compiles
9. **Commit**: Include feature file, test changes, and implementation`;
}
