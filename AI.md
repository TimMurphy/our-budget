# AI.md

Guidance for any AI coding assistant working in this repository.

## Specification

- **[specs/SPEC.md](specs/SPEC.md) is the single source of all specifications for the application.** Read it before changing anything. Do not put specifications anywhere else.
- **Every time you act on a prompt that changes what the application does, update `specs/SPEC.md` in the same change.** That includes new or changed behaviour, rules, decisions and answers to open questions. If the spec and a prompt disagree, follow the prompt and update the spec.
- Keep `specs/SPEC.md` free of real financial data. Opening balances live in `specs/secrets.md`, which is not committed.

## Rules

- **Rust.** The application is written in Rust.
- **Roll our own.** No event-sourcing library, mediator library or ORM.
- **SQLite only.** Database is `data/databases/our-budget.db`.
- **State changes are events.** Commands append events; read models are projections. Never update or delete stored events.
- **Money is integer cents.** 
- **Dates are ISO `YYYY-MM-DD`.**
- **Tests are feature files only.** Gherkin feature files and their Rust step definitions live in `specs/`, with each step file next to its feature. No unit tests, no unit-test framework.
- **Never commit `data/`, and never put its contents anywhere else.** It holds personal financial data. Use made-up data in features, code, logs, commit messages and PRs. Do not copy real descriptions, amounts or account numbers from the CSVs.
- **Keep it simple.** No framework or dependency unless the task needs it. Ask before adding one.
