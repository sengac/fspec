# Reverse ACDD: Reverse Engineering Existing Codebases

This command guides you through reverse engineering an existing codebase to discover user stories, personas, and acceptance criteria, then documenting them using fspec's ACDD workflow.

You will use fspec, which is a CLI program installed locally on this machine.

IMMEDIATELY: run "fspec --help" and the following more detailed help commands:

  fspec help specs        - Gherkin feature file commands
  fspec help work         - Work unit and Kanban workflow commands
  fspec help discovery    - Example mapping commands
  fspec help metrics      - Progress tracking and reporting commands
  fspec help setup        - Configuration and setup commands

**IMPORTANT: ALL fspec commands have comprehensive `--help` documentation**. For ANY command you need to use, run `fspec <command> --help` to see complete usage syntax, examples, and best practices.

Fully read [fspec.md](./fspec.md) and add it to your context before proceeding with this reverse ACDD workflow.

## What is Reverse ACDD?

**Reverse ACDD** is the process of analyzing an existing codebase (without specifications) and inferring:
- User stories and personas
- Acceptance criteria
- User interactions and workflows
- Business rules and constraints

Once discovered, these are documented using fspec's standard ACDD workflow (Gherkin features, work units, epics, tests).

## Workflow Overview

1. **Analyze the codebase** to identify user-facing interactions
2. **Group interactions into epics** (logical business domains)
3. **Create work units** for each user story
4. **Generate feature files** with inferred acceptance criteria
5. **Update foundation.json** with user story maps (Mermaid diagrams)
6. **Create skeleton test files** (structure only, not implemented)
7. **Use Example Mapping** when encountering ambiguous code

## Step 1: Identify User-Facing Interactions

Look for code that represents user interactions:

### Web Applications (Express, React, Vue, etc.)
- **Routes**: POST /login, GET /dashboard, POST /checkout
- **API Endpoints**: GET /api/users, POST /api/orders
- **UI Components**: LoginForm, DashboardWidget, CheckoutFlow
- **Event Handlers**: onClick, onSubmit, onChange

### CLI Applications
- **Commands**: login, logout, deploy, status
- **Subcommands**: user create, user delete
- **Flags/Options**: --verbose, --output=json

### Desktop/Mobile Applications
- **Screens/Views**: LoginScreen, DashboardView, SettingsPanel
- **Actions**: loginButtonClicked, logoutRequested
- **Gestures**: swipe, tap, long-press

### Background Services
- **Scheduled Jobs**: DailyReportGenerator, InvoiceProcessor
- **Event Processors**: OrderCreatedHandler, UserRegisteredHandler
- **Message Handlers**: handlePaymentReceived, handleRefundRequest

## Step 2: Group Interactions into Epics

Organize related interactions into epics (business domains):

### Example Epic Groups
- **User Management** (AUTH): Login, Logout, Registration, Password Reset
- **Payment Processing** (PAY): Checkout, Refund, Invoice Generation
- **Dashboard Features** (DASH): View Metrics, Export Data, Customize Layout
- **Admin Tools** (ADMIN): Manage Users, View Logs, Configure Settings

### Creating Epics with fspec
```bash
fspec create-epic "User Management" AUTH "Authentication and user session management"
fspec create-epic "Payment Processing" PAY "Checkout and payment workflows"
fspec create-epic "Dashboard Features" DASH "User dashboard and data visualization"
```

## Step 3: Create Work Units

For each user story, create a work unit:

```bash
fspec create-work-unit AUTH "User Login" --description "User authenticates with email and password" --epic=user-management
fspec create-work-unit AUTH "User Logout" --description "User ends their session" --epic=user-management
fspec create-work-unit PAY "Complete Checkout" --description "User completes purchase" --epic=payment-processing
```

Move work units to `specifying` status:
```bash
fspec update-work-unit-status AUTH-001 specifying
```

## Step 4: Generate Feature Files

Create feature files with inferred acceptance criteria:

### Template Structure
```gherkin
@phase1 @authentication @api
Feature: User Login

  Background: User Story
    As a registered user
    I want to log in with my email and password
    So that I can access my account

  # Inferred from code - verify with human
  Scenario: Login with valid credentials
    Given I am on the login page
    And I have a registered account
    When I enter my email and password
    And I click the login button
    Then I should be redirected to the dashboard
    And I should see my username in the header

  # Inferred from error handling code
  Scenario: Login with invalid credentials
    Given I am on the login page
    When I enter an incorrect password
    And I click the login button
    Then I should see an error message "Invalid credentials"
    And I should remain on the login page
```

### Inference Strategy

**From Routes/Endpoints:**
- Route: `POST /api/auth/login` → Scenario: "Login with valid credentials"
- Route: `POST /api/auth/logout` → Scenario: "User logs out"
- Route: `GET /api/dashboard` → Scenario: "View dashboard"

**From Error Handling:**
- `if (!user) throw new Error('User not found')` → Scenario: "Login with non-existent user"
- `if (!password) throw new Error('Password required')` → Scenario: "Login with missing password"
- `if (bcrypt.compare(password, hash) === false)` → Scenario: "Login with wrong password"

**From Validation:**
- `email.includes('@')` → Step: "And the email is in valid format"
- `password.length >= 8` → Step: "And the password is at least 8 characters"
- `age >= 18` → Step: "And the user is at least 18 years old"

**From Business Logic:**
- `if (order.total > 0) processPayment()` → Scenario: "Process paid order"
- `if (user.role === 'admin') showAdminPanel()` → Scenario: "Admin views admin panel"
- `if (cart.isEmpty()) redirectToProducts()` → Scenario: "View checkout with empty cart"

## Step 5: Update foundation.json with User Story Maps

Create Mermaid diagrams showing user workflows:

```bash
fspec add-diagram "User Story Maps" "Authentication Flow" "
graph TB
  User[User] -->|Login| AUTH-001[User Login]
  User -->|Logout| AUTH-002[User Logout]
  User -->|Forgot| AUTH-003[Password Reset]

  AUTH-001 -->|Success| DASH-001[View Dashboard]
  AUTH-001 -->|Fail| AUTH-001
  AUTH-003 -->|Reset| AUTH-001
"
```

## Step 6: Create Skeleton Test Files

Generate test file structure (NOT implementation):

```typescript
/**
 * Feature: spec/features/user-login.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 *
 * NOTE: This is a skeleton test file generated by reverse ACDD.
 * Tests are NOT implemented - only structure is provided.
 */

import { describe, it, expect } from 'vitest';

describe('Feature: User Login', () => {
  describe('Scenario: Login with valid credentials', () => {
    it('should redirect to dashboard and display username', async () => {
      // Given I am on the login page
      // TODO: Implement setup

      // And I have a registered account
      // TODO: Implement account creation

      // When I enter my email and password
      // TODO: Implement form submission

      // Then I should be redirected to the dashboard
      // TODO: Implement assertion

      // And I should see my username in the header
      // TODO: Implement assertion
    });
  });
});
```

## Step 7: Handle Ambiguous Code

When you encounter unclear business logic:

### Example: Magic Numbers
```javascript
if (discount > 42) {  // AMBIGUOUS: What does 42 mean?
  applySpecialDiscount();
}
```

**Action:**
1. Document what you know from the code
2. Mark scenario as "AMBIGUOUS" with comment
3. Offer to run Example Mapping with human

```gherkin
# AMBIGUOUS: magic number 42 in discount logic - needs human clarification
Scenario: Apply special discount
  Given a customer has a discount code
  And the discount value is greater than 42  # Why 42? Ask human.
  When they complete checkout
  Then a special discount should be applied
```

### Example Mapping for Ambiguity

Run Example Mapping interactively:

```bash
fspec example-map AUTH-001
```

Ask questions like:
- Red Card: "What does the magic number 42 represent?"
- Red Card: "Should admin users have different checkout flow?"
- Red Card: "What happens when payment gateway is down?"

## Completion Criteria

Reverse ACDD is complete when:

1. ✓ All user-facing interactions have feature files
2. ✓ All epics have at least one work unit
3. ✓ foundation.json contains complete user story map(s)
4. ✓ All ambiguous scenarios are documented with clarification needed
5. ✓ Skeleton test files exist for all feature files

### Example Completion Report

```
Reverse ACDD complete:
- 3 epics created (AUTH, PAY, DASH)
- 8 work units created
- 8 feature files generated
- 8 skeleton test files created
- foundation.json updated with 2 user story map diagrams
- 3 scenarios marked AMBIGUOUS for human review

Next steps:
1. Review AMBIGUOUS scenarios and run Example Mapping
2. Implement skeleton tests (TDD red-green-refactor)
3. Validate all feature files: fspec validate
4. Begin forward ACDD for new features
```

## Transitioning to Forward ACDD

After reverse ACDD is complete, future work follows **forward ACDD**:

1. **Discovery**: Example Mapping for new features
2. **Specify**: Write Gherkin scenarios
3. **Test**: Write failing tests (TDD red phase)
4. **Implement**: Write minimal code to pass tests (green phase)
5. **Validate**: Refactor, review, and validate

Use fspec commands to maintain specifications:
- `fspec validate` - Validate Gherkin syntax
- `fspec format` - Format feature files
- `fspec validate-tags` - Verify tags are registered
- `fspec check` - Run all validation checks

## Example: Reverse Engineering Express.js Application

```javascript
// Existing code without specs
app.post('/api/auth/login', async (req, res) => {
  const { email, password } = req.body;

  if (!email || !password) {
    return res.status(400).json({ error: 'Email and password required' });
  }

  const user = await User.findOne({ email });
  if (!user) {
    return res.status(401).json({ error: 'Invalid credentials' });
  }

  const isValid = await bcrypt.compare(password, user.passwordHash);
  if (!isValid) {
    return res.status(401).json({ error: 'Invalid credentials' });
  }

  const token = jwt.sign({ userId: user.id }, process.env.JWT_SECRET);
  res.json({ token, user: { id: user.id, email: user.email } });
});
```

### Reverse Engineering Steps:

1. **Identify Interaction**: POST /api/auth/login → "User Login"
2. **Create Epic**: `fspec create-epic "User Management" AUTH "Authentication and user sessions"`
3. **Create Work Unit**: `fspec create-work-unit AUTH "User Login" --epic=user-management`
4. **Infer Scenarios from Code**:
   - Valid login → Success response with token
   - Missing email/password → 400 error
   - User not found → 401 error
   - Wrong password → 401 error
5. **Generate Feature File**: `spec/features/user-login.feature`
6. **Create Skeleton Test**: `src/routes/__tests__/auth-login.test.ts`
7. **Update foundation.json**: Add user story map diagram

## Tips for Effective Reverse Engineering

1. **Start with user-facing code** (routes, commands, UI) not internal utilities
2. **Group related functionality** into epics before creating work units
3. **Infer acceptance criteria** from error handling, validation, and business logic
4. **Document ambiguity** with comments - don't guess unclear business rules
5. **Use Example Mapping** to clarify uncertainties with humans
6. **Create minimal test skeletons** - just structure, not implementation
7. **Validate as you go** - run `fspec validate` frequently
8. **Keep user story maps updated** in foundation.json

## Common Pitfalls to Avoid

❌ **Don't implement tests** during reverse ACDD - only create structure
❌ **Don't guess business rules** - mark ambiguous scenarios and ask human
❌ **Don't skip error scenarios** - error handling reveals acceptance criteria
❌ **Don't create work units without epics** - organize into domains first
❌ **Don't forget tags** - use @phase, @component, @feature-group on features

## References

- **ACDD Workflow**: spec/CLAUDE.md
- **Example Mapping**: spec/CLAUDE.md (Example Mapping section)
- **fspec Commands**: Run `fspec --help` or `/fspec` in Claude Code
- **Gherkin Syntax**: https://cucumber.io/docs/gherkin/reference
