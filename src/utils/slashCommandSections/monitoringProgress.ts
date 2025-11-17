export function getMonitoringProgressSection(): string {
  return `## Step 11: Monitoring Progress

\`\`\`bash
fspec board                           # Visual Kanban board
fspec list-work-units --status=implementing  # See what's in progress
fspec show-work-unit EXAMPLE-006           # Detailed work unit view
fspec generate-summary-report         # Comprehensive report
fspec show-coverage                   # Project-wide coverage report
fspec show-coverage user-authentication # Feature-specific coverage
\`\`\`

`;
}
