# Codex Repository Rules

1. Read `docs/PROJECT_STATE.md` before making any change.
2. Continue the declared next task unless a real blocker requires a different step.
3. Keep the repository clean-room. Do not mirror an external project layout blindly.
4. Keep strategy and trading logic inside `crates/domain` and `crates/app`, not inside HTTP handlers.
5. After finishing work, update `docs/PROJECT_STATE.md` with the exact current state and next task.
6. After finishing work, update `docs/BACKLOG.md` to reflect completed and remaining items.
7. Record any intentional tradeoff, scope cut, or behavioral deviation in `docs/DECISIONS.md`.
8. Prefer small focused modules and production-minded defaults over large placeholder files.
9. Use tracing-based logs instead of ad hoc print debugging.
10. Keep MAINNET live trading disabled unless a future task explicitly changes that boundary. Current live execution is constrained to explicit TESTNET-only flows; do not add hidden mainnet bypasses.
