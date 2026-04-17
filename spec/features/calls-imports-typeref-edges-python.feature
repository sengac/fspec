@KGRAPH-043
Feature: Calls/Imports/TypeRef edges — Python
  """
  Uses edge_helpers for shared call/typeref extraction.
  Python import resolution: dot-separated modules to slash-separated paths + .py.
  Supports `import X`, `from X import Y`, and relative imports.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Python `import` and `from X import Y` statements must produce Imports edges using dot-to-slash module path resolution
  #   2. Same-file function calls must produce Calls edges between functions in the same file
  #
  # EXAMPLES:
  #   1. `from click.core import BaseCommand` → Imports edge from source file to click/core.py if local
  #   2. `def main():` calling `validate_config()` in same file → Calls edge from main to validate_config
  #
  # ========================================
  Background: User Story
    As a developer
    I want to get Imports, Calls, and TypeRef edges extracted from Python source files
    So that dead code detection works for Python projects via ast_dead_code

  Scenario: Extract Imports edges from Python import statements
    Given a Python file with `from click.core import BaseCommand`
    And the target file `click/core.py` exists in the project
    When the Python extractor processes the source file
    Then an Imports edge should be emitted from the source file to `click/core.py`

  Scenario: Extract Calls edges from Python function calls
    Given a Python file with `def main():` that calls `validate_config()`
    And `validate_config` is defined in the same file
    When the Python extractor processes the source file
    Then a Calls edge should be emitted from `main` to `validate_config`
