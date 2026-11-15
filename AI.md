# AI.md

Guidance for any AI coding assistant working in this repository.

## Specification

- **[specs/SPEC.md](specs/SPEC.md) is the single source of all specifications for the application.** Read it before changing anything. Do not put specifications anywhere else.
- **Every time you act on a prompt that changes what the application does, update `specs/SPEC.md` in the same change.** That includes new or changed behaviour, rules, decisions and answers to open questions. If the spec and a prompt disagree, follow the prompt and update the spec.
- Keep `specs/SPEC.md` free of real financial data. Opening balances live in `data/secrets.md`, which is not committed.

## Rules

- **Rust.** The application is written in Rust.
- **Roll our own.** No event-sourcing library, mediator library or ORM.
- **SQLite only.** Database is `data/databases/our-budget.db`.
- **State changes are events.** Commands append events; read models are projections. Never update or delete stored events.
- **Money is integer cents.** 
- **Dates are ISO `YYYY-MM-DD`.**
- **Tests are feature files only.** Gherkin feature files and their Rust step definitions live in `specs/`, with each step file next to its feature. No unit tests, no unit-test framework.
- **Never commit `data/`, and never put its contents anywhere else.** It holds personal financial data. Use made-up data in features, code, logs, commit messages and PRs. Do not copy real descriptions, amounts or account numbers from the CSVs.
- **Layout.** `src/` is the application: `model.rs` (events and read model), `event_store.rs`, `mediator.rs`, `handlers.rs` (commands and queries), `banks.rs` (statement parsers), `index.html` (the whole web UI). Run with `cargo run`, test with `cargo test --test specs`.
- **Windows builds.** Rust needs the Visual Studio C++ Build Tools. If `cargo` fails with `LNK1104: msvcrt.lib`, or with `C1083: stdarg.h`, run cargo through `./cargo.ps1` (for example `./cargo.ps1 run`). It finds an MSVC toolset that has the C++ headers and libraries and sets `LIB`, `INCLUDE` and `PATH`.
- **Keep it simple.** No framework or dependency unless the task needs it. Ask before adding one.
