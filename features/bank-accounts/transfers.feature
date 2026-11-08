Feature: Transfers between bank accounts
  As a household member
  I want to mark a bank transfer between our accounts
  So that moving money around isn't counted as income or spending

Background:
  Given a NAB account "Everyday" with opening balance 1000.00 at close of business "30 Oct 2026"
  And a Westpac account "Savings" with opening balance 200.00 at close of business "30 Oct 2026"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount  | Transaction Details  | Balance | Category  |
    | 12 Nov 26 | -80.00  | GROCERIES            | 420.00  | Groceries |
    | 05 Nov 26 | -500.00 | TRANSFER TO SAVINGS  | 500.00  | Transfers |
  And the downloaded statement for "Savings" contains:
    | Date       | Narrative              | Debit Amount | Credit Amount | Balance | Categories |
    | 28/11/2026 | LATE DEPOSIT           |              | 500.00        | 1200.00 | PAYMENT    |
    | 06/11/2026 | TRANSFER FROM EVERYDAY |              | 500.00        | 700.00  | PAYMENT    |
  And the transactions are imported

Scenario: Link the two sides of a transfer
  When I goto /bank-accounts/everyday
  And I link "TRANSFER TO SAVINGS" as a transfer with "TRANSFER FROM EVERYDAY"
  Then "TRANSFER TO SAVINGS" is assigned to "Transfer to Savings"
  When I goto /bank-accounts/savings
  Then "TRANSFER FROM EVERYDAY" is assigned to "Transfer from Everyday"

Scenario: Only close matching transactions in other accounts are offered
  When I goto /bank-accounts/everyday
  Then the transfer candidates for "TRANSFER TO SAVINGS" are:
    | Savings: 2026-11-06 TRANSFER FROM EVERYDAY $500.00 |

Scenario: A linked transfer counts as assigned
  Given the transfer "TRANSFER TO SAVINGS" is linked with "TRANSFER FROM EVERYDAY"
  When I goto /bank-accounts/everyday
  And I choose the "Unassigned" filter
  Then I should see the transactions:
    | GROCERIES |
  When I choose the "Assigned" filter
  Then I should see the transactions:
    | TRANSFER TO SAVINGS |

Scenario: Nothing to link when the other side has not been imported
  When I goto /bank-accounts/everyday
  Then there is no transfer candidate for "GROCERIES"

Scenario: Unlinking a transfer unassigns both sides
  Given the transfer "TRANSFER TO SAVINGS" is linked with "TRANSFER FROM EVERYDAY"
  When I goto /bank-accounts/everyday
  And I unlink the transfer "TRANSFER TO SAVINGS"
  Then "TRANSFER TO SAVINGS" is not assigned
  When I goto /bank-accounts/savings
  Then "TRANSFER FROM EVERYDAY" is not assigned
