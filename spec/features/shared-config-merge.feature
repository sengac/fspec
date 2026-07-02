@done
@configuration
@utils
@persistence
@CONFIG-008
Feature: Shared Rust fspec-config.json module with project-scope deep-merge
  """
  config representation is serde_json::Value (untyped object), mirroring the untyped TS object
  Cores are path-injectable (load_config_with_dirs / write_config_with_dirs) with thin global wrappers using get_data_dir() + std::env::current_dir()
  Module lives in codelet-common because get_data_dir() lives there and multiple crates consume it
  No unwrap() in fallible lib paths; load/write return Result<_, String>
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. User-only config loads its keys when no project file exists
  #   2. Project config deep-merges over user config (project wins per key)
  #   3. Nested objects merge recursively; sibling keys at each level are preserved
  #   4. Arrays and scalar values replace (not merge)
  #   5. Missing files load as empty object (no error)
  #   6. Empty or whitespace-only file loads as empty object
  #   7. Invalid JSON returns an error naming the offending path
  #   8. write_config(User) writes <data_dir>/fspec-config.json, creating the directory
  #   9. write_config(Project) writes <cwd>/spec/fspec-config.json, creating the directory
  #   10. Round-trip: write then load returns the written value merged with the other scope
  #
  # EXAMPLES:
  #   1. User config {model:'opus'} and no project file loads {model:'opus'}
  #   2. User {a:1,b:2} + project {b:3} loads {a:1,b:3}
  #   3. User {ui:{theme:'dark',font:12}} + project {ui:{theme:'light'}} loads {ui:{theme:'light',font:12}}
  #   4. User {tags:['a','b']} + project {tags:['c']} loads {tags:['c']}
  #   5. A file containing only whitespace loads as {}
  #   6. A file containing '{invalid' returns Err mentioning the path
  #   7. write_config(Project,{x:1}) then load returns {x:1} merged with user keys
  #
  # ========================================
  Background: User Story
    As a Rust TUI port
    I want to load and write the shared fspec-config.json with user+project deep-merge
    So that settings interoperate with the TypeScript fspec implementation

  Scenario: User-only config loads when no project file exists
    Given a user config file containing {"model":"opus"}
    And no project config file exists
    When the config is loaded
    Then the loaded config equals {"model":"opus"}

  Scenario: Project config deep-merges over user config per key
    Given a user config file containing {"a":1,"b":2}
    And a project config file containing {"b":3}
    When the config is loaded
    Then the loaded config equals {"a":1,"b":3}

  Scenario: Nested objects merge recursively preserving sibling keys
    Given a user config file containing {"ui":{"theme":"dark","font":12}}
    And a project config file containing {"ui":{"theme":"light"}}
    When the config is loaded
    Then the loaded config equals {"ui":{"theme":"light","font":12}}

  Scenario: Arrays and scalars replace instead of merging
    Given a user config file containing {"tags":["a","b"]}
    And a project config file containing {"tags":["c"]}
    When the config is loaded
    Then the loaded config equals {"tags":["c"]}

  Scenario: Missing files load as an empty object without error
    Given no user config file exists
    And no project config file exists
    When the config is loaded
    Then the loaded config equals {}

  Scenario: Whitespace-only file loads as an empty object
    Given a user config file containing only whitespace
    And no project config file exists
    When the config is loaded
    Then the loaded config equals {}

  Scenario: Invalid JSON returns an error naming the offending path
    Given a user config file containing invalid JSON text "{invalid"
    When the config is loaded
    Then loading returns an error mentioning the user config path

  Scenario: Writing user scope creates the data directory and file
    Given a data directory that does not yet exist
    When the config {"x":1} is written to the user scope
    Then the file <data_dir>/fspec-config.json exists with content {"x":1}

  Scenario: Writing project scope creates the spec directory and file
    Given a working directory with no spec directory
    When the config {"y":2} is written to the project scope
    Then the file <cwd>/spec/fspec-config.json exists with content {"y":2}

  Scenario: Round-trip write then load merges with the other scope
    Given a user config file containing {"u":1}
    When the config {"x":1} is written to the project scope
    And the config is loaded
    Then the loaded config equals {"u":1,"x":1}
