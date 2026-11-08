Feature: Bank account transactions
  As a household member
  I want to see a bank account's transactions and assign them to budget accounts
  So that I know where the money went

Background:
  Given a NAB account "Everyday" with opening balance 100.00 at close of business "30 Oct 2026"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount   | Transaction Details | Balance | Category    |
    | 05 Nov 26 | -60.00   | SUPERMARKET         | 80.00   | Groceries   |
    | 03 Nov 26 | -12.50   | PHARMACY            | 140.00  | Medical     |
    | 02 Nov 26 | 200.00   | WAGES               | 152.50  | Income      |
  And a budget group "Food"
  And a budget account "Groceries" in group "Food"
  And a budget account "Treats" in group "Food"
  And a budget group "Health"
  And a budget account "Medical" in group "Health"
  And the transactions are imported

Scenario: All transactions are shown newest first
  When I goto /bank-accounts/everyday
  Then I should see the transactions:
    | SUPERMARKET |
    | PHARMACY    |
    | WAGES       |

Scenario: Filter to unassigned and assigned transactions
  Given the transaction "PHARMACY" is assigned to "Medical"
  When I goto /bank-accounts/everyday
  And I choose the "Unassigned" filter
  Then I should see the transactions:
    | SUPERMARKET |
    | WAGES       |
  When I choose the "Assigned" filter
  Then I should see the transactions:
    | PHARMACY |
  When I choose the "All transactions" filter
  Then I should see the transactions:
    | SUPERMARKET |
    | PHARMACY    |
    | WAGES       |

Scenario: Assign a transaction to a budget account
  When I goto /bank-accounts/everyday
  And I assign "SUPERMARKET" to "Groceries"
  Then "SUPERMARKET" is assigned to "Groceries"

Scenario: Split a transaction across budget accounts
  When I goto /bank-accounts/everyday
  And I split "SUPERMARKET" across:
    | Groceries | -45.00 |
    | Treats    | -15.00 |
  Then "SUPERMARKET" is assigned to "Groceries -$45.00, Treats -$15.00"

Scenario: A split must add up to the transaction amount
  When I goto /bank-accounts/everyday
  And I split "SUPERMARKET" across:
    | Groceries | -45.00 |
    | Treats    | -10.00 |
  Then I should see the message "Amounts must add up to the transaction amount"
  And "SUPERMARKET" is not assigned

Scenario: An assignment can be removed
  Given the transaction "PHARMACY" is assigned to "Medical"
  When I goto /bank-accounts/everyday
  And I unassign "PHARMACY"
  Then "PHARMACY" is not assigned

Scenario: Create a new budget account while assigning
  When I goto /bank-accounts/everyday
  And I assign "SUPERMARKET" to a new budget account "Pets" in group "Food"
  Then "SUPERMARKET" is assigned to "Pets"
  And the budget account "Pets" is in group "Food"

Scenario: Create a new budget account in a new group while assigning
  When I goto /bank-accounts/everyday
  And I assign "SUPERMARKET" to a new budget account "Cleaning" in a new group "Household"
  Then "SUPERMARKET" is assigned to "Cleaning"
  And the budget account "Cleaning" is in group "Household"

Scenario: A new budget account name must be unique
  When I goto /bank-accounts/everyday
  And I assign "SUPERMARKET" to a new budget account "groceries" in group "Food"
  Then I should see the message "Budget account name must be unique"
  And "SUPERMARKET" is not assigned
