@done
@model-selection
@bug-fix
@thinking-level
@rpc
@rust
@BUG-594
Feature: ThinkingLevel::Numeric and ThinkingChoice::NumericEffort cannot cross the bincode RPC wire
  """
  Final wire layout (bincode 1.x legacy encoding as rpc-server's transport uses it: bincode::serialize, little-endian, fixed-width integers, enum variant = u32, String = u64 byte length + UTF-8, Vec = u64 length + items, Option = u8 0 None / 1 Some + value, bool = u8 0/1). WireNumber (the numeric payload; not a Rust type clients see, only bytes): u32 tag 0 = Whole + i64 (8 bytes), u32 tag 1 = Decimal + f64 IEEE-754 (8 bytes, finite). ThinkingLevel: u32 0 Off | 1 ModelDefault | 2 On | 3 Effort(String) | 4 Numeric(WireNumber) | 5 Budget(u64). NumericEffortKind: u32 0 Whole | 1 Decimal. ThinkingChoice: u32 0 Off | 1 Effort(String) | 2 NumericEffort{kind: NumericEffortKind, min: WireNumber, max: WireNumber} | 3 TokenBudget{min: Option<u64>, max: Option<u64>} | 4 On. ThinkingOptions: u32 0 NoReasoningControl{model: String, reason: String} | 1 Levels{model: String, choices: Vec<ThinkingChoice>, takes_off: bool}. The WireNumber tag says what the number is, not what the range takes: a Decimal range may have Whole bounds (NumericEffort{Decimal, min: Whole 0, max: Whole 1}). Encoder: an integer that fits i64 is Whole, a float is Decimal, an integer above i64::MAX is an encode error. Decoder: Whole -> integer, Decimal -> float, a non-finite Decimal is a decode error. Example bytes: Numeric(0.5) = 04000000 01000000 000000000000E03F; Numeric(3) = 04000000 00000000 0300000000000000. JSON (serde_json and every human-readable serializer) keeps the plain number: {"Numeric":0.5}, {"NumericEffort":{"kind":"Whole","min":1,"max":10}}. Implemented as a serde with-module on the serde_json::Number fields that branches on Serializer::is_human_readable (false for bincode, true for JSON).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On the bincode RPC wire a numeric thinking value (the payload of ThinkingLevel::Numeric, and min and max of ThinkingChoice::NumericEffort) is a tagged number: a u32 tag, 0 = Whole followed by an i64, 1 = Decimal followed by an f64 (little-endian, bincode 1 legacy encoding); the tag order matches NumericEffortKind (Whole, Decimal)
  #   2. The variant indices do not move: ThinkingLevel Off 0, ModelDefault 1, On 2, Effort 3, Numeric 4, Budget 5; ThinkingChoice Off 0, Effort 1, NumericEffort 2, TokenBudget 3, On 4; ThinkingOptions NoReasoningControl 0, Levels 1. Only the payload of the numeric variants changes, and that payload could never be decoded before
  #   3. The JSON forms are unchanged: in every human-readable format (MCP and HTTP JSON, the JSON schema, the provider request) the numeric value stays a plain JSON number, and tui.thinkingLevels in fspec-config.json keeps its own text form
  #   4. No fallback: a whole number that does not fit an i64 cannot be encoded and the encode fails naming it; a Decimal on the wire that is not finite is refused when decoding, naming it
  #   5. Every variant of ThinkingLevel, ThinkingChoice and ThinkingOptions round-trips through bincode unchanged
  #   6. The TUI connected to a daemon over WebSocket reads the thinking options of a numeric-effort model and sets a numeric level on its session
  #
  # EXAMPLES:
  #   1. ThinkingLevel::Numeric(0.5) encodes as [4,0,0,0, 1,0,0,0, 0,0,0,0,0,0,224,63] (variant 4, Decimal, f64 0.5) and decodes back to Numeric(0.5)
  #   2. ThinkingLevel::Numeric(3) encodes as [4,0,0,0, 0,0,0,0, 3,0,0,0,0,0,0,0] (variant 4, Whole, i64 3) and decodes back to Numeric(3); Numeric(-2) round-trips as Whole -2
  #   3. ThinkingChoice::NumericEffort{kind: Decimal, min: 0, max: 1} encodes as variant 2, kind 1, min Whole 0, max Whole 1, and decodes back unchanged
  #   4. Off, ModelDefault, On, Effort("xhigh"), Numeric(0.5), Numeric(3) and Budget(8192) each round-trip through bincode; so do every ThinkingChoice variant and both ThinkingOptions variants
  #   5. Numeric(18446744073709551615) cannot be encoded: the error names 18446744073709551615 and says it does not fit a whole number on the wire
  #   6. The bytes of Numeric with tag Decimal and the f64 NaN are refused when decoding with an error saying the decimal is not finite
  #   7. serde_json of Numeric(0.5) is {"Numeric":0.5} and of NumericEffort{Whole, 1, 10} is {"NumericEffort":{"kind":"Whole","min":1,"max":10}}, as before
  #   8. The TUI's WebSocket backend connected to a daemon whose session model offers effort 0 to 1 (Decimal) receives Levels with NumericEffort{Decimal, 0, 1}, and set_thinking_level(Numeric(0.5)) reaches the session as Numeric(0.5)
  #
  # ========================================
  Background: User Story
    As a client of the fspec daemon (the TUI over a daemon connection, fspec-paseo)
    I want to send and receive numeric thinking levels and numeric effort ranges over the bincode RPC wire
    So that a model with a numeric effort can be read and set over the daemon, and every client can tell a whole number from a decimal

  Scenario: A decimal thinking level crosses the wire as a tagged Decimal
    Given the thinking level Numeric 0.5
    When it is encoded with bincode
    Then the bytes are 4,0,0,0, 1,0,0,0, 0,0,0,0,0,0,224,63
    And decoding the bytes gives Numeric 0.5

  Scenario: A whole thinking level crosses the wire as a tagged Whole
    Given the thinking levels Numeric 3 and Numeric -2
    When each is encoded with bincode
    Then Numeric 3 is the bytes 4,0,0,0, 0,0,0,0, 3,0,0,0,0,0,0,0
    And each decodes back to the same whole number

  Scenario: A numeric effort range crosses the wire with tagged bounds
    Given the choice NumericEffort with kind Decimal, min 0 and max 1
    When it is encoded with bincode
    Then the bytes are variant 2, kind 1, min Whole 0 and max Whole 1
    And decoding the bytes gives the same choice

  Scenario: Every thinking level, choice and options variant round-trips through bincode
    Given every ThinkingLevel variant, every ThinkingChoice variant and both ThinkingOptions variants
    When each is encoded and decoded with bincode
    Then each decodes to the value that was encoded
    And each variant's first four bytes are its unchanged variant index

  Scenario: A whole number that does not fit the wire is an encode error
    Given the thinking level Numeric 18446744073709551615
    When it is encoded with bincode
    Then encoding fails with an error naming 18446744073709551615 that says it does not fit a whole number on the wire

  Scenario: A decimal that is not finite is refused when decoding
    Given the bytes of a Numeric thinking level tagged Decimal holding the f64 NaN
    When they are decoded with bincode
    Then decoding fails with an error saying the decimal is not finite

  Scenario: The JSON forms keep the plain number
    Given the thinking level Numeric 0.5 and the choice NumericEffort with kind Whole, min 1 and max 10
    When each is written as JSON
    Then the level is {"Numeric":0.5}
    And the choice is {"NumericEffort":{"kind":"Whole","min":1,"max":10}}
    And reading that JSON gives the same values

  Scenario: The TUI reads and sets a numeric effort over a daemon connection
    Given a daemon over WebSocket whose session's model offers a Decimal effort from 0 to 1
    And the TUI's WebSocket backend connected to it
    When the TUI asks for the session's thinking options
    Then it receives Levels holding NumericEffort with kind Decimal, min 0 and max 1
    When the TUI sets the session's thinking level to Numeric 0.5
    Then the session's thinking level is Numeric 0.5
