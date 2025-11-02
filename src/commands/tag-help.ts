import type { CommandHelpConfig } from '../utils/help-formatter';

const config: CommandHelpConfig = {
  name: 'register-tag',
  description:
    'Register a new tag in the tags.json registry and regenerate TAGS.md. Tags must follow the format @lowercase-with-hyphens and be assigned to a valid category. Validates against JSON schema and maintains alphabetical order within categories.',
  usage: 'fspec register-tag "<tag>" "<category>" "<description>"',
  whenToUse:
    'Use when defining new tags for feature files and scenarios. Register tags before using them in feature files to ensure they are tracked in tags.json and documented in TAGS.md.',
  arguments: [
    {
      name: 'tag',
      description:
        'Tag name in format @lowercase-with-hyphens (e.g., "@wip", "@critical", "@authentication"). Must start with @ and contain only lowercase letters, numbers, and hyphens.',
      required: true,
    },
    {
      name: 'category',
      description:
        'Category name (case-insensitive) where tag belongs (e.g., "Workflow Tags", "Technical Tags", "Priority Tags"). Must be an existing category in tags.json.',
      required: true,
    },
    {
      name: 'description',
      description:
        'Description of what the tag means and when to use it. Should be clear and actionable.',
      required: true,
    },
  ],
  options: [],
  examples: [
    {
      command:
        'fspec register-tag "@wip" "Workflow Tags" "Work in progress - feature under active development"',
      description: 'Register workflow tag',
      output:
        '✓ Successfully registered @wip in Workflow Tags\n  Updated: spec/tags.json\n  Regenerated: spec/TAGS.md',
    },
    {
      command:
        'fspec register-tag "@authentication" "Technical Tags" "Authentication and authorization features"',
      description: 'Register technical tag',
      output:
        '✓ Successfully registered @authentication in Technical Tags\n  Updated: spec/tags.json\n  Regenerated: spec/TAGS.md',
    },
    {
      command:
        'fspec register-tag "@Critical" "Priority Tags" "Highest priority work"',
      description: 'Register tag with auto-lowercase conversion',
      output:
        'Note: Tag converted to lowercase: @Critical → @critical\n✓ Successfully registered @critical in Priority Tags\n  Updated: spec/tags.json\n  Regenerated: spec/TAGS.md',
    },
  ],
  prerequisites: [
    'tags.json must exist with defined categories',
    'Tag must not already be registered in any category',
    'Category must exist in tags.json',
  ],
  typicalWorkflow: [
    'Identify new tag needed for feature file',
    'Check available categories: Review spec/tags.json',
    'Register tag: fspec register-tag "@tag-name" "Category" "Description"',
    'Use tag in feature files: fspec add-tag-to-feature spec/features/example.feature @tag-name',
    'Verify: Check spec/TAGS.md for updated documentation',
  ],
  commonErrors: [
    {
      error: 'Invalid tag format: "tag"',
      solution:
        'Tag must start with @ and use lowercase-with-hyphens format. Example: @my-tag',
    },
    {
      error: 'Tag @example is already registered in Technical Tags',
      solution:
        'Tag already exists. Use fspec list-tags to see all registered tags, or use a different tag name.',
    },
    {
      error: 'Invalid category: "Custom Tags"',
      solution:
        'Category does not exist. Available categories are listed in the error message. Use one of those or create the category in tags.json first.',
    },
    {
      error: 'Updated tags.json failed schema validation',
      solution:
        'JSON schema validation failed. Check tag format and category name. Changes are rolled back automatically.',
    },
  ],
  relatedCommands: [
    'fspec list-tags - List all registered tags',
    'fspec update-tag - Update tag description',
    'fspec delete-tag - Remove tag from registry',
    'fspec add-tag-to-feature - Add tag to feature file',
    'fspec add-tag-to-scenario - Add tag to specific scenario',
    'fspec validate-tags - Validate all tags in feature files',
  ],
  notes: [
    'Tag names are automatically converted to lowercase',
    'Tags are sorted alphabetically within their category',
    'tags.json is validated against JSON schema after update',
    'TAGS.md is automatically regenerated from tags.json',
    'If TAGS.md generation fails, tags.json is rolled back',
    'Tags must follow format: @lowercase-with-hyphens (no spaces, underscores, or uppercase)',
    'Duplicate tags across categories are not allowed',
  ],
};

export default config;
