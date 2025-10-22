# Getting Started with fspec

## What You'll Learn

In 5 minutes, you'll:
- Install fspec
- Initialize it in your project
- Create your first feature with AI
- Understand the ACDD workflow

## Prerequisites

- Node.js 18+ (for installation)
- An AI agent (Claude Code recommended, but works with any)
- A project to add fspec to (or create a new one)

---

## 1. Installation

```bash
npm install -g @sengac/fspec
```

Verify installation:
```bash
fspec --version
```

---

## 2. Initialize Your Project

```bash
cd /path/to/your/project
fspec init
```

**This will show an interactive menu** to select your AI agent:

```
? Select your AI agent(s) (use spacebar to select, enter to confirm):
❯◯ Claude Code - AI assistant with chat interface
 ◯ Cursor - AI-first code editor
 ◯ Windsurf - AI-powered IDE
 ◯ Cline - VS Code extension
 ... and 14 more options
```

**Or specify directly:**
```bash
fspec init --agent=claude    # Claude Code
fspec init --agent=cursor    # Cursor
fspec init --agent=windsurf  # Windsurf
```

**What this creates:**
- **Agent-specific slash command** (e.g., `.claude/commands/fspec.md`, `.cursor/commands/fspec.md`)
- **Workflow documentation** (e.g., `spec/CLAUDE.md`, `spec/CURSOR.md`)
- **Project structure** (`spec/` directory for features, work units, tags)

---

## 3. Your First Feature

### Tell Your AI Agent What to Build

**Using the `/fspec` slash command:**
```
/fspec Create a story for user login feature
```

*Available in: Claude Code, Cursor, Windsurf, Cline, Aider, and more*

**What happens next:**

#### Step 1: Discovery (Example Mapping)

The AI will ask clarifying questions to understand requirements:

```
AI: "I'll guide you through discovery for user login using Example Mapping.

     Let's start with some questions:

     1. Should we support OAuth in addition to email/password?
     2. What password requirements do you need?
     3. Should we support "Remember Me" functionality?
     4. What happens after 3 failed login attempts?"

You: "Yes to OAuth (Google and GitHub). Passwords need min 8 chars,
     1 uppercase, 1 number. Yes to Remember Me. Lock account after 3 fails."

AI: "Great! Let me capture these as rules and examples..."
```

**The AI runs these commands (you don't):**
```bash
fspec set-user-story AUTH-001 \
  --role "user" \
  --action "log in securely" \
  --benefit "I can access my account"

fspec add-rule AUTH-001 "Password must be at least 8 characters with 1 uppercase and 1 number"
fspec add-rule AUTH-001 "Account locks after 3 failed login attempts"

fspec add-example AUTH-001 "User logs in with valid email/password and is authenticated"
fspec add-example AUTH-001 "User logs in with Google OAuth and is authenticated"
fspec add-example AUTH-001 "User fails login 3 times and account is locked"
```

#### Step 2: Specification

The AI generates validated Gherkin scenarios:

```gherkin
@phase1 @authentication @story @AUTH-001
Feature: User Login

  """
  Architecture notes:
  - Uses JWT tokens for session management
  - OAuth integration via Passport.js
  - Rate limiting on login endpoint
  """

  Background: User Authentication
    As a user
    I want to log in securely
    So that I can access my account

  Scenario: Login with valid email and password
    Given I am on the login page
    When I enter valid credentials
    And I submit the login form
    Then I should be logged in
    And I should see the dashboard

  Scenario: Login with Google OAuth
    Given I am on the login page
    When I click "Login with Google"
    And I authorize the application
    Then I should be logged in
    And I should see the dashboard

  Scenario: Account locks after 3 failed attempts
    Given I am on the login page
    And I have failed login 2 times
    When I enter invalid credentials
    Then my account should be locked
    And I should see an error message
```

**The AI runs:**
```bash
fspec generate-scenarios AUTH-001
fspec validate  # Ensures valid Gherkin
```

#### Step 3: Testing

The AI writes tests that map to scenarios (must fail first - red phase):

```typescript
/**
 * Feature: spec/features/user-login.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

describe('Feature: User Login', () => {
  describe('Scenario: Login with valid email and password', () => {
    it('should authenticate user and redirect to dashboard', async () => {
      // Given I am on the login page
      // When I enter valid credentials and submit
      // Then I should be logged in and see dashboard

      // Test implementation...
    });
  });

  // More test scenarios...
});
```

**The AI runs:**
```bash
npm test  # Tests MUST fail (red phase)

fspec link-coverage user-login \
  --scenario "Login with valid email and password" \
  --test-file src/__tests__/auth.test.ts \
  --test-lines 10-25
```

#### Step 4: Implementation

The AI writes minimal code to pass tests (green phase):

```typescript
// src/auth/login.ts
export async function login(email: string, password: string) {
  // Implementation...
}
```

**The AI runs:**
```bash
npm test  # Tests MUST pass (green phase)

fspec link-coverage user-login \
  --scenario "Login with valid email and password" \
  --test-file src/__tests__/auth.test.ts \
  --impl-file src/auth/login.ts \
  --impl-lines 10-45
```

#### Step 5: Validation

The AI runs all quality checks:

```bash
npm test           # All tests (not just new ones)
npm run typecheck  # TypeScript validation
npm run lint       # Code quality
fspec validate     # Gherkin syntax
fspec check        # Complete validation
```

**Result:** Clean, tested, documented feature!

---

## 4. Check the Board

See what's in progress:

```
/fspec
```

```
AI: Let me check the fspec board...

┌─────────┬────────────┬─────────┬───────────────┬───────────┬──────┐
│ Backlog │ Specifying │ Testing │ Implementing  │Validating │ Done │
├─────────┼────────────┼─────────┼───────────────┼───────────┼──────┤
│ AUTH-002│            │         │ AUTH-001      │           │      │
│ DASH-001│            │         │ User Login    │           │      │
│         │            │         │ (5 pts)       │           │      │
└─────────┴────────────┴─────────┴───────────────┴───────────┴──────┘

Work unit AUTH-001 is currently in implementing status.
3 story points completed this sprint.
```

---

## 5. Understand Work Unit Types

fspec tracks three types of work:

### Story

**User-facing features** that deliver value.

```
/fspec Create a story for password reset
```

- Requires full ACDD workflow
- Needs tests and specifications
- Tracked with story points for estimation

### Bug

**Broken functionality** that needs fixing.

```
/fspec Create a bug for session timeout not working
```

- Requires tests to reproduce issue
- Follows ACDD workflow
- Links to related feature file

### Task

**Non-user-facing work** like refactoring or infrastructure.

```
/fspec Create a task to refactor auth middleware
```

- Tests optional (use judgment)
- Follows simplified workflow
- Useful for tech debt, docs, config

---

## 6. Next Steps

Now that you've created your first feature, explore:

- **[User Guide](./user-guide.md)** - Comprehensive command reference
- **[ACDD Workflow](./acdd-workflow.md)** - Understanding the process
- **[Example Mapping](./example-mapping.md)** - Discovery techniques
- **[Work Units](./work-units.md)** - Project management
- **[Git Checkpoints](./checkpoints.md)** - Safe experimentation
- **[Virtual Hooks](./virtual-hooks.md)** - Quality gates
- **[Reverse ACDD](./reverse-acdd.md)** - Document existing code

---

## Tips for Success

✅ **Always specify work unit type** (story/bug/task)
✅ **Let AI handle fspec commands** - You provide intent, AI executes
✅ **Do Example Mapping** - Don't skip discovery
✅ **Let tests fail first** - Proves they test real behavior
✅ **Run all tests during validation** - Catch regressions
✅ **Use the board** - Track what's in progress

❌ **Don't skip steps** - ACDD order is enforced for a reason
❌ **Don't write code before tests** - Temporal ordering prevents this
❌ **Don't guess requirements** - Example Mapping reveals the truth
❌ **Don't manually edit JSON files** - Always use fspec commands

---

## Troubleshooting

**"AI isn't creating work units"**
- Make sure you specify the type: "Create a story for..."
- Try being more explicit: "/fspec Create a story work unit for user authentication"

**"Tests are passing when they should fail"**
- You may have implementation code already
- Delete the implementation and verify tests fail
- This proves the tests actually work

**"Gherkin validation fails"**
- Run `fspec validate` to see syntax errors
- Check for missing tags (@phase, @component, @story/@bug/@task)
- Ensure Background section has user story

**"Coverage tracking not working"**
- Ensure test file has feature reference comment at top
- Run `fspec show-coverage <feature>` to see current mappings
- Use `fspec link-coverage` to add mappings

**Need more help?**
- Run `fspec <command> --help` for detailed command docs
- Run `fspec help specs` for Gherkin commands
- Run `fspec help work` for Kanban commands
- Run `fspec help discovery` for Example Mapping commands

---

**You're ready!** Start building features with discipline.
