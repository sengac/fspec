# Research Results: Event Storming methodology: explain policies, hotspots, external systems, and bounded contexts. What information should be captured for each artifact type in an Event Storm session?

**Source:** Perplexity AI (sonar)
**Date:** 2025-11-16T02:02:18.876Z

---

## Answer

In Event Storming, the core artifact types are **policies, hotspots, external systems, and bounded contexts**, each capturing different aspects of the domain and system design:

- **Policies** represent business rules or automated decisions triggered by domain events. They dictate how the system should react to events according to business requirements. In Event Storming, policies are captured as constraints or rules linked to commands and events. For each policy, the information to capture includes the event triggering it, the condition or rule applied, and the resulting action or command it enforces[4][11].

- **Hotspots** indicate areas of uncertainty, risk, or known problems in the domain or system design. They are points that may cause issues or require further analysis, such as vulnerabilities or unclear requirements. When documenting hotspots, note the specific event or process step where the hotspot occurs and describe the nature of the risk or concern. This helps focus discussion on potential blockers or technical challenges[4][11].

- **External systems** are systems outside the bounded context that interact with it, often providing inputs or receiving outputs. Capturing external systems involves identifying the system name, the events or commands exchanged, and any relevant integration details. This helps clarify system boundaries and integration points during modeling[6][8].

- **Bounded contexts** define explicit conceptual boundaries within the domain where particular models apply consistently. They represent subdomains that can be developed and evolved independently. In Event Storming, bounded contexts are identified by grouping related domain events, commands, and aggregates that share a coherent model. You capture the name of the bounded context, its scope (what it covers), key events within it, and any interfaces it has with other contexts through event flows or commands[1][2][3][6][10].

### What to capture for each artifact type during an Event Storm session:

| Artifact Type   | Information to Capture                                       |
|-----------------|-------------------------------------------------------------|
| **Policies**    | Event triggering the policy, conditions/rules, resulting commands/actions enforced by the policy[4][11] |
| **Hotspots**   | Event or process location of hotspot, description of risk/uncertainty/problem to investigate further[4][11] |
| **External Systems** | System name, types of exchanged events/commands, purpose of interaction, integration details[6][8] |
| **Bounded Contexts** | Name of the context, domain events included, commands and aggregates it contains, scope/limitations, interfaces to other contexts[1][2][3][6][10] |

These captured details, combined with domain events and commands, enable a comprehensive and shared understanding of the domain's complexity, system boundaries, decision points, and potential problem areas in an Event Storming workshop[1][4][6][11].

---

**Tokens Used:** 605 (prompt: 38, completion: 567)
