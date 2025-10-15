export const removeAttachmentHelp = `
SYNOPSIS
  fspec remove-attachment <workUnitId> <fileName> [options]

DESCRIPTION
  Remove an attachment from a work unit. By default, this removes both the tracking entry
  and deletes the file from disk. Use --keep-file to preserve the file.

ARGUMENTS
  <workUnitId>
    The ID of the work unit to remove the attachment from (e.g., AUTH-001)

  <fileName>
    The name of the file to remove (e.g., diagram.png)
    Must match the basename of the attachment path

OPTIONS
  --keep-file
    Keep the file on disk (only remove from work unit tracking)

WHEN TO USE
  - To remove outdated or incorrect attachments
  - To clean up broken attachment links
  - To remove duplicate attachments
  - When attachment is no longer relevant to the work unit

PREREQUISITES
  - Work unit must exist
  - Attachment must be tracked in the work unit

TYPICAL WORKFLOW
  1. List attachments: fspec list-attachments AUTH-001
  2. Identify attachment to remove
  3. Remove attachment: fspec remove-attachment AUTH-001 old-diagram.png
  4. Verify removal: fspec list-attachments AUTH-001

EXAMPLES
  # Remove attachment and delete file
  $ fspec remove-attachment AUTH-001 auth-flow.png
  ✓ Attachment removed from work unit and file deleted
    File: spec/attachments/AUTH-001/auth-flow.png

  # Remove attachment but keep file on disk
  $ fspec remove-attachment AUTH-001 important-doc.pdf --keep-file
  ✓ Attachment removed from work unit (file kept)
    File: spec/attachments/AUTH-001/important-doc.pdf

  # Remove attachment when file is already missing
  $ fspec remove-attachment AUTH-001 missing-file.png
  ⚠ Attachment removed from work unit (file was already missing)
    File: spec/attachments/AUTH-001/missing-file.png

COMMON ERRORS
  Error: Work unit 'AUTH-001' does not exist
    → Create the work unit first or check the work unit ID

  Error: Work unit 'AUTH-001' has no attachments to remove
    → The work unit has no attachments
    → Use 'fspec list-attachments AUTH-001' to verify

  Error: Attachment 'diagram.png' not found for work unit 'AUTH-001'
    → The file name doesn't match any tracked attachments
    → Use 'fspec list-attachments AUTH-001' to see available attachments
    → Ensure the file name matches exactly (case-sensitive)

RELATED COMMANDS
  fspec add-attachment <workUnitId> <filePath>  Add attachment
  fspec list-attachments <workUnitId>  List all attachments
  fspec show-work-unit <workUnitId>  View work unit details

NOTES
  - By default, both tracking entry and file are removed
  - Use --keep-file to preserve the file on disk
  - Removing attachment updates work unit timestamps
  - File name is matched against basename of attachment paths
  - If file doesn't exist, only tracking entry is removed (warning shown)

AI AGENT GUIDANCE
  - Use 'fspec list-attachments' before removing to confirm file name
  - Consider --keep-file if file might be useful later
  - Remove broken links (missing files) to keep work unit clean
  - Re-add corrected attachments after removal if needed
`;
