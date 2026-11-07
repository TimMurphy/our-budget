# AI.md

Guidance for any AI coding assistant working in this repository.

## What this is

A local budgeting website. TypeScript single-page app, a small Node API, a hand-rolled event store on SQLite, and a hand-rolled mediator. The product spec is `specs/README.md`.

## Architecture

- `src/client/` — SPA (Vite, TypeScript, no framework). Talks to the API only. Pages in `pages/`.
- `src/server/` — Node HTTP API. `POST /api/<RequestType>` forwards the JSON body to the mediator. `main.ts` registers accounts and imports statements on start, then hourly.
- `src/core/mediator.ts` — hand-rolled mediator. One handler per command/query type.
- `src/core/event-store.ts` — hand-rolled, append-only event store on SQLite (`node:sqlite`).
- `src/core/model.ts` — read model rebuilt from events; commands validate against it, queries read from it.
- `src/core/bank-accounts.ts`, `budgets.ts` — commands, queries and event writing.
- `src/core/banks.ts` — known banks (NAB, Westpac). Each parses its own downloaded CSV format.
- `src/core/app.ts` — wiring.
- `data/` — untracked. `accounts.json` (accounts and opening balances), `databases/our-budget.db`, `bank-transactions/*.csv` (downloaded statements).

## Rules

- **Roll our own.** No event-sourcing library, mediator library or ORM. Extend `src/core/`.
- **SQLite only**, via `node:sqlite`.
- **State changes are events.** Commands append events; read models are projections. Never update or delete stored events.
- **Money is integer cents.** Dates are ISO `YYYY-MM-DD`.
- **Tests are feature files only.** Gherkin in `features/` with step definitions next to each feature (`<feature>.steps.ts`); shared steps live in `features/*.steps.ts`. No unit tests, no unit-test framework.
- **Feature tests drive a real browser** (Playwright) against the real SPA, API and a temp SQLite file. See `features/support/world.ts`.
- **Never commit `data/`, and never put its contents anywhere else.** It holds personal financial data. Use made-up data in features, code, logs, commit messages and PRs. Do not copy real descriptions, amounts or account numbers from the CSVs.
- **Ports.** SPA 4000, API 4001.
- **Keep it simple.** No framework or dependency unless the task needs it. Ask before adding one.

## Commands

```sh
npm install
npx playwright install chromium   # once
npm run dev        # API on :4001, SPA on :4000
npm test           # run feature files
npm run typecheck
npm run build
```

Requires Node 22.13 or later.

## Before finishing a change

1. `npm run typecheck` passes.
2. `npm test` passes.
3. New behaviour has a feature file.
4. Nothing from `data/` is staged.
5. Do not commit or push unless asked.
6. Do not commit to the main branch.
