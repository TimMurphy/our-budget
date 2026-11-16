Feature: Auto assign
  An auto assign rule is a phrase in a bank transaction description and the budget account to assign it to.
  Transactions assigned by a rule are pending approval until they are approved.

  Background:
    Given today is "2026-11-16"
    And a Westpac bank account "Joint" with opening balance "$0.00"
    And I create budget account "Groceries"

  Scenario: Creating a rule assigns the unassigned transactions that match, pending approval
    Given the "Joint" statement contains:
      | date       | amount | description             |
      | 2026-11-02 | -50.00 | POS WOOLWORTHS KYABRAM  |
      | 2026-11-03 | -30.00 | woolworths stanhope     |
      | 2026-11-04 | -10.00 | Bakery                  |
      | 2026-11-05 | -20.00 | Woolworths manual       |
    And the statements have been imported
    And I assign "Woolworths manual" in "Joint" to "Treats"
    When I add an auto assign rule "Woolworths" for "Groceries"
    Then the rule assigned 2 transactions
    And "POS WOOLWORTHS KYABRAM" in "Joint" is assigned "-$50.00" to "Groceries"
    And "POS WOOLWORTHS KYABRAM" in "Joint" is pending approval
    And "woolworths stanhope" in "Joint" is pending approval
    And "Bakery" in "Joint" is "unassigned"
    And "Woolworths manual" in "Joint" is assigned "-$20.00" to "Treats"
    And "Woolworths manual" in "Joint" is not pending approval

  Scenario: Importing assigns new transactions that match a rule
    Given I add an auto assign rule "bus fare" for "Groceries"
    And the "Joint" statement contains:
      | date       | amount | description   |
      | 2026-11-02 | -3.00  | City Bus Fare |
      | 2026-11-03 | -9.00  | Bookshop      |
    When the statements are imported
    Then the import auto assigned 1 transaction
    And "City Bus Fare" in "Joint" is assigned "-$3.00" to "Groceries"
    And "City Bus Fare" in "Joint" is pending approval
    And "Bookshop" in "Joint" is "unassigned"

  Scenario: Importing does not reassign a transaction that was unassigned by hand
    Given I add an auto assign rule "woolworths" for "Groceries"
    And the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Woolworths A |
    And the statements have been imported
    And I unassign "Woolworths A" in "Joint"
    And the "Joint" statement contains:
      | date       | amount | description  |
      | 2026-11-02 | -50.00 | Woolworths A |
      | 2026-11-03 | -20.00 | Woolworths B |
    When the statements are imported
    Then "Woolworths A" in "Joint" is "unassigned"
    And "Woolworths B" in "Joint" is pending approval

  Scenario: The first rule that matches wins
    Given I create budget account "Fruit"
    And I add an auto assign rule "fresh" for "Fruit"
    And I add an auto assign rule "market" for "Groceries"
    And the "Joint" statement contains:
      | date       | amount | description  |
      | 2026-11-02 | -12.00 | Fresh Market |
    When the statements are imported
    Then "Fresh Market" in "Joint" is assigned "-$12.00" to "Fruit"

  Scenario: Approving a transaction clears pending approval and keeps the assignment
    Given the "Joint" statement contains:
      | date       | amount | description |
      | 2026-11-02 | -50.00 | Woolworths A |
      | 2026-11-03 | -20.00 | Woolworths B |
    And the statements have been imported
    And I add an auto assign rule "woolworths" for "Groceries"
    When I approve "Woolworths A" in "Joint"
    Then "Woolworths A" in "Joint" is not pending approval
    And "Woolworths A" in "Joint" is assigned "-$50.00" to "Groceries"
    And "Woolworths B" in "Joint" is pending approval

  Scenario: Several transactions can be approved at once
    Given the "Joint" statement contains:
      | date       | amount | description  |
      | 2026-11-02 | -50.00 | Woolworths A |
      | 2026-11-03 | -20.00 | Woolworths B |
    And the statements have been imported
    And I add an auto assign rule "woolworths" for "Groceries"
    When I approve every pending transaction in "Joint"
    Then "Joint" filtered by "pending" lists ""

  Scenario: Changing an auto assigned transaction by hand approves it
    Given the "Joint" statement contains:
      | date       | amount | description  |
      | 2026-11-02 | -50.00 | Woolworths A |
    And the statements have been imported
    And I add an auto assign rule "woolworths" for "Groceries"
    When I assign "Woolworths A" in "Joint" to "Treats"
    Then "Woolworths A" in "Joint" is assigned "-$50.00" to "Treats"
    And "Woolworths A" in "Joint" is not pending approval

  Scenario: The pending approval filter lists only transactions waiting for approval
    Given the "Joint" statement contains:
      | date       | amount | description  |
      | 2026-11-02 | -50.00 | Woolworths A |
      | 2026-11-03 | -20.00 | Bakery       |
      | 2026-11-04 | -10.00 | Library      |
    And the statements have been imported
    And I assign "Library" in "Joint" to "Treats"
    And I add an auto assign rule "woolworths" for "Groceries"
    Then "Joint" filtered by "pending" lists "Woolworths A"
    And "Joint" filtered by "assigned" lists "Library, Woolworths A"
    And "Joint" filtered by "unassigned" lists "Bakery"
    And "Joint" has 1 transaction pending approval

  Scenario: A phrase can have only one rule and the budget account must exist
    Given I add an auto assign rule "woolworths" for "Groceries"
    When I add an auto assign rule "WOOLWORTHS" for "Groceries"
    Then the request fails with "already exists"
    When I add an auto assign rule "bakery" for "No such account"
    Then the request fails with "no such budget account"

  Scenario: Removing a rule keeps the assignments it made and stops matching new transactions
    Given I add an auto assign rule "woolworths" for "Groceries"
    And the "Joint" statement contains:
      | date       | amount | description  |
      | 2026-11-02 | -50.00 | Woolworths A |
    And the statements have been imported
    When I remove the auto assign rule "woolworths"
    Then there are 0 auto assign rules
    And "Woolworths A" in "Joint" is assigned "-$50.00" to "Groceries"
    When the "Joint" statement contains:
      | date       | amount | description  |
      | 2026-11-02 | -50.00 | Woolworths A |
      | 2026-11-03 | -20.00 | Woolworths B |
    And the statements are imported
    Then "Woolworths B" in "Joint" is "unassigned"
