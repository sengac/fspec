@BUG-582
@done
@PROV-175
@session
@profiles
@rust
Feature: Profile config numbers are validated instead of coerced
  """
  rust/sessions/src/profile_sections/config.rs de_opt_u32_lenient /
  de_u32_lenient / coerce_u32 saturated values above u32::MAX, truncated
  floats (80.7 -> 80), turned non-numeric, negative and non-finite values
  into None (an optional limit silently became "undeclared") and an invalid
  required value into 0; and load_local_server_profiles skipped a profile
  that failed to parse and returned an empty list on any read or parse error.
  Now the deserializers accept whole numbers from 0 to u32::MAX (an integral
  float such as 128000.0 is the same number — an explicit rule) and reject
  anything else naming the value; load_local_server_profiles returns Result:
  a missing file or profiles section is no profiles, while an unreadable or
  malformed file, an unresolvable user directory and a profile that fails to
  parse are errors naming the profile and field path (serde_path_to_error).
  Callers propagate: OpenAI profile env vars, the session's model vision
  resolution and the local-profile model selector sections.
  Tests: rust/sessions/tests/prov175_profile_numbers_are_validated.rs.
  """

  Background: User Story
    As a user editing model profiles in fspec-config.json
    I want to have an invalid profile number reported naming the profile, field and value
    So that a typo never silently becomes a different limit or makes the profile disappear

  Scenario: A fractional profile number is an error naming the profile, field and value
    Given profile "local" declares contextWindow 80.7
    When the local server profiles are loaded
    Then loading fails naming "local", "contextWindow" and 80.7

  Scenario: An oversized profile number is an error, not saturated
    Given profile "local" declares contextWindow 5000000000
    When the local server profiles are loaded
    Then loading fails naming 5000000000

  Scenario: An integral float is the same whole number
    Given profile "local" declares contextWindow 128000.0
    When the local server profiles are loaded
    Then profile "local" has a context window of 128000
