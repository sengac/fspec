export const addAttachmentHelp = `
SYNOPSIS
  fspec add-attachment <workUnitId> <filePath> [options]

DESCRIPTION
  Add an attachment to a work unit during Example Mapping. Attachments are stored in
  spec/attachments/<workUnitId>/ and tracked as relative paths in the work unit.

  Use attachments to supplement architecture notes and non-functional requirements with:
  - Architecture diagrams (PNG, SVG, PDF)
  - UI mockups and wireframes
  - API documentation
  - Screenshots and examples
  - Reference documents
  - Any supporting materials

ARGUMENTS
  <workUnitId>
    The ID of the work unit to attach the file to (e.g., AUTH-001, DASH-002)

  <filePath>
    Path to the file to attach. Can be absolute or relative to current directory.
    The file will be copied to spec/attachments/<workUnitId>/

OPTIONS
  -d, --description <text>
    Optional description of the attachment (what it represents, why it's relevant)

WHEN TO USE
  - During Example Mapping discovery phase to capture visual context
  - When architecture notes alone aren't sufficient to convey design
  - To attach mockups, diagrams, or screenshots that clarify requirements
  - To include external documentation or reference materials
  - Any time supporting files would help clarify the work unit

PREREQUISITES
  - Work unit must exist (created with 'fspec create-work-unit')
  - Source file must exist at the specified path
  - Work unit should be in 'specifying' or earlier status

TYPICAL WORKFLOW
  1. Create work unit: fspec create-work-unit AUTH "User Authentication"
  2. Move to specifying: fspec update-work-unit-status AUTH-001 specifying
  3. Start Example Mapping: Add rules, examples, questions
  4. Add architecture notes: fspec add-architecture-note AUTH-001 "Uses JWT tokens"
  5. Add attachments: fspec add-attachment AUTH-001 docs/auth-flow.png
  6. Continue discovery until all questions answered
  7. Generate scenarios: fspec generate-scenarios AUTH-001

EXAMPLES
  # Add architecture diagram during discovery
  $ fspec add-attachment AUTH-001 diagrams/auth-flow.png
  ✓ Attachment added successfully
    File: spec/attachments/AUTH-001/auth-flow.png

  # Add mockup with description
  $ fspec add-attachment UI-002 mockups/dashboard.png --description "Dashboard layout v2"
  ✓ Attachment added successfully
    File: spec/attachments/UI-002/dashboard.png
    Description: Dashboard layout v2

  # Add API documentation
  $ fspec add-attachment API-003 docs/stripe-api-reference.pdf
  ✓ Attachment added successfully
    File: spec/attachments/API-003/stripe-api-reference.pdf

  # Add screenshot for bug reproduction
  $ fspec add-attachment BUG-005 screenshots/error-state.png
  ✓ Attachment added successfully
    File: spec/attachments/BUG-005/error-state.png

COMMON ERRORS
  Error: Work unit 'AUTH-001' does not exist
    → Create the work unit first with 'fspec create-work-unit'

  Error: Source file 'diagram.png' does not exist
    → Check the file path is correct (absolute or relative to current directory)

  Error: Attachment 'diagram.png' already exists
    → The file has already been attached to this work unit
    → Use 'fspec list-attachments AUTH-001' to see all attachments
    → Remove the old attachment first with 'fspec remove-attachment'

COMMON PATTERNS
  # Attach multiple diagrams
  fspec add-attachment AUTH-001 diagrams/auth-sequence.png
  fspec add-attachment AUTH-001 diagrams/token-refresh.png
  fspec add-attachment AUTH-001 diagrams/password-reset.png

  # Attach files during discovery conversation
  # Human: "Here's a mockup of the login screen"
  # AI: "Let me attach that to the work unit"
  fspec add-attachment AUTH-001 ~/Downloads/login-mockup.png

  # Attach reference documentation
  fspec add-attachment PAY-001 docs/stripe-integration-guide.pdf
  fspec add-attachment PAY-001 docs/pci-compliance-checklist.pdf

  # View all attachments for a work unit
  fspec list-attachments AUTH-001

  # View work unit with attachments
  fspec show-work-unit AUTH-001

RELATED COMMANDS
  fspec list-attachments <workUnitId>    List all attachments for a work unit
  fspec remove-attachment <workUnitId> <fileName>  Remove attachment
  fspec show-work-unit <workUnitId>      View work unit details (includes attachments)
  fspec add-architecture-note            Add text-based architecture notes
  fspec add-rule                         Add business rules
  fspec add-example                      Add examples

NOTES
  - Attachments are copied to spec/attachments/<workUnitId>/ directory
  - Original files are not modified
  - Paths are stored as relative paths from project root
  - All file types are supported (images, PDFs, documents, etc.)
  - Attachments are version controlled (add to git)
  - Use attachments to supplement, not replace, architecture notes

AI AGENT GUIDANCE
  - Attachments work alongside architecture notes and NFRs
  - Use attachments when visual context is needed
  - Always confirm file path exists before adding
  - Store attachments in spec/attachments/<workUnitId>/ (automatic)
  - List attachments with 'fspec list-attachments' to see what's attached
`;
