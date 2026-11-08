/** Auto assign rules: assign unassigned transactions by description, pending approval. */
import type { EventStore } from "./event-store.ts";
import type { Model } from "./model.ts";

/**
 * Assign every unassigned transaction that a rule matches, for its full amount.
 * These are "pending approval" until the user approves them. Returns how many were assigned.
 */
export function autoAssign(store: EventStore, model: Model): number {
  let count = 0;
  for (const t of [...model.transactions.values()]) {
    if (model.isAssigned(t)) continue;
    const rule = model.ruleFor(t.description);
    if (!rule) continue;
    store.append(`transactions-${t.accountId}`, "any", [
      {
        type: "TransactionAutoAssigned",
        data: {
          transactionId: t.id,
          ruleId: rule.id,
          allocations: [{ budgetAccountId: rule.budgetAccountId, amountCents: t.amountCents }],
        },
      },
    ]);
    count++;
  }
  return count;
}
