Feature: Opening balances
  Opening balances come from the uncommitted secrets file. They are recorded as events
  when the database is empty, and never read from the file again.

  Scenario: An empty database records the opening balances
    Given the secrets file contains:
      """
      - NAB - Home Loan: -250,000.50
      - Westpac - Our Joint Account: 1200.00
      """
    When the household bank accounts are set up
    Then "NAB - Home Loan" has balance "-$250,000.50"
    And "Westpac - Our Joint Account" has balance "$1,200.00"
    And "NAB - Nicole's Account" has balance "$0.00"

  Scenario: Later changes to the secrets file are ignored
    Given the secrets file contains:
      """
      - Westpac - Our Joint Account: 1200.00
      """
    And the household bank accounts are set up
    When the secrets file contains:
      """
      - Westpac - Our Joint Account: 999.00
      """
    And the household bank accounts are set up
    Then "Westpac - Our Joint Account" has balance "$1,200.00"

  Scenario: A missing secrets file means opening balances of zero
    When the household bank accounts are set up
    Then "NAB - Home Loan" has balance "$0.00"
