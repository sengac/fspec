# Example Mapping: Discovery Before Specification

## Overview

**Example Mapping** is a collaborative conversation technique for discovering requirements through four types of "cards":

- 🟨 **Yellow Card (Story)** - The user story being discussed
- 🟦 **Blue Cards (Rules)** - Business rules and acceptance criteria
- 🟩 **Green Cards (Examples)** - Concrete scenarios that illustrate the rules
- 🟥 **Red Cards (Questions)** - Uncertainties that need answers

**Goal:** Reach shared understanding before writing any specifications.

**Time box:** ~25 minutes per story (stop when red cards are answered).

---

## Why Example Mapping?

### The Problem

Without structured discovery:
- ❌ AI guesses requirements instead of asking
- ❌ Hidden complexity discovered too late
- ❌ Edge cases missed until production
- ❌ No shared understanding between human and AI

### The Solution

**Example Mapping forces conversation:**
- ✅ Surfaces uncertainties before coding
- ✅ Uncovers edge cases early
- ✅ Builds shared understanding
- ✅ Creates better specifications

### Real-World Impact

**Without Example Mapping:**
```
You: "Add user login"
AI: [writes code for email/password only]
You: "Wait, we need OAuth too"
AI: [rewrites everything]
You: "And what about password reset?"
AI: [more rewrites]
```

**With Example Mapping:**
```
You: "Let's map out user login requirements"
AI: "What login methods do you need?" (red card)
You: "Email/password plus Google and GitHub OAuth"
AI: "Password requirements?" (red card)
You: "Min 8 chars, 1 uppercase, 1 number"
AI: "What happens after 3 failed attempts?" (red card)
You: "Lock the account"
AI: "Got it! Here are the rules and examples..."
    [captures everything before writing specs]
```

**Result:** Complete understanding, no rework.

---

## The Four Cards

### Yellow Card: User Story

**What it is:** The feature being discussed.

**Format:** "As a [role], I want to [action], so that [benefit]"

**In fspec:**
```bash
fspec set-user-story AUTH-001 \
  --role "user" \
  --action "log in securely" \
  --benefit "I can access my account"
```

**Critical:** Set this BEFORE generating scenarios to avoid placeholder text.

### Blue Cards: Business Rules

**What they are:** Constraints, policies, business logic.

**Examples:**
- "Password must be at least 8 characters"
- "Account locks after 3 failed login attempts"
- "OAuth tokens expire after 1 hour"

**In fspec:**
```bash
fspec add-rule AUTH-001 "Password must be at least 8 characters with 1 uppercase and 1 number"
fspec add-rule AUTH-001 "Account locks after 3 failed login attempts"
fspec add-rule AUTH-001 "OAuth tokens expire after 1 hour"
```

**Questions to ask:**
- What constraints govern this feature?
- What policies must be followed?
- What conditions must be met?

### Green Cards: Concrete Examples

**What they are:** Specific scenarios that demonstrate the rules.

**Examples:**
- "User enters valid email and password, is authenticated"
- "User fails login 3 times, account is locked"
- "User clicks 'Login with Google', authorizes app, is authenticated"

**In fspec:**
```bash
fspec add-example AUTH-001 "User enters valid email/password and is authenticated"
fspec add-example AUTH-001 "User fails login 3 times and account is locked"
fspec add-example AUTH-001 "User logs in with Google OAuth and is authenticated"
```

**Questions to ask:**
- Can you give me a concrete example of when this rule applies?
- What would happen in the case where [specific scenario]?
- How should the system behave when [edge case]?

### Red Cards: Questions

**What they are:** Uncertainties that need human answers.

**Examples:**
- "@human: Should we support custom port ranges?"
- "@human: What happens if the port is already in use?"
- "@human: Should we support IPv6?"

**In fspec:**
```bash
fspec add-question AUTH-001 "@human: Should we support password reset via email?"
fspec add-question AUTH-001 "@human: What's the minimum session timeout?"
fspec add-question AUTH-001 "@human: Should OAuth support Microsoft?"
```

**When answered:**
```bash
fspec answer-question AUTH-001 0 --answer "Yes, reset via email with 1-hour expiry token"
fspec answer-question AUTH-001 1 --answer "15 minutes for standard users, 8 hours for 'Remember Me'"
fspec answer-question AUTH-001 2 --answer "Not in Phase 1, add to backlog as AUTH-010"
```

---

## The Example Mapping Process

### Step 0: Start with the User Story

**AI sets up the work unit:**
```bash
fspec create-story AUTH "User Login"
fspec update-work-unit-status AUTH-001 specifying
```

**AI asks about the user story:**
```
AI: "Let's define the user story for login.
     Who is the user (role)?
     What do they want to do (action)?
     Why do they want to do it (benefit)?"

You: "A user wants to log in securely so they can access their account features"

AI: fspec set-user-story AUTH-001 \
      --role "user" \
      --action "log in securely" \
      --benefit "I can access account features"
```

### Step 1: Identify Business Rules (Blue Cards)

**AI asks rule-discovery questions:**
```
AI: "What are the key business rules for login?"
    "Are there any constraints or policies?"
    "What conditions must be met for this to work?"

You: "Passwords must be at least 8 characters with 1 uppercase and 1 number.
     Accounts lock after 3 failed attempts.
     OAuth tokens expire after 1 hour."

AI: fspec add-rule AUTH-001 "Password must be at least 8 characters with 1 uppercase and 1 number"
    fspec add-rule AUTH-001 "Account locks after 3 failed login attempts"
    fspec add-rule AUTH-001 "OAuth tokens expire after 1 hour"
```

### Step 2: Gather Concrete Examples (Green Cards)

**For each rule, AI asks for examples:**
```
AI: "Can you give me a concrete example of the password rule?"

You: "User enters 'Password1' and it's accepted.
     User enters 'password' and it's rejected (no uppercase, no number)"

AI: fspec add-example AUTH-001 "User enters 'Password1' and it's accepted"
    fspec add-example AUTH-001 "User enters 'password' and it's rejected"

AI: "What about the account lockout rule?"

You: "User fails login 2 times, can still try.
     User fails login 3 times, account is locked and gets error message"

AI: fspec add-example AUTH-001 "User fails login 2 times and can still attempt login"
    fspec add-example AUTH-001 "User fails login 3 times, account locks, sees error message"
```

### Step 3: Surface Uncertainties (Red Cards)

**AI asks questions when encountering gaps:**
```
AI: fspec add-question AUTH-001 "@human: Should we support password reset?"
    fspec add-question AUTH-001 "@human: How long should sessions last?"
    fspec add-question AUTH-001 "@human: Should locked accounts auto-unlock?"

AI: "I have some questions about the login feature:

     1. Should we support password reset via email?
     2. How long should user sessions last?
     3. Should locked accounts automatically unlock after a time period?"

You: "Yes to password reset with 1-hour token expiry.
     Sessions: 15 minutes standard, 8 hours with 'Remember Me'.
     Locked accounts: Manual unlock by admin only."

AI: fspec answer-question AUTH-001 0 --answer "Yes, password reset via email with 1-hour expiry token"
    fspec answer-question AUTH-001 1 --answer "15 minutes standard, 8 hours with Remember Me"
    fspec answer-question AUTH-001 2 --answer "Manual unlock by admin only"
```

### Step 4: Iterate Until Consensus

**AI continues asking until no red cards remain:**
```
AI: "Based on your answers, I see we also need:
     - Password reset feature
     - Session management
     - Remember Me functionality
     - Account unlock by admin

     Should these be separate work units or part of AUTH-001?"

You: "Separate work units. AUTH-001 is just basic login."

AI: "Got it! I'll create backlog items for those.

     For AUTH-001, do we have shared understanding now?
     Any remaining questions or uncertainties?"

You: "Yes, we're good. Move forward with the spec."
```

### Step 5: Transform to Gherkin

**AI generates feature file from example map:**
```bash
fspec generate-scenarios AUTH-001
```

**What this does:**
- Rules (blue cards) → Background context or scenario preconditions
- Examples (green cards) → Concrete Given-When-Then scenarios
- User story (yellow card) → Background section
- Answered questions → Inform scenario details

**Generated feature file:**
```gherkin
@phase1 @authentication @story @AUTH-001
Feature: User Login

  """
  Architecture notes:
  - JWT tokens for session management
  - OAuth integration via Passport.js
  - Bcrypt for password hashing
  """

  Background: User Authentication
    As a user
    I want to log in securely
    So that I can access account features

  Scenario: Login with valid password
    Given I am on the login page
    And my password is "Password1"
    When I submit the login form
    Then I should be authenticated
    And I should see the dashboard

  Scenario: Login rejected for weak password
    Given I am on the login page
    And my password is "password"
    When I submit the login form
    Then I should see an error "Password must contain uppercase and number"

  Scenario: Account locks after 3 failed attempts
    Given I have failed login 2 times
    When I enter invalid credentials
    Then my account should be locked
    And I should see "Account locked. Contact admin to unlock"
```

---

## When to Stop Example Mapping

### Green Lights (Continue)

✅ **Continue when:**
- Red cards still exist
- Rules need more examples
- Scope is unclear
- Team doesn't have shared understanding

### Red Lights (Stop)

🛑 **Stop when:**
- No red cards remain unanswered
- Each rule has at least one concrete example
- Scope feels clear and bounded
- Human confirms shared understanding
- Time box reached (~25 minutes)

### Yellow Lights (Decide)

⚠️ **Consider stopping if:**
- Too many red cards emerge (story too large → split it)
- Questions reveal dependencies (block until resolved)
- Scope keeps expanding (re-scope or split)

---

## Example Mapping Patterns

### Pattern 1: Simple Feature (5-10 minutes)

**Characteristics:**
- 1-2 business rules
- 3-5 examples
- 0-1 questions

**Example:** "Add logout button"
- Rule: "User can log out from any page"
- Example: "User clicks logout, session ends, redirects to login"
- Questions: None

### Pattern 2: Moderate Feature (15-25 minutes)

**Characteristics:**
- 3-5 business rules
- 5-10 examples
- 2-4 questions

**Example:** "User login" (our example above)
- 3 rules (password, lockout, tokens)
- 6 examples (valid/invalid passwords, lockout scenarios)
- 3 questions (reset, sessions, unlock)

### Pattern 3: Complex Feature (25+ minutes → Split)

**Characteristics:**
- 6+ business rules
- 10+ examples
- 5+ questions

**Action:** **Split into multiple work units**

**Example:** "User management system"
- Split into: Login, Registration, Password Reset, Profile Management, Account Settings

### Pattern 4: Too Many Red Cards (Blocked)

**Characteristics:**
- Many unanswered questions
- Unknowns outweigh knowns
- Missing key information

**Action:** **Mark as blocked, research first**

**Example:** "Integrate with external API"
- Questions about API capabilities, rate limits, auth methods
- Need API documentation and access before mapping

---

## Common Mistakes

### Mistake 1: Skipping Example Mapping

**What happens:** Jump straight to feature file.

**Result:** Missing edge cases, incomplete understanding, rework.

**Fix:** Always do Example Mapping, even for "simple" features.

### Mistake 2: Not Asking Enough Questions

**What happens:** AI assumes instead of asking.

**Result:** Built the wrong thing.

**Fix:** Add red cards liberally. "When in doubt, ask."

### Mistake 3: Too Many Examples

**What happens:** Example Mapping takes hours.

**Result:** Analysis paralysis, delayed implementation.

**Fix:** One example per rule is often enough. More for complex rules.

### Mistake 4: Ignoring Red Cards

**What happens:** Questions left unanswered.

**Result:** Specifications have gaps, ambiguity remains.

**Fix:** Answer ALL red cards before generating scenarios.

### Mistake 5: No Time Boxing

**What happens:** Example Mapping goes on forever.

**Result:** Diminishing returns, wasted time.

**Fix:** Set 25-minute timer. If not done, story is too large → split it.

---

## fspec Commands for Example Mapping

### User Story
```bash
fspec set-user-story <work-unit-id> \
  --role "..." \
  --action "..." \
  --benefit "..."
```

### Rules (Blue Cards)
```bash
fspec add-rule <work-unit-id> "Rule text"
fspec remove-rule <work-unit-id> <index>
```

### Examples (Green Cards)
```bash
fspec add-example <work-unit-id> "Example text"
fspec remove-example <work-unit-id> <index>
```

### Questions (Red Cards)
```bash
fspec add-question <work-unit-id> "@human: Question text?"
fspec answer-question <work-unit-id> <index> --answer "Answer text"
fspec remove-question <work-unit-id> <index>
```

### Attachments (Supporting Files)
```bash
fspec add-attachment <work-unit-id> <file-path> --description "Description"
fspec list-attachments <work-unit-id>
fspec remove-attachment <work-unit-id> <file-name>
```

### View Example Map
```bash
fspec show-work-unit <work-unit-id>
```

### Generate Feature File
```bash
fspec generate-scenarios <work-unit-id>
fspec generate-scenarios <work-unit-id> --feature=custom-name
```

---

## Tips for Effective Example Mapping

### For Humans

✅ **DO:**
- Answer questions honestly (don't guess)
- Provide concrete examples
- Say "I don't know" if uncertain
- Challenge assumptions
- Think about edge cases

❌ **DON'T:**
- Rush through discovery
- Assume AI understands context
- Skip examples ("it's obvious")
- Ignore red cards
- Say "yes" to everything

### For AI Agents

✅ **DO:**
- Ask questions when uncertain
- Capture rules and examples
- Surface edge cases
- Time box discovery (~25 min)
- Confirm shared understanding

❌ **DON'T:**
- Guess requirements
- Skip red cards
- Accept vague answers
- Move forward with unanswered questions
- Over-complicate simple features

---

## Example Mapping in Practice

### Full Session Example

```
You: /fspec Create a story for user registration

AI: I'll create AUTH-002 for user registration and start Example Mapping...

    STEP 0: User Story
    Who is registering (role)?

You: New users who want to create an account

AI: What do they want to do (action)?

You: Register with email and password

AI: Why do they want to do it (benefit)?

You: So they can access the application

AI: fspec set-user-story AUTH-002 \
      --role "new user" \
      --action "register with email and password" \
      --benefit "I can access the application"

    STEP 1: Business Rules
    What are the key rules for registration?

You: Email must be unique. Password requirements same as login.
     Must verify email before full access.

AI: fspec add-rule AUTH-002 "Email address must be unique"
    fspec add-rule AUTH-002 "Password must be at least 8 characters with 1 uppercase and 1 number"
    fspec add-rule AUTH-002 "Email verification required before full account access"

    STEP 2: Concrete Examples
    Can you give examples of the unique email rule?

You: User tries to register with existing email, gets error.
     User registers with new email, account created.

AI: fspec add-example AUTH-002 "User registers with existing email and gets error"
    fspec add-example AUTH-002 "User registers with new email and account is created"

    STEP 3: Questions
    I have some questions:

    fspec add-question AUTH-002 "@human: Should we allow social registration (OAuth)?"
    fspec add-question AUTH-002 "@human: How long is the email verification link valid?"
    fspec add-question AUTH-002 "@human: What happens if user never verifies email?"

    1. Should we allow registration via Google/GitHub OAuth?
    2. How long should the email verification link be valid?
    3. What happens if a user never verifies their email?

You: No OAuth for registration in Phase 1.
     Verification link valid for 24 hours.
     Unverified accounts deleted after 7 days.

AI: fspec answer-question AUTH-002 0 --answer "No OAuth registration in Phase 1"
    fspec answer-question AUTH-002 1 --answer "Verification link valid for 24 hours"
    fspec answer-question AUTH-002 2 --answer "Unverified accounts deleted after 7 days"

    STEP 4: Confirm Understanding
    Do we have shared understanding? Any other questions?

You: Yes, looks complete!

AI: Great! I'll generate the feature file now...
    fspec generate-scenarios AUTH-002

    ✓ Created spec/features/user-registration.feature
    ✓ 5 scenarios generated from example map
    ✓ Validated Gherkin syntax

    Ready to move to testing phase!
```

---

## Resources

- **Original technique:** [Example Mapping Introduction](https://cucumber.io/blog/bdd/example-mapping-introduction/)
- **Matt Wynne's guide:** [Example Mapping](https://cucumber.io/blog/bdd/example-mapping-webinar/)
- **In fspec:** Commands listed above, `fspec help discovery`

---

**Example Mapping: The conversation that prevents chaos.**

**Ask questions. Gather examples. Build the right thing.**
