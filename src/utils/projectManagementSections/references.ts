export function getReferencesSection(): string {
  return `## References

- **Gherkin Reference**: https://cucumber.io/docs/gherkin/reference
- **Gherkin Best Practices**: https://cucumber.io/docs/bdd/better-gherkin
- **Cucumber Parser**: https://github.com/cucumber/gherkin
- **fspec Foundation**: [spec/FOUNDATION.md](./FOUNDATION.md)
- **Tag Registry**: [spec/TAGS.md](./TAGS.md)
- **System-Reminder Research**: [OutSight AI - Claude Code Analysis](https://medium.com/@outsightai/peeking-under-the-hood-of-claude-code-70f5a94a9a62)

## Enforcement

**AI Agent Integration**:
- fspec commands guide AI to create well-structured specifications
- Validation catches errors immediately, enabling self-correction
- Clear error messages help AI understand and fix issues

**Automation Integration**:
- Lifecycle hooks invoke fspec to validate specifications during development
- Pre-commit hooks reject malformed feature files
- Post-command hooks ensure specs stay aligned with code changes

**Developer Responsibility**:
- Read this document before creating new specifications
- Follow the Gherkin syntax and tag requirements strictly
- Keep \`spec/TAGS.md\` up to date (or use \`fspec register-tag\`)
- Write tests for every scenario before implementing features
- Use fspec commands to maintain specification quality`;
}
