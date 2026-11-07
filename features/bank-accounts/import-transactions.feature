Feature: Import bank transactions
  As a household member
  I want transactions imported from my downloaded bank statements
  So that I can see what has been spent without typing it in

Scenario: Only transactions after the opening balance date are imported
  Given a NAB account "Everyday" with opening balance 100.00 at close of business "30 Oct 2026"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount | Transaction Details | Balance | Category    |
    | 04 Nov 26 | -25.00 | COFFEE SHOP         | 75.00   | Restaurants |
    | 30 Oct 26 | -10.00 | LUNCH               | 100.00  | Restaurants |
    | 29 Oct 26 | -5.00  | NEWSPAPER           | 110.00  | Other       |
  When the transactions are imported
  Then "Everyday" has 1 transaction
  And "Everyday" has a balance of 75.00

Scenario: Westpac debits and credits are both read
  Given a Westpac account "Joint" with opening balance 500.00 at close of business "30 Oct 2026"
  And the downloaded statement for "Joint" contains:
    | Date       | Narrative     | Debit Amount | Credit Amount | Balance | Categories |
    | 03/11/2026 | WAGES         |              | 1000.00       | 1490.00 | PAYMENT    |
    | 02/11/2026 | GROCERIES     | 10.00        |               | 490.00  | POS        |
  When the transactions are imported
  Then "Joint" has 2 transactions
  And "Joint" has a balance of 1490.00

Scenario: Importing again does not duplicate transactions
  Given a NAB account "Everyday" with opening balance 100.00 at close of business "30 Oct 2026"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount | Transaction Details | Balance | Category    |
    | 02 Nov 26 | -4.00  | COFFEE SHOP         | 92.00   | Restaurants |
    | 02 Nov 26 | -4.00  | COFFEE SHOP         | 96.00   | Restaurants |
  When the transactions are imported
  And the transactions are imported
  Then "Everyday" has 2 transactions
  And 0 transactions were imported and 2 skipped

Scenario: A newer statement only adds the new transactions
  Given a NAB account "Everyday" with opening balance 100.00 at close of business "30 Oct 2026"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount | Transaction Details | Balance | Category    |
    | 02 Nov 26 | -4.00  | COFFEE SHOP         | 96.00   | Restaurants |
  And the transactions are imported
  When the downloaded statement for "Everyday" contains:
    | Date      | Amount | Transaction Details | Balance | Category    |
    | 05 Nov 26 | -6.00  | BAKERY              | 90.00   | Restaurants |
    | 02 Nov 26 | -4.00  | COFFEE SHOP         | 96.00   | Restaurants |
  And the transactions are imported
  Then "Everyday" has 2 transactions
  And 1 transactions were imported and 1 skipped
  And "Everyday" has a balance of 90.00
