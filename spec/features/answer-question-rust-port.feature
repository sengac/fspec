@done
@RPC-196
@cli
@example-mapping
Feature: Port answer-question command to Rust
  """
  Core impl at rust/fspec-core/src/commands/answer_question.rs. Reuses io::ensure::ensure_work_units_file
  (auto-creates), io::locked_file::write_json_atomic (atomic write), io::time::iso8601_now (timestamps).
  WorkUnit.questions / rules / assumptions / nextRuleId all live in WorkUnit.extra (round-tripped via
  serde flatten). RuleItem is created with the same shape as add-rule: id, text, deleted, createdAt
  with post-increment of nextRuleId (default 0). Assumptions are stable-id items like rules (BUG-633).
  Two-front-doors: CLI bridge parses index with clap usize and forwards JSON workUnitId, index, answer,
  addTo to commands::answer_question::run — no domain logic in the bridge.
  """

  Background: User Story
    As a fspec maintainer porting RPC-003 commands to Rust
    I want to have the `answer-question` command added as a Rust parity port
    So that the standalone Rust binary and the dispatcher can both mark Example Mapping questions answered and optionally promote them to rules or assumptions without falling back to the TS implementation

  Scenario: addTo=rule creates a proper RuleItem with id from nextRuleId
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Should we support OAuth?',deleted:false,createdAt:'x'}] and nextRuleId=0
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Yes, Google OAuth' addTo='rule'
    Then the dispatcher returns success=true
    And the returned data contains question='Should we support OAuth?'
    And the returned data contains addedTo='rules'
    And the returned data contains addedContent='Yes, Google OAuth'
    And spec/work-units.json on disk shows AUTH-001.questions[0].selected=true
    And spec/work-units.json on disk shows AUTH-001.questions[0].answered=true
    And spec/work-units.json on disk shows AUTH-001.questions[0].answer='Yes, Google OAuth'
    And spec/work-units.json on disk shows AUTH-001.rules[0].id=0
    And spec/work-units.json on disk shows AUTH-001.rules[0].text='Yes, Google OAuth'
    And spec/work-units.json on disk shows AUTH-001.rules[0].deleted=false
    And spec/work-units.json on disk shows AUTH-001.rules[0].createdAt is a freshly bumped ISO-8601 timestamp
    And spec/work-units.json on disk shows AUTH-001.nextRuleId=1
    And spec/work-units.json on disk shows AUTH-001.updatedAt is a freshly bumped ISO-8601 timestamp

  Scenario: addTo=rule with preexisting nextRuleId increments sequentially
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}] and nextRuleId=5
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Yes' addTo='rule'
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.rules[0].id=5
    And spec/work-units.json on disk shows AUTH-001.nextRuleId=6

  Scenario: addTo=assumption appends an assumption item to the assumptions array
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Server is HTTPS only' addTo='assumption'
    Then the dispatcher returns success=true
    And the returned data contains addedTo='assumptions'
    And spec/work-units.json on disk shows AUTH-001.assumptions with one item, id 0 and text 'Server is HTTPS only'
    And spec/work-units.json on disk shows AUTH-001 has no rules added

  Scenario: addTo=none with answer marks question but does not modify rules/assumptions
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Maybe' addTo='none'
    Then the dispatcher returns success=true
    And the returned data does NOT contain addedTo
    And the returned data does NOT contain addedContent
    And spec/work-units.json on disk shows AUTH-001.questions[0].answered=true
    And spec/work-units.json on disk shows AUTH-001.questions[0].answer='Maybe'
    And spec/work-units.json on disk shows AUTH-001 has no rules added
    And spec/work-units.json on disk shows AUTH-001 has no assumptions added

  Scenario: No answer leaves answer/answered untouched but still selects the question
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 and no answer
    Then the dispatcher returns success=true
    And spec/work-units.json on disk shows AUTH-001.questions[0].selected=true
    And spec/work-units.json on disk shows AUTH-001.questions[0] has no answered field set
    And spec/work-units.json on disk shows AUTH-001.questions[0] has no answer field set

  Scenario: Missing work unit surfaces canonical error
    Given a project root tempdir with spec/work-units.json containing only AUTH-001 status=specifying
    When I dispatch answer-question with workUnitId='NOPE-001' index=0 answer='X' addTo='rule'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'NOPE-001' does not exist"
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Non-specifying status is rejected verbatim
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=backlog with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}]
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='X' addTo='rule'
    Then the dispatcher returns success=false
    And the error message contains the substring "Can only answer questions during discovery/specification phase. AUTH-001 is in 'backlog' state."
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: No questions array yields canonical error
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with no questions field
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='X' addTo='rule'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Work unit AUTH-001 has no questions'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Out-of-range index yields canonical error
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q1',deleted:false,createdAt:'x'},{id:1,text:'Q2',deleted:false,createdAt:'x'},{id:2,text:'Q3',deleted:false,createdAt:'x'}]
    When I dispatch answer-question with workUnitId='AUTH-001' index=5 answer='X' addTo='rule'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Invalid question index 5. Valid range: 0-2'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Raw-string legacy question entry is rejected
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=['legacy raw string question']
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='X' addTo='rule'
    Then the dispatcher returns success=false
    And the error message contains the substring 'Question format is invalid. Expected QuestionItem object.'
    And spec/work-units.json on disk is byte-equal to its pre-call contents

  Scenario: Auto-creates spec/work-units.json when missing then reports canonical missing-source error
    Given an empty project root directory with no spec/ subdirectory
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='X' addTo='rule'
    Then the dispatcher returns success=false
    And the error message contains the substring "Work unit 'AUTH-001' does not exist"
    And spec/work-units.json now exists on disk with the canonical empty initial structure

  Scenario: Concurrent answers to different questions are all recorded
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with unanswered questions 0 'Q0?', 1 'Q1?' and 2 'Q2?' and nextRuleId=0
    When 3 threads dispatch answer-question at once with workUnitId='AUTH-001', index=0, 1 and 2, answer='Answer 0', 'Answer 1' and 'Answer 2', addTo='rule'
    Then every dispatch returns success=true
    And spec/work-units.json on disk shows questions 0, 1 and 2 selected=true with answer 'Answer 0', 'Answer 1' and 'Answer 2'
    And spec/work-units.json on disk shows 3 rules with ids 0, 1 and 2 whose texts are 'Answer 0', 'Answer 1' and 'Answer 2' in any order
    And spec/work-units.json on disk shows AUTH-001.nextRuleId=3

  Scenario: Answering the same question twice keeps the last answer and adds both rules
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying with questions=[{id:0,text:'Q?',deleted:false,createdAt:'x'}] and nextRuleId=0
    When I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Answer A' addTo='rule'
    And I dispatch answer-question with workUnitId='AUTH-001' index=0 answer='Answer B' addTo='rule'
    Then both dispatches return success=true
    And spec/work-units.json on disk shows AUTH-001.questions[0].selected=true
    And spec/work-units.json on disk shows AUTH-001.questions[0].answer='Answer B'
    And spec/work-units.json on disk shows AUTH-001.rules as 'Answer A' with id 0 then 'Answer B' with id 1

  Scenario: The testing gate lists only the questions answer-question has not answered
    Given a project root tempdir with spec/work-units.json containing AUTH-001 status=specifying that passes review, with unanswered questions 0 'Q0?', 1 'Q1?' and 2 'Q2?'
    And spec/features/login.feature is tagged @AUTH-001 and has one scenario
    When I dispatch answer-question with workUnitId='AUTH-001' index=1 answer='Yes' addTo='none'
    And I dispatch update-work-unit-status with workUnitId='AUTH-001' status='testing' skipTemporalValidation=true
    Then the update-work-unit-status dispatch returns success=false
    And its error lists the lines '  - [0] Q0?' and '  - [2] Q2?' and no line for question 1
