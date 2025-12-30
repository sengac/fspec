export function getEventStormSection(): string {
  return `## Step 4: Feature Event Storm (BEFORE Example Mapping)

**WHEN TO USE FEATURE EVENT STORM**: STOP and assess domain complexity before jumping to Example Mapping.

### Research-First Workflow

**CRITICAL**: Before deciding whether to use Feature Event Storm, FIRST research the codebase using AST analysis to understand the domain structure.

\`\`\`bash
# Step 1: Research relevant code using AST pattern matching
# Find all functions in the domain area:
fspec research --tool=ast --pattern="function $NAME" --lang=typescript --path=src/auth/

# Find classes to understand domain entities:
fspec research --tool=ast --pattern="class $NAME" --lang=typescript --path=src/auth/

# Find interfaces to understand data structures:
fspec research --tool=ast --pattern="interface $NAME" --lang=typescript --path=src/auth/

# Find async functions (often indicate external integrations or events):
fspec research --tool=ast --pattern="async function $NAME" --lang=typescript --path=src/auth/

# Step 2: Analyze findings to understand domain
# - What domain events exist in the code?
# - What commands trigger those events?
# - What business rules/policies are present?

# Step 3: If uncertain after research, ASK USER
# Share your findings and let the user decide:
# "I found 3 domain events: UserRegistered, LoginAttempted, SessionExpired.
#  Should we do Feature Event Storm to map the full authentication flow?"

# Step 4: Proceed with chosen approach
# - Feature Event Storm (if complex/unfamiliar)
# - Example Mapping (if simple/clear)
\`\`\`

**AST Pattern Tips:**
- Use \`$NAME\` as a wildcard to match any identifier
- Use \`$$$ARGS\` to match multiple arguments or body content
- Adjust \`--lang\` for your codebase: typescript, tsx, javascript, rust, python, go, etc.
- Adjust \`--path\` to focus on the relevant domain area

**Decision is SUBJECTIVE and COLLABORATIVE** - emphasize no guessing, always ask if unsure. Research builds familiarity before judgment.

### Self-Assessment Questions

Ask yourself these questions BEFORE deciding to use Event Storm:

1. **Do you understand the core domain events?**
   - Can you name the key business events that happen in this domain?
   - Are the triggers and outcomes clear?

2. **Are commands and policies clear?**
   - Do you know what user actions (commands) trigger events?
   - Are the business rules (policies) that react to events obvious?

3. **Is there significant domain complexity?**
   - Multiple bounded contexts or subdomains?
   - Complex business workflows?
   - Uncertain about event flow or timing?

### Decision: Feature Event Storm vs Skip to Example Mapping

**RUN FEATURE EVENT STORM FIRST** if:
- ❌ You answered "no" to any self-assessment question
- ❌ This is a complex domain with many entities and business rules
- ❌ Multiple teams or bounded contexts involved
- ❌ Unclear what the core domain events are
- ❌ Story estimate is 13+ points (too large - needs breakdown)

**SKIP TO EXAMPLE MAPPING** if:
- ✅ Simple CRUD operation or bug fix (2-5 points)
- ✅ Domain events, commands, and policies are obvious
- ✅ You clearly understand the business flow
- ✅ Single bounded context with straightforward logic

### Examples: When Event Storm Helped

**Example 1: Payment Processing (Complex Domain)**
- **Without Event Storm**: Jumped to Example Mapping, missed critical events (PaymentAuthorized, PaymentSettled, RefundIssued), had to rewrite scenarios 3 times
- **With Event Storm**: Discovered 12 domain events, 8 commands, 5 policies → transformed to Example Mapping → clean scenarios on first try
- **Result**: Event Storm saved 4 hours of rework

**Example 2: User Login Bug (Simple)**
- **Without Event Storm**: Went directly to Example Mapping, wrote 3 scenarios, fixed bug in 30 minutes
- **With Event Storm**: Would have been overkill, wasted time discovering obvious events (UserLoggedIn, LoginFailed)
- **Result**: Skipping Event Storm was correct decision

**Example 3: E-commerce Checkout (Multi-Context)**
- **Without Event Storm**: Missed that Inventory, Payment, and Shipping are separate bounded contexts, wrote monolithic scenarios
- **With Event Storm**: Identified 3 bounded contexts, 15 pivotal events crossing boundaries → proper separation of concerns
- **Result**: Event Storm prevented architectural mistakes

### How Event Storm Works in fspec

Event Storm is a **free-form collaborative session** (NOT field-by-field like discover-foundation).

\`\`\`bash
# Run Event Storm discovery for a work unit
fspec discover-event-storm <work-unit-id>
\`\`\`

This emits guidance (you're reading it now!) and you use these commands freely:

#### Step 1: Capture Domain Events (Orange Cards)

Domain events are **things that happened** in the business domain (past tense):

\`\`\`bash
fspec add-domain-event AUTH-001 "UserRegistered"
fspec add-domain-event AUTH-001 "EmailVerified"
fspec add-domain-event AUTH-001 "PasswordResetRequested"
\`\`\`

**Ask the human**: "What are the key business events that happen in this domain?"

#### Step 2: Capture Commands (Blue Cards)

Commands are **user actions** that trigger events (imperative):

\`\`\`bash
fspec add-command AUTH-001 "RegisterUser"
fspec add-command AUTH-001 "VerifyEmail"
fspec add-command AUTH-001 "RequestPasswordReset"
\`\`\`

**Ask the human**: "What user actions trigger these events?"

#### Step 3: Capture Policies (Purple Cards)

Policies are **reactive business rules** (WHEN event THEN command):

\`\`\`bash
fspec add-policy AUTH-001 "Send welcome email" --when "UserRegistered" --then "SendWelcomeEmail"
fspec add-policy AUTH-001 "Send verification link" --when "UserRegistered" --then "SendVerificationEmail"
\`\`\`

**Ask the human**: "What happens automatically when events occur?"

#### Step 4: Capture Hotspots (Red Cards)

Hotspots are **uncertainties, risks, or problems** to investigate:

\`\`\`bash
fspec add-hotspot AUTH-001 "Email delivery timeout" --concern "Unclear how long to wait for email verification"
fspec add-hotspot AUTH-001 "Password complexity rules" --concern "Need to clarify password requirements"
\`\`\`

**Ask the human**: "What are we uncertain about? What risks exist?"

### When to Stop Event Storm

Stop when:
1. ✅ You've captured all major domain events
2. ✅ Commands that trigger events are identified
3. ✅ Policies (reactive rules) are documented
4. ✅ Hotspots (uncertainties) are captured for later
5. ✅ Shared understanding with human is reached (aim for ~25 minutes)

Check your work:
\`\`\`bash
fspec show-event-storm <work-unit-id>
\`\`\`

### Transform Event Storm → Example Mapping

Once Event Storm is complete, transform artifacts to Example Mapping:

\`\`\`bash
fspec generate-example-mapping-from-event-storm <work-unit-id>
\`\`\`

This automatically converts:
- **Policies** → Business rules (blue cards)
- **Events** → Examples (green cards)
- **Hotspots** → Questions (red cards)

Then continue with Example Mapping workflow (add more rules/examples as needed) and generate scenarios.

### Event Storm Flow

\`\`\`
Event Storm Session
  ↓
Capture: Events → Commands → Policies → Hotspots
  ↓
fspec generate-example-mapping-from-event-storm <work-unit-id>
  ↓
Example Mapping (add more rules/examples)
  ↓
fspec generate-scenarios <work-unit-id>
  ↓
Gherkin Feature File
\`\`\`

### Available Commands

\`\`\`bash
# Discovery
fspec discover-event-storm <work-unit-id>

# Capture artifacts (use freely in any order)
fspec add-domain-event <work-unit-id> <text>
fspec add-command <work-unit-id> <text>
fspec add-policy <work-unit-id> <text> --when <event> --then <command>
fspec add-hotspot <work-unit-id> <text> --concern <description>

# View artifacts
fspec show-event-storm <work-unit-id>

# Transform to Example Mapping
fspec generate-example-mapping-from-event-storm <work-unit-id>
\`\`\`

**For more details**: Run \`fspec add-domain-event --help\`, \`fspec add-policy --help\`, etc.

`;
}
