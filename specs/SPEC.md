# Our Budget

Our Budget is a cross-platform desktop application (Windows, Linux and macOS (not tested)) that tracks our spending and savings. It keeps its data in a SQLite database, `data/databases/our-budget.db`.

It is written in Rust. Specs and tests are Gherkin feature files, with their Rust step definitions, in `specs/` (see [AI.md](../AI.md)).

This file contains all the specifications for the application. It is updated whenever a prompt changes what the application does.

Opening balances are as at close of business 30/10/2026 and are kept in [secrets.md](../data/secrets.md), which is not committed.

## Running

- `cargo run` starts the application. It opens a native window titled "Our Budget", maximized to fill the screen (the title bar and taskbar stay visible, and the window can be restored to a smaller size), that shows the pages using the operating system's own web view. There is no web server, no port and no browser: the page talks to the application inside the same process, so nothing is reachable over the network.
- On Windows, if `cargo` cannot find the C++ build tools, run it as `./cargo.ps1 run` (or `./cargo.ps1 test --test specs`). The script finds a Visual Studio C++ toolset that has the headers and libraries, sets up the environment and then runs cargo.
- `cargo test --test specs` runs every feature file in `specs/`. The steps drive the application through the mediator, against an in-memory event store and a temporary statements folder.
- Opening balances are read from `data/secrets.md`, one line per bank account in the form `- <account name>: <amount>`, for example `- NAB - Home Loan: -123456.78`. A missing line means an opening balance of $0.00.
- On start, if the database has no events at all, each bank account is registered and its opening balance is recorded as an event. The secrets file is not read again once the database has events, so changing it later has no effect. To start over, delete `data/databases/our-budget.db`.

## Building

- `cargo build --release` produces one native executable (`target/release/our-budget.exe` on Windows) for the operating system it is built on. The pages are embedded in it, so there is nothing else to install except the web view the operating system provides. In a release build on Windows no console window opens.
- Windows, Linux and macOS each have a native build. Each is built on that operating system, because the bundled SQLite is C code that needs that system's C compiler. Cross-compiling to macOS is not supported.
- Targets:
    - Windows: `x86_64-pc-windows-msvc`
    - Linux: `x86_64-unknown-linux-gnu`
    - macOS: `aarch64-apple-darwin`, and `x86_64-apple-darwin` for Intel Macs
- The web view that draws the window:
    - Windows: Microsoft Edge WebView2, which is included with Windows 11 and current Windows 10.
    - macOS: WebKit, which is part of the system.
    - Linux: WebKitGTK, installed with the system package manager (`libwebkit2gtk-4.1` and its development files to build).
- The `data` folder is the one in the folder the program is started from. If there is none there, the one beside the executable is used. If neither exists, one is created in the current folder. The web view keeps its own settings, such as the remembered date order, in `data/webview`.
- If the program cannot start, it writes the reason to `data/error.txt`.
- macOS is not tested.

## Concepts

- **Bank account:** A real account at a bank. Its transactions are imported from downloaded CSV statements.
- **Transaction:** A single debit or credit on a bank account. Immutable once imported.
- **Budget account:** A named envelope that money is assigned to (for example "Groceries" or "Rates"). Not a bank account.
- **Group:** A named collection of budget accounts, used to organise the budget screen.
- **Assignment:** A part of a transaction (or all of it) allocated to one budget account.

Rules that apply everywhere:

- Money is stored as integer cents and shown in AUD as `$1,234.56`. Debits are negative.
- Dates are ISO `YYYY-MM-DD` in storage and `DD/MM/YYYY` on screen.
- Every state change is an event appended to the event store. Screens read from projections built from the events. Events are never updated or deleted.
- The read models are tables in the database: `bank_accounts`, `transactions`, `assignments`, `import_failures`, `groups`, `budget_accounts` and `budgets`. Every query reads from them. They are updated by applying each event, in the same transaction that stores the event, so they are always up to date.
- The tables are not rebuilt on every start. A `read_model_state` row records the last event applied. On start only the events stored after it are applied.
- The tables are rebuilt from all the events only when there is no `read_model_state` row (a new database), it is from a different version of the program (`PROJECTION_VERSION` in `src/projection.rs` is bumped when the tables or what is written to them changes), or the stored events no longer match it, for example because events were deleted by hand.
- The application is for one household. There are no logins.
- The page follows the computer's light or dark theme. In the dark theme, links are amber, not blue, so they stay readable.

## Bank Accounts

- Transactions are imported periodically.
- Each transaction must be assignable to one or more budget accounts.
- Filters
    - All transactions
    - Unassigned
    - Assigned

### Bank account list

- Lists every bank account with its name, bank and current balance.
- Current balance is the opening balance plus the sum of all imported transactions.
- Also shows the total of all bank accounts.
- Selecting a bank account opens its transaction list.
- "Bank accounts" in the navigation bar opens a drop-down listing every bank account. Choosing one opens its transaction list. Clicking "Bank accounts" itself opens the bank account list.

### Importing transactions

- The application imports each account's statement file on start and then every hour while it is running. A manual "Import now" action is also available.
- Each bank has its own CSV format (NAB and Westpac). The importer for each bank parses its own format.
- A transaction is identified by its bank account, date, amount, description and its position among identical rows in the file. Importing the same file twice adds nothing.
- Statement files overlap and grow over time, so a re-import adds only the rows not already imported.
- A row that cannot be parsed is skipped. The import records the file name and row number and carries on with the remaining rows. The failures are shown on the bank account screen.
- A missing statement file is not an error. The account simply has nothing new.
- Transactions are never edited or deleted by an import.
- Only transactions dated on or after 1 Nov 2026 are imported. Earlier rows in a statement are ignored. Opening balances are as at close of business 30/10/2026, so earlier activity is already inside them.
- NAB statements have a header row. Columns include: date (`DD Mon YY`), amount (negative for debits), account number, blank, type, transaction details. Without a header row, that column order is assumed.
- Statements are read from `data/back-accounts/`, one file per bank account, named in the Accounts section below.
- Westpac statements have a header row. Columns include: bank account, date (`DD/MM/YYYY`), narrative, debit amount, credit amount. A debit is stored as a negative amount.

### Transaction list

- Shows date, description, amount and the budget accounts it is assigned to, newest first.
- Clicking the "Assigned to" cell of a transaction opens a drop-down list of budget accounts, grouped by group, with Income, Unassigned and "New budget account…". Choosing one assigns the whole transaction to it. "New budget account…" opens a dialog with a Group drop-down (the existing groups in alphabetical order, Ungrouped selected by default, plus "Add group…" which reveals a new group name field) and a budget account name field. Create makes the group if needed, then the budget account, and assigns the transaction to it. An error such as a duplicate name is shown in the dialog. Unassigned removes the assignment. A split transaction opens the split editor instead, and a transfer cannot be changed here.
- Shows the running balance for the bank account.
- The page has no back link. The bank account heading sits close to the top of the page, and the navigation bar's "Bank accounts" drop-down is the way to move between accounts.
- The list of transactions scrolls inside its own area. The account heading, the unassigned count, the filters and the column headings stay in view while the rows scroll.
- Clicking the Date column heading reverses the order, between newest first (the default) and oldest first. An arrow shows the current order. The choice is remembered in the browser and used for every bank account the next time the page is opened. The running balance stays with its transaction.
- Filters: All transactions, Unassigned, Assigned. A bank account opens on Unassigned. The selected filter is shown in the page address so it survives a refresh.
- Shows how many transactions are unassigned and their total.
- Searching by description narrows the list within the current filter.

### Assigning transactions

- A transaction can be assigned to one budget account, or split across several.
- The assigned amounts must add up exactly to the transaction amount. A split cannot be saved otherwise.
- A transaction with its full amount assigned is Assigned. A transaction with nothing or only part assigned is Unassigned.
- An assignment can be changed or removed at any time. Both changes are new events. The earlier assignment stays in the history.
- A new budget account can be created while assigning, by typing a name that does not exist yet. It joins the group "Ungrouped".
- An assignment counts in the month of its transaction's date. It cannot be moved to a different month.
- The same assignment can be applied to several selected transactions at once. Ticking transactions and choosing "Assign selected" opens a dialog with a drop-down of budget accounts (Income, then the accounts grouped by group) and "+ New budget account…", which opens the same new-budget-account dialog as the "Assigned to" cell. The transactions stay ticked until the assignment is made, and nothing is changed if the dialog is cancelled. "Assign selected" and "Unassign selected" with nothing ticked show a message and change nothing. A count of ticked transactions is shown beside the buttons.

### Transfers between bank accounts

- A debit in one bank account and a matching credit in another, of the same amount and no more than 5 days apart, is a transfer.
- A transaction that is assigned to budget accounts must be unassigned before it can be linked. A linked transaction cannot be assigned.
- A transaction can be linked to its matching transaction in another bank account.
- Linked transactions count as Assigned and do not affect the budget, since the money did not leave the household.
- A link can be removed. Both transactions then return to Unassigned.
- A transaction can be in only one link.

### Accounts

#### NAB - Home Loan

Opening balance: see [secrets.md](../data/secrets.md)

data/back-accounts/NAB - Home Loan.csv

#### NAB - Nicole's Account

Opening balance: see [secrets.md](../data/secrets.md)

data/back-accounts/NAB - Nicole's Account.csv

#### Westpac - Our Account

Opening balance: see [secrets.md](../data/secrets.md)

data/back-accounts/Westpac - Our Account.csv

## Budgets

- Budget amount goes against last month's income
- Each account belongs to a group
- Accounts can be dragged and dropped between groups
- Auto monthly is optional. Amount can be determined by
    - Yearly
    - Quarterly
    - Monthly
    - Fortnight
    - Weekly

### Budget screen

- Shows one month at a time, with previous and next month controls. It opens on the current month.
- Groups are shown as sections. Each section lists its budget accounts and shows the group total.
- For each budget account the screen shows:
    - Budgeted: money set aside this month.
    - Spent: the sum of assignments this month.
    - Available: what has carried over from earlier months, plus Budgeted, plus Spent.
- At the top it shows Income last month, the total Budgeted this month and what is left to budget.
- Available carries over from month to month, but only when it is positive.
- An overspent account shows a negative Available in red for that month. At month end the overspend is not carried forward. It is taken from next month's income, which reduces what is left to budget next month, and the account starts the next month at zero.

### Income

- "Income" is a fixed budget account. It always exists and is in no group. It cannot be renamed, closed or deleted, and no other budget account can be marked as income.
- Income is the sum of credits assigned to the budget account "Income".
- The budget for a month is funded by the income received in the previous month. For example, income received in October is budgeted in November.
- Overspending from last month is taken from this income before it is available to budget.
- Budgeting more than last month's income, after overspending, is allowed, but the screen flags "Over budgeted" with the amount.

### Budget accounts

- A budget account has a name and belongs to exactly one group.
- Names are unique, ignoring case.
- A budget account can be renamed. Its history is kept.
- A budget account can be closed once its Available is zero. A closed account no longer appears on the budget screen or in the assign list, and its past assignments remain.
- New budget accounts start in the group "Ungrouped".
- Closing is checked against the Available amount for the current month.

### Groups

- "Ungrouped" is a built-in group. It cannot be renamed or removed.

- A group has a name. Names are unique, ignoring case.
- Groups can be added, renamed and reordered.
- A group can be removed only when it has no budget accounts.
- Budget accounts are dragged and dropped between groups. The new group applies to every month, not only the one on screen.

### Auto monthly

- Auto monthly is optional for each budget account.
- When on, the account's Budgeted amount for each new month is filled in automatically.
- The amount is entered per Yearly, Quarterly, Monthly, Fortnight or Weekly period and converted to a monthly amount:

    - Yearly: amount ÷ 12
    - Quarterly: amount ÷ 3
    - Monthly: amount
    - Fortnight: amount × 26 ÷ 12
    - Weekly: amount × 52 ÷ 12

- Results are rounded to the nearest cent. Any remaining cent is not carried forward.
- Auto monthly fills a month only when no Budgeted amount has been set for that month. A value entered by hand is never overwritten.
- Changing the auto monthly setting affects the months that are not yet filled in.

## Not covered

- Multiple users, logins or sharing.
- Currencies other than AUD.
- Automatic bank connections. Statements are downloaded by hand.
- Editing or deleting imported transactions.
