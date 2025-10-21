export function getReadyToStartSection(): string {
  return `## Ready to Start

Run these commands to begin:
\`\`\`bash
fspec board                           # See the current state
fspec list-work-units --status=backlog # View available work
\`\`\`

Pick a work unit and start moving it through the Kanban!
`;
}
