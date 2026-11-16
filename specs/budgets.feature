Feature: Budgets
  Money is budgeted into budget accounts, which belong to groups.
  Last month's income funds this month's budget.

  Background:
    Given today is "2026-11-16"
    And a Westpac bank account "Joint" with opening balance "$0.00"

  Scenario: Budget accounts start in Ungrouped and can be dragged to another group
    Given I add group "Bills"
    And I create budget account "Rates"
    Then group "Ungrouped" has budget accounts "Rates"
    When I move budget account "Rates" to group "Bills"
    Then group "Bills" has budget accounts "Rates"
    And group "Ungrouped" has budget accounts ""

  Scenario: A budget account can be created directly in a group
    Given I add group "Bills"
    When I create budget account "Rates" in group "Bills"
    Then group "Bills" has budget accounts "Rates"

  Scenario: Names are unique ignoring case
    Given I add group "Bills"
    And I create budget account "Rates"
    When I create budget account "RATES"
    Then the request fails with "already exists"
    When I add group "bills"
    Then the request fails with "already exists"

  Scenario: A group can be removed only when it has no budget accounts
    Given I add group "Bills"
    And I create budget account "Rates" in group "Bills"
    When I remove group "Bills"
    Then the request fails with "no budget accounts"
    When I move budget account "Rates" to group "Ungrouped"
    And I remove group "Bills"
    Then budget screen has no group "Bills"

  Scenario: The Income account is fixed
    When I rename budget account "Income" to "Wages"
    Then the request fails with "fixed"
    When I close budget account "Income"
    Then the request fails with "fixed"

  Scenario: Budget accounts and groups can be renamed
    Given I add group "Bills"
    And I create budget account "Rates" in group "Bills"
    When I rename budget account "Rates" to "Council rates"
    And I rename group "Bills" to "Household bills"
    Then group "Household bills" has budget accounts "Council rates"

  Scenario: The budget comes from last month's income
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-10-28 | 3000.00 | October pay |
      | 2026-11-10 | 2500.00 | November pay |
    And the statements have been imported
    And I assign "October pay" in "Joint" to "Income"
    And I assign "November pay" in "Joint" to "Income"
    And I create budget account "Groceries"
    When I budget "$500.00" for "Groceries" in "2026-11"
    Then in "2026-11" left to budget is "$2,500.00"
    And in "2026-12" left to budget is "$2,500.00"

  Scenario: Budgeting more than last month's income is flagged
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-10-28 | 1000.00 | October pay |
    And the statements have been imported
    And I assign "October pay" in "Joint" to "Income"
    And I create budget account "Groceries"
    When I budget "$1,200.00" for "Groceries" in "2026-11"
    Then in "2026-11" over budgeted is "$200.00"

  Scenario: Available carries over from month to month
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-10-12 | -150.00 | October shop |
    And the statements have been imported
    And I assign "October shop" in "Joint" to "Groceries"
    When I budget "$200.00" for "Groceries" in "2026-10"
    And I budget "$100.00" for "Groceries" in "2026-11"
    Then in "2026-10" budget account "Groceries" shows budgeted "$200.00", spent "-$150.00" and available "$50.00"
    And in "2026-11" budget account "Groceries" shows budgeted "$100.00", spent "$0.00" and available "$150.00"

  Scenario: Spending counts in the month of the transaction
    Given the "Joint" statement contains:
      | date       | amount | description |
      | 2026-10-31 | -40.00 | Late shop   |
    And the statements have been imported
    When I assign "Late shop" in "Joint" to "Groceries"
    Then in "2026-10" budget account "Groceries" shows budgeted "$0.00", spent "-$40.00" and available "-$40.00"
    And in "2026-11" budget account "Groceries" shows budgeted "$0.00", spent "$0.00" and available "$0.00"

  Scenario: Overspending is taken from next month's income
    Given the "Joint" statement contains:
      | date       | amount  | description  |
      | 2026-10-02 | 3000.00 | October pay  |
      | 2026-10-12 | -130.00 | October shop |
    And the statements have been imported
    And I assign "October pay" in "Joint" to "Income"
    And I assign "October shop" in "Joint" to "Groceries"
    When I budget "$100.00" for "Groceries" in "2026-10"
    Then in "2026-10" budget account "Groceries" shows budgeted "$100.00", spent "-$130.00" and available "-$30.00"
    And in "2026-11" budget account "Groceries" shows budgeted "$0.00", spent "$0.00" and available "$0.00"
    And in "2026-11" left to budget is "$2,970.00"

  Scenario Outline: Auto monthly converts each period to a monthly amount
    Given I create budget account "Insurance"
    When budget account "Insurance" is set to auto monthly "<period>" "<amount>"
    Then in "2026-11" budget account "Insurance" shows budgeted "<monthly>", spent "$0.00" and available "<monthly>"

    Examples:
      | period    | amount    | monthly   |
      | Yearly    | $1,200.00 | $100.00   |
      | Quarterly | $300.00   | $100.00   |
      | Monthly   | $100.00   | $100.00   |
      | Fortnight | $100.00   | $216.67   |
      | Weekly    | $100.00   | $433.33   |

  Scenario: Auto monthly never overwrites an amount set by hand
    Given I create budget account "Insurance"
    And budget account "Insurance" is set to auto monthly "Monthly" "$100.00"
    When I budget "$250.00" for "Insurance" in "2026-11"
    Then in "2026-11" budget account "Insurance" shows budgeted "$250.00", spent "$0.00" and available "$250.00"
    And in "2026-12" budget account "Insurance" shows budgeted "$100.00", spent "$0.00" and available "$350.00"

  Scenario: Auto monthly can be turned off
    Given I create budget account "Insurance"
    And budget account "Insurance" is set to auto monthly "Monthly" "$100.00"
    When auto monthly is turned off for "Insurance"
    Then in "2026-11" budget account "Insurance" shows budgeted "$0.00", spent "$0.00" and available "$0.00"

  Scenario: A budget account can be closed only when nothing is available
    Given I create budget account "Gifts"
    And I budget "$50.00" for "Gifts" in "2026-11"
    When I close budget account "Gifts"
    Then the request fails with "zero"
    When I budget "$0.00" for "Gifts" in "2026-11"
    And I close budget account "Gifts"
    Then budget screen has no budget account "Gifts"

  Scenario: Budget accounts in different groups can have the same name
    Given I add group "Bills"
    And I create budget account "Insurance" in group "Bills"
    When I create budget account "Insurance"
    Then group "Bills" has budget accounts "Insurance"
    And group "Ungrouped" has budget accounts "Insurance"
    When I create budget account "insurance" in group "Bills"
    Then the request fails with "already exists in that group"

  Scenario: Income is a reserved name
    Given I add group "Bills"
    When I create budget account "Income" in group "Bills"
    Then the request fails with "already exists"

  Scenario: A name used in two groups must be chosen by group when assigning
    Given the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -80.00 | Premium     |
    And the statements have been imported
    And I add group "Bills"
    And I create budget account "Insurance" in group "Bills"
    And I create budget account "Insurance"
    When I assign "Premium" in "Joint" to "Insurance"
    Then the request fails with "more than one budget account"
    When I assign "Premium" in "Joint" to "Insurance" in group "Bills"
    Then in "2026-11" budget account "Insurance" in group "Bills" shows budgeted "$0.00", spent "-$80.00" and available "-$80.00"
    And in "2026-11" budget account "Insurance" in group "Ungrouped" shows budgeted "$0.00", spent "$0.00" and available "$0.00"

  Scenario: A budget account cannot be renamed or moved onto a name already used in the group
    Given I add group "Bills"
    And I create budget account "Insurance" in group "Bills"
    And I create budget account "Rates" in group "Bills"
    And I create budget account "Insurance"
    When I rename budget account "Rates" to "insurance"
    Then the request fails with "already exists in that group"
    When I move the budget account "Insurance" in group "Ungrouped" to group "Bills"
    Then the request fails with "already exists in that group"
