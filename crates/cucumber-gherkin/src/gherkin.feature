@syntax @variety
Feature: Gherkin syntax variety
  Purpose: Provide a single, generic, English-only feature file that contains
    a wide variety of Gherkin constructs (not domain-specific) to exercise parsers.

  Background: shared preconditions
    Given a clean environment
    And a default configuration is loaded
  # Basic keywords and wildcard step

  @basic
  Scenario: Basic keyword coverage
    Given a precondition exists
    When an action is executed
    * an extra implicit step is noted
    Then an expected result is returned
    But an alternate result may also be possible
  # DocString with explicit content type

  @docstring
  Scenario: DocString with content type
    Given the following payload:
      """json
      {
        "type": "example",
        "values": [1, 2, 3]
      }
      """
    When the payload is processed
    Then the raw docstring must be preserved
  # Single-quoted docstring variant and blank docstring

  Scenario: Single-quoted docstring and empty docstring
    Given a description block:
    And an intentionally empty block:
    When both are captured
    Then the original spacing and content should remain
  # Data table with empty cells and escaped pipes and quoted cells

  @tables
  Scenario: Data table edge cases
    Given the following table:
    When the table is parsed
    Then all rows should be accessible including empty and escaped cells
  # Inline comment after a step and a standalone comment

  Scenario: Comments and inline comments
    # This is a standalone comment before steps
    Given a condition is set   # inline comment after a step
    When the condition is evaluated
    Then the result should be visible
  # Scenario Outline with multiple Examples blocks and tags on Examples

  @outline
  Scenario Outline: Parameter substitution and formats
    Given an input value <input>
    When the input is normalized
    Then the normalized value should be <normalized>

    Examples: integers
      | input | normalized |
      |    42 |         42 |
      |     0 |          0 |

    @formats
    Examples: formatted numbers
      | input   | normalized |
      | "1,000" |       1000 |
      |   2.5e2 |        250 |
  # Examples with a description and a header row containing spaces

  Scenario Outline: Strings and trimming
    Given a raw string <raw>
    When it is trimmed
    Then it becomes <trimmed>

    Examples: with spaces
      | raw         | trimmed    |
      | " leading"  | "leading"  |
      | "trailing " | "trailing" |
  # Rule block with its own Background to test scoping

  Rule: Scoped rule behavior

    Background:
      Given rule-specific setup is applied

    Scenario: Rule-scope scenario
      Given a rule-specific precondition
      When the rule executes
      Then the rule result should be recorded

    Scenario: Another rule-scope scenario
      Given multiple rule conditions
      When they are combined
      Then combined output is available
  # Steps that begin with non-keyword characters, numerics, and punctuation

    Scenario: Non-keyword line starts
      Given the line begins with an asterisk: * not a keyword here
      And the line begins with a number: 1. enumerated step text
      When punctuation is included: !@#$%^&*()[]{};':",.<>/?
      Then such content must be preserved as step text
  # Step text that contains characters which could be mistaken for a table

    Scenario: Literal pipe sequence in step text
      Given a literal string "|not|a|table|" is provided inside a step
      When the parser receives it
      Then it should not be interpreted as a data table
  # Case-insensitivity of keywords and mixed capitalization in steps

    Scenario: Mixed-case keywords and chaining
    gIvEn a mixed-case keyword line
    WhEn the step runner executes
    tHeN chaining with And and But should still work

      And additional steps continue to run
      But the order remains deterministic
  # Very long single-line step to exercise buffering limits

    Scenario: Long single-line step
      Given a very long single-line string "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
      When it is handled
      Then it should not cause a parse failure
  # Examples that include quoted values and values that look like numbers

    @examples
    Scenario Outline: Quoted and numeric values
      Given a field with value <value>
      When inspected
      Then it should be treated as <type>

      Examples:
        | value | type   |
        | "100" | string |
        |   100 | number |
  # Inline table-like text in a step argument (should remain literal)

    Scenario: Inline table-like literal
      Given the argument "col1|col2|col3" is passed literally
      When the argument is parsed
      Then it should remain a single string
  # Use of tags on examples block

    Scenario Outline: Tagged examples usage
      Given a sample <input>
      When it is processed
      Then the output should be <output>

      @fast
      Examples: fast cases
        | input | output |
        | a     | A      |
        | b     | B      |
  # EOF behavior: last scenario ends at file end (no extra newline required)

    Scenario: End-of-file handling
      Given the file may end directly after this step
      When the parser reaches EOF
      Then the final scenario must be recognized
