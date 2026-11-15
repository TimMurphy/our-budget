# AI.md

Guidance for any AI coding assistant working in this repository.

## Rules

- **Roll our own.** No event-sourcing library, mediator library or ORM.
- **SQLite only.** Database is `data/database/our-budget.db`.
- **State changes are events.** Commands append events; read models are projections. Never update or delete stored events.
- **Money is integer cents.** 
- **Dates are ISO `YYYY-MM-DD`.**
- **Tests are feature files only.** Gherkin in `features/` with step definitions next to each feature (`<feature>.steps.ts`); shared steps live in `features/*.steps.ts`. No unit tests, no unit-test framework.
- **Never commit `data/`, and never put its contents anywhere else.** It holds personal financial data. Use made-up data in features, code, logs, commit messages and PRs. Do not copy real descriptions, amounts or account numbers from the CSVs.
- **Keep it simple.** No framework or dependency unless the task needs it. Ask before adding one.
