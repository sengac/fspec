@KGRAPH-046
Feature: Calls/Imports/TypeRef edges — Java

  """
  Follows PHP extractor pattern using KindMatcher. Resolves Java package imports
  to file paths by converting dots to path separators + .java.
  """

  Background: User Story
    As a developer
    I want to get Imports, Calls, and TypeRef edges extracted from Java source files
    So that dead code detection works for Java projects via ast_dead_code

  Scenario: Extract Imports edges from Java import statements
    Given a Java file with `import com.myapp.service.UserService;`
    And the target file `com/myapp/service/UserService.java` exists in the project
    When the Java extractor processes the source file
    Then an Imports edge should be emitted from the source file to the target file
    And external `import java.util.List` imports should NOT produce edges

  Scenario: Extract Calls edges from Java method calls
    Given a Java file with method `processRequest()` that calls `validate()`
    And `validate` is defined in the same file
    When the Java extractor processes the source file
    Then a Calls edge should be emitted from `processRequest` to `validate`

  Scenario: Extract TypeRef edges from Java type annotations
    Given a Java file with `public Response handle(Request req)`
    And types `Request` and `Response` are defined in the same file
    When the Java extractor processes the source file
    Then TypeRef edges should be emitted from `handle` to `Request` and `Response`
