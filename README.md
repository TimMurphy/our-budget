# Our Budget

A local budgeting website: import bank statements, assign each transaction to one or more budget accounts, and fund monthly budgets from last month's income. See [specs/README.md](specs/README.md).

- **SPA**: TypeScript, Vite, no framework (`src/client`)
- **Event store**: hand-rolled, append-only, on SQLite via `node:sqlite` (`src/core/event-store.ts`)
- **Mediator**: hand-rolled command/query dispatch (`src/core/mediator.ts`)
- **Tests**: Gherkin feature files only, run in a real browser (`features/`)
- **Local data**: `data/` is untracked — `accounts.json`, `databases/our-budget.db`, and downloaded statements in `bank-transactions/`

## Run

Requires Node 22.13+.

```sh
npm install
npx playwright install chromium
npm run dev     # API on :4001, SPA on http://localhost:4000
npm test
```

## Setting up your data

Create `data/accounts.json`, then drop each bank's downloaded CSV at the path named in `statement`:

```json
{
  "accounts": [
    {
      "name": "NAB - Everyday",
      "bank": "nab",
      "openingDate": "2026-10-30",
      "openingBalance": 100.0,
      "statement": "bank-transactions/everyday.csv"
    }
  ]
}
```

`bank` is `nab` or `wbc` (Westpac). `openingBalance` is the account balance at close of business on `openingDate`, with the bank's own sign (a loan is negative). Only transactions after `openingDate` are imported. The server imports on start and hourly; **Import now** on the Bank Accounts page imports on demand.
