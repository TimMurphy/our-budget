Feature: Auto assign transactions
  As a household member
  I want transactions assigned automatically by their description
  So that I only have to approve them

Background:
  Given a NAB account "Everyday" with opening balance 500.00 at close of business "30 Oct 2026"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount | Transaction Details                 | Balance | Category |
    | 06 Nov 26 | -9.00  | BAKERY                              | 391.00  | Other    |
    | 05 Nov 26 | -20.00 | PAYPAL AUSTRALIA SAM EXAMPLE 456    | 400.00  | Other    |
    | 04 Nov 26 | -50.00 | PAYPAL AUSTRALIA SAM EXAMPLE 123    | 420.00  | Other    |
  And a budget group "People"
  And a budget account "Sam's Spending Money" in group "People"
  And a budget account "Other Spending" in group "People"
  And the transactions are imported

Scenario: Define a rule while assigning an unassigned transaction
  When I goto /bank-accounts/everyday
  And I assign "SAM EXAMPLE 123" to "Sam's Spending Money" and always assign "PAYPAL AUSTRALIA SAM EXAMPLE"
  Then "SAM EXAMPLE 123" is assigned to "Sam's Spending Money"
  And the transactions in the "Pending approval" filter are:
    | PAYPAL AUSTRALIA SAM EXAMPLE 456 |

Scenario: Auto assigned transactions wait in Pending approval, not Unassigned or Assigned
  Given an auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE" for "Sam's Spending Money"
  When I goto /bank-accounts/everyday
  Then the transactions in the "Pending approval" filter are:
    | PAYPAL AUSTRALIA SAM EXAMPLE 456 |
    | PAYPAL AUSTRALIA SAM EXAMPLE 123 |
  And "PAYPAL AUSTRALIA SAM EXAMPLE 456" is pending approval for "Sam's Spending Money"
  And the transactions in the "Unassigned" filter are:
    | BAKERY |
  And there are no transactions in the "Assigned" filter

Scenario: Approve a pending transaction
  Given an auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE" for "Sam's Spending Money"
  When I goto /bank-accounts/everyday
  And I choose the "Pending approval" filter
  And I approve "PAYPAL AUSTRALIA SAM EXAMPLE 123"
  Then the transactions in the "Pending approval" filter are:
    | PAYPAL AUSTRALIA SAM EXAMPLE 456 |
  And the transactions in the "Assigned" filter are:
    | PAYPAL AUSTRALIA SAM EXAMPLE 123 |

Scenario: Approve all pending transactions
  Given an auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE" for "Sam's Spending Money"
  When I goto /bank-accounts/everyday
  And I choose the "Pending approval" filter
  And I approve all
  Then there are no transactions in the "Pending approval" filter
  And the transactions in the "Assigned" filter are:
    | PAYPAL AUSTRALIA SAM EXAMPLE 456 |
    | PAYPAL AUSTRALIA SAM EXAMPLE 123 |

Scenario: Change a pending transaction to a different budget account
  Given an auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE" for "Sam's Spending Money"
  When I goto /bank-accounts/everyday
  And I assign "SAM EXAMPLE 123" to "Other Spending"
  Then "SAM EXAMPLE 123" is assigned to "Other Spending"
  And the transactions in the "Assigned" filter are:
    | PAYPAL AUSTRALIA SAM EXAMPLE 123 |

Scenario: Reject a pending transaction by unassigning it
  Given an auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE" for "Sam's Spending Money"
  When I goto /bank-accounts/everyday
  And I unassign "SAM EXAMPLE 123"
  Then "SAM EXAMPLE 123" is not assigned

Scenario: Newly imported transactions are auto assigned
  Given an auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE" for "Sam's Spending Money"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount | Transaction Details                 | Balance | Category |
    | 08 Nov 26 | -12.00 | PAYPAL AUSTRALIA SAM EXAMPLE 789    | 379.00  | Other    |
    | 06 Nov 26 | -9.00  | BAKERY                              | 391.00  | Other    |
    | 05 Nov 26 | -20.00 | PAYPAL AUSTRALIA SAM EXAMPLE 456    | 400.00  | Other    |
    | 04 Nov 26 | -50.00 | PAYPAL AUSTRALIA SAM EXAMPLE 123    | 420.00  | Other    |
  When the transactions are imported
  Then 1 transaction was imported and 1 auto assigned
  When I goto /bank-accounts/everyday
  Then the transactions in the "Pending approval" filter are:
    | PAYPAL AUSTRALIA SAM EXAMPLE 789 |
    | PAYPAL AUSTRALIA SAM EXAMPLE 456 |
    | PAYPAL AUSTRALIA SAM EXAMPLE 123 |

Scenario: The longest matching text wins
  Given an auto assign rule "PAYPAL" for "Other Spending"
  And an auto assign rule "paypal  australia sam example" for "Sam's Spending Money"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount | Transaction Details                 | Balance | Category |
    | 08 Nov 26 | -12.00 | PAYPAL AUSTRALIA SAM EXAMPLE 789    | 379.00  | Other    |
    | 07 Nov 26 | -5.00  | PAYPAL SOMEONE ELSE                 | 386.00  | Other    |
    | 06 Nov 26 | -9.00  | BAKERY                              | 391.00  | Other    |
    | 05 Nov 26 | -20.00 | PAYPAL AUSTRALIA SAM EXAMPLE 456    | 400.00  | Other    |
    | 04 Nov 26 | -50.00 | PAYPAL AUSTRALIA SAM EXAMPLE 123    | 420.00  | Other    |
  When the transactions are imported
  And I goto /bank-accounts/everyday
  Then "PAYPAL AUSTRALIA SAM EXAMPLE 789" is pending approval for "Sam's Spending Money"
  And "PAYPAL SOMEONE ELSE" is pending approval for "Other Spending"

Scenario: Pending transactions do not count in the budget until approved
  Given an auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE" for "Sam's Spending Money"
  Then the November 2026 activity of "Sam's Spending Money" is -0.00
  When I goto /bank-accounts/everyday
  And I choose the "Pending approval" filter
  And I approve all
  Then the November 2026 activity of "Sam's Spending Money" is -70.00

Scenario: Rules can be added and deleted on the Budgets page
  When I goto /budgets
  And I add the auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE" for "Sam's Spending Money"
  Then I should see the message "Rule added. 2 transactions assigned, waiting for approval."
  And the rules list contains "PAYPAL AUSTRALIA SAM EXAMPLE"
  When I delete the auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE"
  Then the rules list does not contain "PAYPAL AUSTRALIA SAM EXAMPLE"

Scenario: A duplicate rule is rejected
  Given an auto assign rule "PAYPAL AUSTRALIA SAM EXAMPLE" for "Sam's Spending Money"
  When I goto /budgets
  And I add the auto assign rule "paypal australia sam example" for "Other Spending"
  Then I should see the message "An auto assign rule for that text already exists"
