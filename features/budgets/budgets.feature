Feature: Budgets
  As a household member
  I want to group budget accounts and fund them from last month's income
  So that every dollar has a job

Scenario: Add a group and a budget account
  When I goto /budgets
  And I add the group "Food"
  And I add the budget account "Groceries" to group "Food"
  Then group "Food" contains budget account "Groceries"

Scenario: Budget account names are unique
  Given a budget group "Food"
  And a budget account "Groceries" in group "Food"
  When I goto /budgets
  And I add the budget account "groceries" to group "Food"
  Then I should see the message "Budget account name must be unique"

Scenario: Drag and drop a budget account to another group
  Given a budget group "Food"
  And a budget group "Home"
  And a budget account "Groceries" in group "Food"
  When I goto /budgets
  And I drag budget account "Groceries" to group "Home"
  Then group "Home" contains budget account "Groceries"
  And group "Food" does not contain budget account "Groceries"

Scenario Outline: Auto monthly works out the monthly amount from the cadence
  Given a budget group "Bills"
  When I goto /budgets?month=2026-11
  And I add the budget account "Rates" to group "Bills" with auto <cadence> amount <amount>
  Then the budgeted amount for "Rates" is <monthly>

  Examples:
    | cadence     | amount  | monthly |
    | yearly      | 1200.00 | 100.00  |
    | quarterly   | 300.00  | 100.00  |
    | monthly     | 100.00  | 100.00  |
    | fortnightly | 50.00   | 108.33  |
    | weekly      | 10.00   | 43.33   |

Scenario: A budgeted amount can be set for a month
  Given a budget group "Food"
  And a budget account "Groceries" in group "Food"
  When I goto /budgets?month=2026-11
  And I set the budgeted amount for "Groceries" to 400.00
  And I goto /budgets?month=2026-12
  Then the budgeted amount for "Groceries" is 0.00
  When I goto /budgets?month=2026-11
  Then the budgeted amount for "Groceries" is 400.00

Scenario: Budget amounts are funded from last month's income
  Given a NAB account "Everyday" with opening balance 100.00 at close of business "30 Oct 2026"
  And the downloaded statement for "Everyday" contains:
    | Date      | Amount  | Transaction Details | Balance | Category |
    | 12 Nov 26 | -30.00  | SHOPPING            | 3070.00 | Other    |
    | 02 Nov 26 | 3000.00 | WAGES               | 3100.00 | Income   |
  And a budget group "Income"
  And an income budget account "Wages" in group "Income"
  And a budget group "Food"
  And a budget account "Groceries" in group "Food"
  And the transactions are imported
  And the transaction "WAGES" is assigned to "Wages"
  And the transaction "SHOPPING" is assigned to "Groceries"
  When I goto /budgets?month=2026-12
  And I set the budgeted amount for "Groceries" to 800.00
  Then the summary shows "Income last month" of "$3,000.00"
  And the summary shows "Budgeted" of "$800.00"
  And the summary shows "To budget" of "$2,200.00"
