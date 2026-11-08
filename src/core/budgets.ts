/** Budget groups, budget accounts, monthly amounts, and assigning transactions to them. */
import { autoAssign } from "./auto-assign.ts";
import type { EventStore } from "./event-store.ts";
import type { Mediator, Request } from "./mediator.ts";
import { normalizeText, type BudgetKind, type Cadence, type Model } from "./model.ts";

const cadences: Cadence[] = ["yearly", "quarterly", "monthly", "fortnightly", "weekly"];
const isMonth = (m: string) => /^\d{4}-(0[1-9]|1[0-2])$/.test(m);

export interface CreateBudgetGroup extends Request<{ id: string }> { type: "CreateBudgetGroup"; name: string }

export interface CreateBudgetAccount extends Request<{ id: string }> {
  type: "CreateBudgetAccount";
  groupId: string;
  name: string;
  kind?: BudgetKind;
  /** Optional auto monthly: an amount that recurs on a cadence. */
  auto?: { cadence: Cadence; amountCents: number };
}

export interface MoveBudgetAccount extends Request<{ ok: true }> {
  type: "MoveBudgetAccount";
  budgetAccountId: string;
  groupId: string;
}

export interface SetBudgetAmount extends Request<{ ok: true }> {
  type: "SetBudgetAmount";
  budgetAccountId: string;
  month: string; // YYYY-MM
  amountCents: number;
}

export interface AssignTransaction extends Request<{ ok: true }> {
  type: "AssignTransaction";
  transactionId: string;
  /** Must add up to the transaction's amount. Empty clears the assignment. */
  allocations: { budgetAccountId: string; amountCents: number }[];
}

export interface LinkTransfer extends Request<{ ok: true }> {
  type: "LinkTransfer";
  transactionId: string;
  /** The other side of the transfer, in a different bank account. */
  counterpartTransactionId: string;
}

export interface UnlinkTransfer extends Request<{ ok: true }> {
  type: "UnlinkTransfer";
  transactionId: string;
}

export interface TransferCandidate {
  transactionId: string;
  accountName: string;
  date: string;
  description: string;
  amountCents: number;
}
/** Unassigned transactions in other bank accounts that could be the other side of this transfer. */
export interface GetTransferCandidates extends Request<TransferCandidate[]> {
  type: "GetTransferCandidates";
  transactionId: string;
}

export interface CreateAutoAssignRule extends Request<{ id: string; autoAssigned: number }> {
  type: "CreateAutoAssignRule";
  /** Text to look for in a transaction's description. */
  pattern: string;
  budgetAccountId: string;
}

export interface DeleteAutoAssignRule extends Request<{ ok: true }> {
  type: "DeleteAutoAssignRule";
  ruleId: string;
}

export interface ApproveTransactions extends Request<{ approved: number }> {
  type: "ApproveTransactions";
  transactionIds: string[];
}

export interface AutoAssignRuleView {
  id: string;
  pattern: string;
  budgetAccountId: string;
  budgetAccountName: string;
  groupName: string;
}
export interface GetAutoAssignRules extends Request<AutoAssignRuleView[]> { type: "GetAutoAssignRules" }

export interface BudgetAccountView {
  id: string;
  name: string;
  kind: BudgetKind;
  auto: { cadence: Cadence; amountCents: number } | null;
  budgetedCents: number;
  activityCents: number;
}
export interface BudgetGroupView { id: string; name: string; accounts: BudgetAccountView[] }
export interface BudgetView {
  month: string;
  /** Income assigned to income accounts in the previous month: what this month's budget is funded from. */
  incomeLastMonthCents: number;
  budgetedCents: number;
  toBudgetCents: number;
  groups: BudgetGroupView[];
}
export interface GetBudget extends Request<BudgetView> { type: "GetBudget"; month: string }

const previousMonth = (month: string) => {
  const [y, m] = month.split("-").map(Number) as [number, number];
  return m === 1 ? `${y - 1}-12` : `${y}-${String(m - 1).padStart(2, "0")}`;
};

export function registerBudgets(mediator: Mediator, store: EventStore, model: Model): void {
  const needGroup = (id: string) => {
    if (!model.groups.some((g) => g.id === id)) throw new Error("Unknown budget group");
  };
  const needBudgetAccount = (id: string) => {
    const a = model.budgetAccounts.find((b) => b.id === id);
    if (!a) throw new Error("Unknown budget account");
    return a;
  };

  mediator.register<CreateBudgetGroup>("CreateBudgetGroup", (cmd) => {
    const name = cmd.name?.trim();
    if (!name) throw new Error("Group name is required");
    if (model.groups.some((g) => g.name.toLowerCase() === name.toLowerCase())) {
      throw new Error("Group name must be unique");
    }
    const id = crypto.randomUUID();
    store.append("budget", "any", [{ type: "BudgetGroupCreated", data: { groupId: id, name } }]);
    return { id };
  });

  mediator.register<CreateBudgetAccount>("CreateBudgetAccount", (cmd) => {
    const name = cmd.name?.trim();
    if (!name) throw new Error("Budget account name is required");
    needGroup(cmd.groupId);
    if (model.budgetAccounts.some((a) => a.name.toLowerCase() === name.toLowerCase())) {
      throw new Error("Budget account name must be unique");
    }
    if (cmd.auto && (!cadences.includes(cmd.auto.cadence) || !Number.isInteger(cmd.auto.amountCents))) {
      throw new Error("Auto monthly needs a cadence and an amount");
    }
    const id = crypto.randomUUID();
    store.append("budget", "any", [
      {
        type: "BudgetAccountCreated",
        data: { budgetAccountId: id, groupId: cmd.groupId, name, kind: cmd.kind ?? "expense", auto: cmd.auto ?? null },
      },
    ]);
    return { id };
  });

  mediator.register<MoveBudgetAccount>("MoveBudgetAccount", (cmd) => {
    needBudgetAccount(cmd.budgetAccountId);
    needGroup(cmd.groupId);
    store.append("budget", "any", [
      { type: "BudgetAccountMoved", data: { budgetAccountId: cmd.budgetAccountId, groupId: cmd.groupId } },
    ]);
    return { ok: true };
  });

  mediator.register<SetBudgetAmount>("SetBudgetAmount", (cmd) => {
    needBudgetAccount(cmd.budgetAccountId);
    if (!isMonth(cmd.month)) throw new Error("Month must be YYYY-MM");
    if (!Number.isInteger(cmd.amountCents) || cmd.amountCents < 0) throw new Error("Amount must be zero or more");
    store.append("budget", "any", [
      { type: "BudgetAmountSet", data: { budgetAccountId: cmd.budgetAccountId, month: cmd.month, amountCents: cmd.amountCents } },
    ]);
    return { ok: true };
  });

  mediator.register<AssignTransaction>("AssignTransaction", (cmd) => {
    const tx = model.transactions.get(cmd.transactionId);
    if (!tx) throw new Error("Unknown transaction");
    if (tx.transferWith) throw new Error("This transaction is a transfer; unlink the transfer first");
    const allocations = cmd.allocations ?? [];
    for (const a of allocations) {
      needBudgetAccount(a.budgetAccountId);
      if (!Number.isInteger(a.amountCents) || a.amountCents === 0) throw new Error("Each amount must be non-zero");
    }
    if (new Set(allocations.map((a) => a.budgetAccountId)).size !== allocations.length) {
      throw new Error("Each budget account can appear once");
    }
    const total = allocations.reduce((s, a) => s + a.amountCents, 0);
    if (allocations.length && total !== tx.amountCents) {
      throw new Error(`Amounts must add up to the transaction amount (${tx.amountCents / 100}), got ${total / 100}`);
    }
    store.append(`transactions-${tx.accountId}`, "any", [
      { type: "TransactionAssigned", data: { transactionId: tx.id, allocations } },
    ]);
    return { ok: true };
  });

  const days = (a: string, b: string) => Math.abs(Date.parse(a) - Date.parse(b)) / 86_400_000;

  mediator.register<LinkTransfer>("LinkTransfer", (cmd) => {
    const a = model.transactions.get(cmd.transactionId);
    const b = model.transactions.get(cmd.counterpartTransactionId);
    if (!a || !b) throw new Error("Unknown transaction");
    if (a.accountId === b.accountId) throw new Error("A transfer must be between two different bank accounts");
    if (a.amountCents + b.amountCents !== 0) throw new Error("A transfer's two sides must have the same amount, one in and one out");
    for (const t of [a, b]) {
      if (model.isAssigned(t)) throw new Error("Both transactions must be unassigned");
    }
    store.append(`transactions-${a.accountId}`, "any", [
      { type: "TransferLinked", data: { transactionId: a.id, counterpartTransactionId: b.id } },
    ]);
    return { ok: true };
  });

  mediator.register<UnlinkTransfer>("UnlinkTransfer", (cmd) => {
    const t = model.transactions.get(cmd.transactionId);
    if (!t) throw new Error("Unknown transaction");
    if (!t.transferWith) throw new Error("This transaction is not a transfer");
    store.append(`transactions-${t.accountId}`, "any", [
      { type: "TransferUnlinked", data: { transactionId: t.id, counterpartTransactionId: t.transferWith } },
    ]);
    return { ok: true };
  });

  mediator.register<GetTransferCandidates>("GetTransferCandidates", (q) => {
    const tx = model.transactions.get(q.transactionId);
    if (!tx) throw new Error("Unknown transaction");
    return [...model.transactions.values()]
      .filter((o) => o.accountId !== tx.accountId && o.amountCents === -tx.amountCents && !model.isAssigned(o) && days(o.date, tx.date) <= 7)
      .sort((x, y) => days(x.date, tx.date) - days(y.date, tx.date))
      .map((o) => ({
        transactionId: o.id,
        accountName: model.accounts.find((a) => a.id === o.accountId)?.name ?? "?",
        date: o.date,
        description: o.description,
        amountCents: o.amountCents,
      }));
  });

  mediator.register<CreateAutoAssignRule>("CreateAutoAssignRule", (cmd) => {
    const pattern = cmd.pattern?.trim().replace(/\s+/g, " ");
    if (!pattern) throw new Error("Text to match is required");
    needBudgetAccount(cmd.budgetAccountId);
    if (model.rules.some((r) => normalizeText(r.pattern) === normalizeText(pattern))) {
      throw new Error("An auto assign rule for that text already exists");
    }
    const id = crypto.randomUUID();
    store.append("budget", "any", [
      { type: "AutoAssignRuleCreated", data: { ruleId: id, pattern, budgetAccountId: cmd.budgetAccountId } },
    ]);
    return { id, autoAssigned: autoAssign(store, model) };
  });

  mediator.register<DeleteAutoAssignRule>("DeleteAutoAssignRule", (cmd) => {
    if (!model.rules.some((r) => r.id === cmd.ruleId)) throw new Error("Unknown auto assign rule");
    store.append("budget", "any", [{ type: "AutoAssignRuleDeleted", data: { ruleId: cmd.ruleId } }]);
    return { ok: true };
  });

  mediator.register<ApproveTransactions>("ApproveTransactions", (cmd) => {
    let approved = 0;
    for (const id of cmd.transactionIds ?? []) {
      const t = model.transactions.get(id);
      if (!t) throw new Error("Unknown transaction");
      if (!t.pendingApproval) continue;
      store.append(`transactions-${t.accountId}`, "any", [{ type: "TransactionApproved", data: { transactionId: id } }]);
      approved++;
    }
    return { approved };
  });

  mediator.register<GetAutoAssignRules>("GetAutoAssignRules", () =>
    model.rules.map((r) => {
      const account = model.budgetAccounts.find((a) => a.id === r.budgetAccountId);
      return {
        id: r.id,
        pattern: r.pattern,
        budgetAccountId: r.budgetAccountId,
        budgetAccountName: account?.name ?? "?",
        groupName: model.groups.find((g) => g.id === account?.groupId)?.name ?? "?",
      };
    }),
  );

  mediator.register<GetBudget>("GetBudget", (q) => {
    if (!isMonth(q.month)) throw new Error("Month must be YYYY-MM");
    const activity = new Map<string, number>();
    const incomeIds = new Set(model.budgetAccounts.filter((a) => a.kind === "income").map((a) => a.id));
    let incomeLastMonthCents = 0;
    const last = previousMonth(q.month);
    for (const t of model.transactions.values()) {
      if (t.pendingApproval) continue; // auto assigned but not approved yet: not in the budget
      for (const al of t.allocations) {
        if (t.date.startsWith(q.month)) activity.set(al.budgetAccountId, (activity.get(al.budgetAccountId) ?? 0) + al.amountCents);
        if (t.date.startsWith(last) && incomeIds.has(al.budgetAccountId)) incomeLastMonthCents += al.amountCents;
      }
    }
    let budgetedCents = 0;
    const groups = model.groups.map((g): BudgetGroupView => ({
      id: g.id,
      name: g.name,
      accounts: model.budgetAccounts
        .filter((a) => a.groupId === g.id)
        .map((a): BudgetAccountView => {
          const budgeted = a.kind === "income" ? 0 : model.budgetedCents(a, q.month);
          budgetedCents += budgeted;
          return { id: a.id, name: a.name, kind: a.kind, auto: a.auto, budgetedCents: budgeted, activityCents: activity.get(a.id) ?? 0 };
        }),
    }));
    return { month: q.month, incomeLastMonthCents, budgetedCents, toBudgetCents: incomeLastMonthCents - budgetedCents, groups };
  });
}
