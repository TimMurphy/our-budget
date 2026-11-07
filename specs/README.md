# Our Budget

Our Budget is a SPA website with a SQLite database, data/databases/our-budget.db, to track our spending and savings.

Opening balances are as at cob 30/10/2026 and are kept in [secrets.md](secrets.md), which is not committed.

## Bank Accounts

- Transactions are imported periodically.
- Each transaction must be assignable to one or more budget accounts.
- Filters
    - All transactions
    - Unassigned
    - Assigned

### Accounts

#### NAB - Home Loan

Opening balance: see [secrets.md](secrets.md)

data/bank-transactions/home-loan.csv

#### NAB - Nicole's Account

Opening balance: see [secrets.md](secrets.md)

data/bank-transactions/nicole.csv

#### Westpac - Our Joint Account

Opening balance: see [secrets.md](secrets.md)

data/bank-transactions/joint.csv

## Budgets

- Budget amount goes against last month's income
- Each account belongs to a group
- Accounts can be dragged and dropped between groups
- Auto monthly is optional. Amount can be determined by
    - Yearly
    - Quarterly
    - Monthly
    - Fornight
    - Weekly
