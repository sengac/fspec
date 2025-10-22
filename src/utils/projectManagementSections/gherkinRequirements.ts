export function getGherkinRequirementsSection(): string {
  return `## Gherkin Feature File Requirements

### 1. ALL Acceptance Criteria MUST Be in .feature Files

- **File Location**: All \`.feature\` files live in the \`spec/features/\` directory
- **File Naming**: Use kebab-case names that describe the feature (e.g., \`gherkin-validation.feature\`, \`tag-registry-management.feature\`)
- **File Format**: Gherkin syntax following the official specification: https://cucumber.io/docs/gherkin/reference

### 2. User Stories MUST Be at the Top as Background

Following the Gherkin specification, user stories belong in the \`Background\` section at the top of each feature file.

**Format**:
\`\`\`gherkin
@phase1 @cli @feature-management
Feature: Create Feature File with Template

  Background: User Story
    As a developer using AI agents for spec-driven development
    I want to create new feature files with proper Gherkin structure
    So that AI can write valid specifications without manual setup

  Scenario: Create feature file with default template
    Given I am in a project with a spec/features/ directory
    When I run \`fspec create-feature "User Authentication"\`
    Then a file "spec/features/user-authentication.feature" should be created
    And the file should contain a valid Gherkin feature structure
    And the file should include a Background section placeholder
    And the file should include a Scenario placeholder
\`\`\`

### 3. Architecture Notes MUST Use Triple-Quoted Blocks

Use Gherkin's doc string syntax (""") for architecture notes, implementation details, and technical context.

**Format**:
\`\`\`gherkin
@phase1 @parser @validation @gherkin
Feature: Gherkin Syntax Validation

  """
  Architecture notes:
  - This feature uses @cucumber/gherkin-parser for official Gherkin validation
  - Parser returns AST (Abstract Syntax Tree) or syntax errors
  - Validation is synchronous and fast (no async operations needed)
  - Error messages are formatted for AI agent comprehension
  - Supports all Gherkin keywords: Feature, Background, Scenario, Given, When, Then, And, But
  - Validates doc strings ("""), data tables (|), and tags (@)

  Critical implementation requirements:
  - MUST use @cucumber/gherkin-parser (official Cucumber parser)
  - MUST report line numbers for syntax errors
  - MUST validate ALL .feature files when no specific file provided
  - MUST exit with non-zero code on validation failure
  - Error output MUST be clear enough for AI to self-correct

  References:
  - Gherkin Spec: https://cucumber.io/docs/gherkin/reference
  - Parser Docs: https://github.com/cucumber/gherkin
  """

  Background: User Story
    As an AI agent writing Gherkin specifications
    I want immediate syntax validation feedback
    So that I can correct errors before committing malformed feature files
\`\`\`

### 4. Tags MUST Be Used for Organization

Tags can be applied at both **feature level** and **scenario level** following the \`@tag\` syntax.

#### Feature-Level Tags (Required)

Every feature file MUST have these tags at the top:

**Required Tags**:
- **Phase Tag**: \`@phase1\`, \`@phase2\`, \`@phase3\` (from FOUNDATION.md phases)
- **Component Tag**: \`@cli\`, \`@parser\`, \`@generator\`, \`@validator\`, \`@formatter\`, \`@file-ops\` (architectural component)
- **Feature Group Tag**: \`@feature-management\`, \`@tag-management\`, \`@validation\`, \`@querying\`, etc. (functional area)

**Optional Tags**:
- **Technical Tags**: \`@gherkin\`, \`@cucumber-parser\`, \`@prettier\`, \`@mermaid\`, \`@ast\`, etc.
- **Platform Tags**: \`@windows\`, \`@macos\`, \`@linux\`, \`@cross-platform\`
- **Priority Tag**: \`@critical\`, \`@high\`, \`@medium\`, \`@low\` (implementation priority)
- **Status Tag**: \`@wip\`, \`@todo\`, \`@done\`, \`@deprecated\`, \`@blocked\` (development status)
- **Testing Tags**: \`@unit-test\`, \`@integration-test\`, \`@e2e-test\`, \`@manual-test\`
- **Automation Tags**: \`@hook\`, \`@cli-integration\`, \`@acdd\`, \`@spec-alignment\`

**Feature-Level Example**:
\`\`\`gherkin
@phase1 @cli @parser @validation @gherkin @cucumber-parser @cross-platform @critical @integration-test
Feature: Gherkin Syntax Validation
\`\`\`

#### Scenario-Level Tags (Optional)

Individual scenarios can have their own tags for more granular organization:

**Common Scenario Tags**:
- **Test Type**: \`@smoke\`, \`@regression\`, \`@sanity\`, \`@acceptance\`
- **Test Scope**: \`@edge-case\`, \`@happy-path\`, \`@error-handling\`
- **Environment**: \`@local\`, \`@staging\`, \`@production\`

**IMPORTANT**: Work unit ID tags (e.g., \`@AUTH-001\`, \`@DASH-002\`) MUST be at feature level only, never at scenario level. Use coverage files (\`*.feature.coverage\`) for fine-grained scenario-to-implementation traceability (two-tier linking system).

**Scenario-Level Example**:
\`\`\`gherkin
@phase1
@authentication
@cli
Feature: User Login

  @smoke
  @critical
  Scenario: Successful login with valid credentials
    Given I am on the login page
    When I enter valid credentials
    Then I should be logged in

  @regression
  @edge-case
  Scenario: Login with expired session
    Given I have an expired session
    When I attempt to login
    Then I should be prompted to re-authenticate
\`\`\`

**Important Notes**:
- Scenarios **inherit** all feature-level tags automatically
- Scenario-level tags are **optional** and used for fine-grained filtering
- Required tags (phase, component, feature-group) only apply to feature-level tags
- All tags (feature-level and scenario-level) MUST be registered in \`spec/tags.json\`

**Tag Registry**: All tags MUST be documented in \`spec/TAGS.md\` with their purpose and usage guidelines.`;
}
