export const listAttachmentsHelp = `
SYNOPSIS
  fspec list-attachments <workUnitId>

DESCRIPTION
  List all attachments for a work unit. Shows file paths, sizes, and modification dates.
  Verifies whether files exist on the filesystem and highlights missing files.

ARGUMENTS
  <workUnitId>
    The ID of the work unit to list attachments for (e.g., AUTH-001, DASH-002)

WHEN TO USE
  - To see what files are attached to a work unit
  - To verify attachments exist on filesystem
  - To check file sizes and modification dates
  - Before adding duplicate attachments
  - When reviewing work unit context

PREREQUISITES
  - Work unit must exist (created with 'fspec create-story', 'fspec create-bug', or 'fspec create-task')

TYPICAL WORKFLOW
  1. View work unit details: fspec show-work-unit AUTH-001
  2. List attachments: fspec list-attachments AUTH-001
  3. Review attachment files in spec/attachments/AUTH-001/
  4. Add more attachments if needed: fspec add-attachment AUTH-001 <file>

EXAMPLES
  # List attachments for work unit
  $ fspec list-attachments AUTH-001

  Attachments for AUTH-001 (3):

    ✓ spec/attachments/AUTH-001/auth-flow.png
      Size: 125.45 KB
      Modified: 1/15/2025, 10:30:00 AM

    ✓ spec/attachments/AUTH-001/token-refresh.png
      Size: 89.23 KB
      Modified: 1/15/2025, 11:00:00 AM

    ✓ spec/attachments/AUTH-001/api-docs.pdf
      Size: 523.67 KB
      Modified: 1/15/2025, 11:15:00 AM

  # No attachments
  $ fspec list-attachments AUTH-002
  No attachments found for work unit AUTH-002

  # Missing files detected
  $ fspec list-attachments AUTH-003

  Attachments for AUTH-003 (2):

    ✓ spec/attachments/AUTH-003/diagram.png
      Size: 45.12 KB
      Modified: 1/14/2025, 3:00:00 PM

    ✗ spec/attachments/AUTH-003/deleted-file.pdf
      File not found on filesystem

COMMON ERRORS
  Error: Work unit 'AUTH-001' does not exist
    → Create the work unit first with 'fspec create-story', 'fspec create-bug', or 'fspec create-task'

  No attachments found for work unit AUTH-001
    → The work unit exists but has no attachments
    → Add attachments with 'fspec add-attachment AUTH-001 <file>'

RELATED COMMANDS
  fspec add-attachment <workUnitId> <filePath>  Add attachment
  fspec remove-attachment <workUnitId> <fileName>  Remove attachment
  fspec show-work-unit <workUnitId>  View work unit details (includes attachments)

NOTES
  - Attachments are stored in spec/attachments/<workUnitId>/
  - File sizes are displayed in kilobytes (KB)
  - Missing files are highlighted with ✗ marker
  - Use 'fspec remove-attachment' to remove broken links

AI AGENT GUIDANCE
  - Use this command to check what's already attached before adding duplicates
  - Verify attachment paths are correct (✓ vs ✗ markers)
  - Use 'fspec show-work-unit' for complete work unit context
  - Remove missing attachments with 'fspec remove-attachment'
`;
