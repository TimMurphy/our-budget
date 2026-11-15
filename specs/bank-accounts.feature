Feature: Bank accounts
  Transactions are imported from downloaded statements and assigned to budget accounts.

  Background:
    Given a Westpac bank account "Joint" with opening balance "$1,000.00"
    And a NAB bank account "Loan" with opening balance "-$5,000.00"

  Scenario: Balances are the opening balance plus imported transactions
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-11-02 | -50.00  | Supermarket |
      | 2026-11-03 | 2000.00 | Pay         |
    And the "Loan" statement contains:
      | date       | amount  | description |
      | 2026-11-03 | -300.00 | Interest    |
    When the statements are imported
    Then "Joint" has balance "$2,950.00"
    And "Loan" has balance "-$5,300.00"
    And the total of all bank accounts is "-$2,350.00"

  Scenario: Importing the same statement twice adds nothing
    Given the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Coffee      |
      | 2026-11-02 | -50.00 | Coffee      |
    When the statements are imported
    And the statements are imported
    Then "Joint" has 2 transactions

  Scenario: A re-import adds only the new rows
    Given the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Coffee      |
    And the statements have been imported
    And the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Coffee      |
      | 2026-11-04 | -20.00 | Bus         |
    When the statements are imported
    Then "Joint" has 2 transactions
    And "Joint" has balance "$930.00"

  Scenario: A row that cannot be parsed is skipped and reported
    Given the "Joint" statement file contains:
      """
      Bank Account,Date,Narrative,Debit Amount,Credit Amount,Balance,Categories,Serial
      123456,02/11/2026,Good row,10.00,,,,
      123456,not-a-date,Bad row,10.00,,,,
      123456,04/11/2026,Another good row,,5.00,,,
      """
    When the statements are imported
    Then "Joint" has 2 transactions
    And "Joint" has 1 import failure

  Scenario: A missing statement file is not an error
    Given the "Joint" statement file is missing
    When the statements are imported
    Then "Joint" has 0 transactions

  Scenario: Transactions are filtered as all, unassigned and assigned
    Given the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Coffee      |
      | 2026-11-03 | -20.00 | Bus         |
      | 2026-11-04 | -10.00 | Lunch       |
    And the statements have been imported
    When I assign "Bus" in "Joint" to "Transport"
    Then "Joint" filtered by "all" lists "Lunch, Bus, Coffee"
    And "Joint" filtered by "unassigned" lists "Lunch, Coffee"
    And "Joint" filtered by "assigned" lists "Bus"
    And "Joint" shows 2 unassigned transactions totalling "-$60.00"

  Scenario: Searching narrows the list within the filter
    Given the "Joint" statement contains:
      | date       | amount | description    |
      | 2026-11-02 | -50.00 | Coffee shop    |
      | 2026-11-03 | -20.00 | Bus            |
    And the statements have been imported
    Then "Joint" searched for "COFFEE" lists "Coffee shop"

  Scenario: A transaction can be split across budget accounts
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-11-02 | -100.00 | Shopping    |
    And the statements have been imported
    When I split "Shopping" in "Joint" into:
      | budget account | amount |
      | Groceries      | -60.00 |
      | Household      | -40.00 |
    Then "Shopping" in "Joint" is "assigned"
    And "Shopping" in "Joint" is assigned "-$60.00" to "Groceries"
    And "Shopping" in "Joint" is assigned "-$40.00" to "Household"

  Scenario: A split must add up to the transaction amount
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-11-02 | -100.00 | Shopping    |
    And the statements have been imported
    When I split "Shopping" in "Joint" into:
      | budget account | amount |
      | Groceries      | -60.00 |
      | Household      | -30.00 |
    Then the request fails with "add up"
    And "Shopping" in "Joint" is "unassigned"

  Scenario: Assigning to a new name creates the budget account in Ungrouped
    Given the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Coffee      |
    And the statements have been imported
    When I assign "Coffee" in "Joint" to "Eating Out"
    Then budget account "Eating Out" is in group "Ungrouped"

  Scenario: An assignment can be changed and removed
    Given the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Coffee      |
    And the statements have been imported
    And I assign "Coffee" in "Joint" to "Eating Out"
    When I assign "Coffee" in "Joint" to "Treats"
    Then "Coffee" in "Joint" is assigned "-$50.00" to "Treats"
    When I unassign "Coffee" in "Joint"
    Then "Coffee" in "Joint" is "unassigned"

  Scenario: The same assignment is applied to several transactions at once
    Given the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Coffee A    |
      | 2026-11-03 | -20.00 | Coffee B    |
    And the statements have been imported
    When I assign every transaction in "Joint" matching "Coffee" to "Eating Out"
    Then "Coffee A" in "Joint" is assigned "-$50.00" to "Eating Out"
    And "Coffee B" in "Joint" is assigned "-$20.00" to "Eating Out"

  Scenario: Income is a fixed budget account that credits are assigned to
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-11-03 | 2000.00 | Pay         |
    And the statements have been imported
    When I assign "Pay" in "Joint" to "Income"
    Then "Pay" in "Joint" is assigned "$2,000.00" to "Income"

  Scenario: Matching transactions in two bank accounts can be linked as a transfer
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-11-03 | -500.00 | To loan     |
    And the "Loan" statement contains:
      | date       | amount | description |
      | 2026-11-05 | 500.00 | From joint  |
    And the statements have been imported
    When I link "To loan" in "Joint" with "From joint" in "Loan"
    Then "To loan" in "Joint" is "transfer"
    And "From joint" in "Loan" is "transfer"
    And "Joint" shows 0 unassigned transactions totalling "$0.00"

  Scenario: Removing a transfer link returns both transactions to unassigned
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-11-03 | -500.00 | To loan     |
    And the "Loan" statement contains:
      | date       | amount | description |
      | 2026-11-03 | 500.00 | From joint  |
    And the statements have been imported
    And I link "To loan" in "Joint" with "From joint" in "Loan"
    When I unlink "To loan" in "Joint"
    Then "To loan" in "Joint" is "unassigned"
    And "From joint" in "Loan" is "unassigned"

  Scenario: Transfers must match in amount and date
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-11-03 | -500.00 | To loan     |
    And the "Loan" statement contains:
      | date       | amount | description |
      | 2026-11-03 | 400.00 | Wrong sum   |
      | 2026-11-20 | 500.00 | Too late    |
    And the statements have been imported
    When I link "To loan" in "Joint" with "Wrong sum" in "Loan"
    Then the request fails with "matching"
    When I link "To loan" in "Joint" with "Too late" in "Loan"
    Then the request fails with "within"

  Scenario: A transaction can be in only one link
    Given the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-11-03 | -500.00 | To loan     |
    And the "Loan" statement contains:
      | date       | amount | description |
      | 2026-11-03 | 500.00 | First       |
      | 2026-11-03 | 500.00 | Second      |
    And the statements have been imported
    And I link "To loan" in "Joint" with "First" in "Loan"
    When I link "To loan" in "Joint" with "Second" in "Loan"
    Then the request fails with "already linked"

  Scenario: Transactions before the import start date are not imported
    Given transactions before "2026-11-01" are not imported
    And the "Joint" statement contains:
      | date       | amount  | description |
      | 2026-10-31 | -100.00 | Too early   |
      | 2026-11-01 | -50.00  | First day   |
      | 2026-11-02 | -20.00  | Later       |
    When the statements are imported
    Then "Joint" has 2 transactions
    And "Joint" filtered by "all" lists "Later, First day"
    And "Joint" has balance "$930.00"
