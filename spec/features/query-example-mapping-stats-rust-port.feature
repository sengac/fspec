@done
@querying
@cli
@RPC-260
Feature: Port query-example-mapping-stats command to Rust
  """
  Uses ensure_work_units_file from crate::io::ensure (auto-creating spec/work-units.json with canonical defaults if missing) — TS source-of-truth at src/commands/query-example-mapping-stats.ts:62 calls ensureWorkUnitsFile, so the Rust port matches that auto-create behaviour exactly.

  Reads rules/examples/questions/assumptions arrays from each WorkUnit via the typed fields exposed on the shared types::work_unit::WorkUnit struct (already populated by RPC-241..253). The arrays are exposed as RuleItem / ExampleItem / QuestionItem soft-delete records on the TS side, but the TS source uses `.length` directly — so the Rust port follows TS exactly and counts the raw array length, including soft-deleted items.

  Result struct uses #[derive(Serialize)] with explicit declaration-order fields and #[serde(rename_all = "camelCase")] to mirror TS JSON.stringify field order. Do NOT route through json!{} which alphabetizes via BTreeMap.

  CLI bridge text path is intentionally silent (TS source-of-truth at src/commands/query-example-mapping-stats.ts:171-173 only prints when format==='json' — bug we replicate). Do NOT add a render_text function — the bridge prints stdout only when args.format == 'json'.

  Both invocation paths (LLM dispatcher and clap subcommand) call the single fspec_core::commands::query_example_mapping_stats::run function; CLI bridge does only JSON arg marshalling and stdout rendering.

  The dispatcher payload shape is `{ workUnitId: Option<String>, hasQuestions: Option<bool>, questionsFor: Option<String>, format: Option<String> }`. The TS CLI registration at src/commands/query-example-mapping-stats.ts:163-178 only exposes `--format`, so the clap variant matches the TS CLI surface (--format only). The dispatcher exposes the full filter shape because TS handlers accept it via the API entry point.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Loads spec/work-units.json via the shared ensure_work_units_file helper (auto-creating the file with canonical defaults if missing)
  #   2. Per-work-unit stats record contains workUnitId, optional title, status, and four counts (rules, examples, questions, assumptions) plus completenessScore
  #   3. completenessScore is 33 if rules.length>0, +34 if examples.length>0, +33 if questions.length===0 — yielding 0/33/34/66/67/100 depending on combination
  #   4. Aggregate field workUnitsWithRules counts work units with at least one rule; workUnitsWithExamples, workUnitsWithQuestions, workUnitsWithAssumptions follow the same pattern
  #   5. avgRulesPerWorkUnit = totalRules/workUnits.length when workUnits.length>0 else 0 — emitted as raw f64 (TS division semantics, NO rounding); avgExamples/Questions/Assumptions follow the same pattern
  #   6. workUnitId filter selects only the matching work unit; if no match, the command throws an error 'Work unit '<id>' does not exist'
  #   7. hasQuestions=true filters to work units whose questions array length > 0; hasQuestions=false filters to work units whose questions array length == 0
  #   8. questionsFor='alice' filters to work units that have at least one question containing the substring '@alice'
  #   9. Result JSON field declaration order: workUnits, workUnitsWithRules, workUnitsWithExamples, workUnitsWithQuestions, workUnitsWithAssumptions, avgRulesPerWorkUnit, avgExamplesPerWorkUnit, avgQuestionsPerWorkUnit, avgAssumptionsPerWorkUnit
  #   10. The TS CLI registration only exposes --format and only prints to stdout when format==='json' (text mode prints NOTHING) — Rust replicates this exact silent-text bug
  #   11. JSON output is pretty-printed with 2-space indentation
  #   12. Dispatcher path always returns the JSON payload as a String (even when no format flag is set), but the CLI bridge only prints to stdout when format='json'
  #
  # ========================================
  Background: User Story
    As a fspec maintainer porting commands to Rust
    I want to have a Rust port of query-example-mapping-stats wired through both the LLM dispatcher and the clap subcommand
    So that the fspec daemon and the standalone Rust binary share one aggregation implementation with byte-parity to the TS source

  Scenario: Returns empty stats when work-units.json is auto-created in an empty workspace
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch query-example-mapping-stats with format='json'
    Then the dispatcher returns success=true
    Then the returned JSON has workUnits=[] (empty array)
    Then the returned JSON has workUnitsWithRules=0, workUnitsWithExamples=0, workUnitsWithQuestions=0, workUnitsWithAssumptions=0
    Then the returned JSON has avgRulesPerWorkUnit=0, avgExamplesPerWorkUnit=0, avgQuestionsPerWorkUnit=0, avgAssumptionsPerWorkUnit=0
    Then spec/work-units.json exists after the call (auto-created by ensure_work_units_file)

  Scenario: completenessScore is 100 when rules and examples are non-empty and questions is empty
    Given spec/work-units.json contains AUTH-001 with 2 rules, 1 example, 0 questions, 0 assumptions
    When I dispatch query-example-mapping-stats with format='json'
    Then the dispatcher returns success=true
    Then the returned JSON workUnits[0] has workUnitId='AUTH-001'
    Then the returned JSON workUnits[0] has rules=2, examples=1, questions=0, assumptions=0
    Then the returned JSON workUnits[0] has completenessScore=100

  Scenario: completenessScore is 0 when only questions are present
    Given spec/work-units.json contains AUTH-002 with 0 rules, 0 examples, 1 question, 0 assumptions
    When I dispatch query-example-mapping-stats with format='json'
    Then the returned JSON workUnits[0] has completenessScore=0

  Scenario: completenessScore is 66 with rules only and no questions
    Given spec/work-units.json contains AUTH-001 with 1 rule, 0 examples, 0 questions, 0 assumptions
    When I dispatch query-example-mapping-stats with format='json'
    Then the returned JSON workUnits[0] has completenessScore=66

  Scenario: completenessScore is 67 with examples only and no questions
    Given spec/work-units.json contains AUTH-001 with 0 rules, 1 example, 0 questions, 0 assumptions
    When I dispatch query-example-mapping-stats with format='json'
    Then the returned JSON workUnits[0] has completenessScore=67

  Scenario: Aggregate counts and averages reflect every retained work unit
    Given spec/work-units.json contains AUTH-001 (2 rules, 1 example, 0 questions, 0 assumptions) and AUTH-002 (0 rules, 0 examples, 1 question, 0 assumptions)
    When I dispatch query-example-mapping-stats with format='json'
    Then the returned JSON has workUnitsWithRules=1, workUnitsWithExamples=1, workUnitsWithQuestions=1, workUnitsWithAssumptions=0
    Then the returned JSON has avgRulesPerWorkUnit=1
    Then the returned JSON has avgExamplesPerWorkUnit=0.5
    Then the returned JSON has avgQuestionsPerWorkUnit=0.5

  Scenario: workUnitId filter narrows the result to a single work unit
    Given spec/work-units.json contains AUTH-001 (2 rules, 1 example), AUTH-002 (0 rules, 1 question), AUTH-003 (1 rule)
    When I dispatch query-example-mapping-stats with workUnitId='AUTH-001' and format='json'
    Then the dispatcher returns success=true
    Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-001'
    Then the returned JSON has workUnitsWithRules=1 and workUnitsWithQuestions=0
    Then the returned JSON has avgRulesPerWorkUnit=2 and avgQuestionsPerWorkUnit=0

  Scenario: workUnitId filter against a missing id surfaces a structured error
    Given spec/work-units.json contains AUTH-001, AUTH-002, AUTH-003
    When I dispatch query-example-mapping-stats with workUnitId='NOPE-999' and format='json'
    Then the dispatcher returns success=false with an error message containing the substring "Work unit 'NOPE-999' does not exist"

  Scenario: hasQuestions=true keeps only units with at least one question
    Given spec/work-units.json contains AUTH-001 with 1 question and AUTH-002 with 0 questions
    When I dispatch query-example-mapping-stats with hasQuestions=true and format='json'
    Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-001'

  Scenario: hasQuestions=false keeps only units with zero questions
    Given spec/work-units.json contains AUTH-001 with 1 question and AUTH-002 with 0 questions
    When I dispatch query-example-mapping-stats with hasQuestions=false and format='json'
    Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-002'

  Scenario: questionsFor='alice' keeps only units mentioning @alice in any question
    Given spec/work-units.json contains AUTH-001 with question '@alice should we cache?' and AUTH-002 with question '@bob review'
    When I dispatch query-example-mapping-stats with questionsFor='alice' and format='json'
    Then the returned JSON workUnits has exactly one entry whose workUnitId='AUTH-001'

  Scenario: Result JSON field order matches the declared TS shape
    Given spec/work-units.json contains AUTH-001 with 1 rule and 1 example
    When I dispatch query-example-mapping-stats with format='json'
    Then the returned JSON field declaration order is workUnits, workUnitsWithRules, workUnitsWithExamples, workUnitsWithQuestions, workUnitsWithAssumptions, avgRulesPerWorkUnit, avgExamplesPerWorkUnit, avgQuestionsPerWorkUnit, avgAssumptionsPerWorkUnit

  Scenario: Escalates malformed work-units.json as a structured parse error
    Given spec/work-units.json exists but contains the malformed bytes '{ not json'
    When I dispatch query-example-mapping-stats against that project root
    Then the dispatcher returns success=false with an error message containing the substring 'Failed to parse work-units.json'

  Scenario: Per-work-unit stats record carries the work-unit title and status verbatim
    Given spec/work-units.json contains AUTH-001 with title 'Login flow' and status 'implementing'
    When I dispatch query-example-mapping-stats with format='json'
    Then the returned JSON workUnits[0] has title='Login flow' and status='implementing'
