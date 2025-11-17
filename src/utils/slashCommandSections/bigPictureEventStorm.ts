/**
 * Big Picture Event Storming Section
 *
 * Documentation for foundation-level Event Storming workflow
 */

export function getBigPictureEventStormSection(): string {
  return `
## Step 1.5a: Big Picture Event Storming (Foundation Level)

**WHEN TO USE**: Immediately after completing foundation discovery (\`fspec discover-foundation --finalize\`).

**CRITICAL**: Big Picture Event Storming is conducted at the FOUNDATION level to establish domain architecture before creating individual work units. This is different from work unit-level Event Storming (Step 1.6).

### What is Big Picture Event Storming?

Big Picture Event Storming is a collaborative session to discover:
- **Bounded Contexts** - Strategic boundaries in the domain
- **Aggregates** - Core domain entities within each bounded context
- **Domain Events** - Key business events that occur in the domain
- **Commands** - User/system actions that trigger events

This information is stored in \`foundation.json\` eventStorm field and used for:
1. **Tag Ontology Generation** - Derive component/feature tags from bounded contexts
2. **Domain Architecture Visualization** - Generate bounded context maps
3. **EXMAP-004 Integration** - Tag discovery from Event Storm artifacts

### when to conduct Big Picture Event Storming

**Trigger**: After \`fspec discover-foundation --finalize\` completes successfully

**Typical workflow**:
\`\`\`bash
# 1. Foundation discovery complete
fspec discover-foundation --finalize
✓ Generated spec/foundation.json
✓ Created work unit FOUND-XXX: Big Picture Event Storming

# 2. Move work unit to specifying
fspec update-work-unit-status FOUND-XXX specifying

# 3. Conduct Big Picture Event Storm (commands below)
\`\`\`

### Big Picture Event Storm Commands

**Foundation-level commands** (different from work unit-level):

\`\`\`bash
# Add bounded context
fspec add-foundation-bounded-context "Work Management"
fspec add-foundation-bounded-context "Specification"
fspec add-foundation-bounded-context "Testing & Validation"

# Add aggregate to bounded context
fspec add-aggregate-to-foundation "Work Management" "WorkUnit"
fspec add-aggregate-to-foundation "Work Management" "Epic"
fspec add-aggregate-to-foundation "Specification" "Feature"

# Add domain event to bounded context
fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated"
fspec add-domain-event-to-foundation "Work Management" "WorkUnitStatusChanged"
fspec add-domain-event-to-foundation "Specification" "FeatureFileCreated"

# Add command to bounded context
fspec add-command-to-foundation "Work Management" "CreateWorkUnit"
fspec add-command-to-foundation "Work Management" "UpdateWorkUnitStatus"

# View foundation Event Storm
fspec show-foundation-event-storm
\`\`\`

### Foundation vs Work Unit Event Storming

| Aspect | Foundation (Big Picture) | Work Unit (Process Modeling) |
|--------|-------------------------|------------------------------|
| **Scope** | entire domain | single feature |
| **Storage** | \`foundation.json\` eventStorm field | \`work-units.json\` per work unit |
| **Bounded Contexts** | Strategic boundaries | Tactical scope |
| **Commands** | \`add-foundation-bounded-context\` | \`add-bounded-context <work-unit-id>\` |
| **When** | once after foundation discovery | many times, per story |
| **Output** | tag ontology, architecture maps | scenarios, feature files |

### Big Picture Event Storm Session Flow

**CRITICAL**: Ask human for domain knowledge. DO NOT guess bounded contexts or domain events.

1. **Identify Bounded Contexts**
   \`\`\`
   AI: "What are the major strategic boundaries in this system?"
   AI: "What are the distinct domains or subdomains?"

   Human: "Work Management, Specification, Testing"

   fspec add-foundation-bounded-context "Work Management"
   fspec add-foundation-bounded-context "Specification"
   fspec add-foundation-bounded-context "Testing"
   \`\`\`

2. **Identify Aggregates per Context**
   \`\`\`
   AI: "What are the core entities in the Work Management bounded context?"

   Human: "WorkUnit and Epic"

   fspec add-aggregate-to-foundation "Work Management" "WorkUnit"
   fspec add-aggregate-to-foundation "Work Management" "Epic"
   \`\`\`

3. **Identify Domain Events per Context**
   \`\`\`
   AI: "What key business events happen in Work Management?"

   Human: "Work units are created, status changes, dependencies added"

   fspec add-domain-event-to-foundation "Work Management" "WorkUnitCreated"
   fspec add-domain-event-to-foundation "Work Management" "WorkUnitStatusChanged"
   fspec add-domain-event-to-foundation "Work Management" "WorkUnitDependencyAdded"
   \`\`\`

4. **View and Validate**
   \`\`\`bash
   fspec show-foundation-event-storm
   \`\`\`

### When to Stop Big Picture Event Storming

Stop when:
1. ✅ All major bounded contexts identified (typically 3-8)
2. ✅ Core aggregates per context captured (1-5 per context)
3. ✅ Key domain events identified (5-15 per context)
4. ✅ Human confirms domain architecture is accurate

**Do NOT**:
- ❌ Capture every possible event (only pivotal domain events)
- ❌ Model implementation details (focus on business domain)
- ❌ Skip human confirmation (always verify with stakeholder)

### Integration with Tag Ontology (EXMAP-004)

Once Big Picture Event Storm is complete, use it for tag discovery:

\`\`\`bash
# Generate component tags from bounded contexts
fspec derive-tags-from-foundation

# Output:
# ✓ Created 15 component tags
# ✓ Created 12 feature group tags
# ✓ Created 47 hierarchical relationships
\`\`\`

This populates \`spec/tags.json\` with domain-driven tags automatically generated from your bounded contexts.

### Example: fspec Project Big Picture Event Storm

**Bounded Contexts**:
- Work Management (work units, epics, Kanban)
- Specification (Gherkin, Example Mapping, Event Storm)
- Testing & Validation (coverage, validation, quality gates)
- Infrastructure (CLI, hooks, checkpoints)

**Aggregates** (Work Management):
- WorkUnit
- Epic
- Dependency

**Domain Events** (Work Management):
- WorkUnitCreated
- WorkUnitStatusChanged
- WorkUnitBlocked
- EpicCreated
- DependencyAdded

**Commands** (Work Management):
- CreateWorkUnit
- UpdateWorkUnitStatus
- BlockWorkUnit
- CreateEpic
- AddDependency

### After Big Picture Event Storm

Once complete, move to work unit-level Event Storming (Step 1.6) when creating individual stories.

---
`;
}
