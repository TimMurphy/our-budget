/** Read model: built by replaying events. Queries read from it; commands validate against it. */
import type { StoredEvent } from "./event-store.ts";

export type Cadence = "yearly" | "quarterly" | "monthly" | "fortnightly" | "weekly";
export type BudgetKind = "income" | "expense";

export interface Allocation { budgetAccountId: string; amountCents: number }

export interface BankAccount {
  id: string;
  slug: string;
  name: string;
  bankCode: string;
  openingDate: string;
  openingBalanceCents: number;
  statement: string;
}

export interface Transaction {
  id: string;
  accountId: string;
  date: string;
  /** Global order of import, to keep same-day transactions in order. */
  position: number;
  description: string;
  amountCents: number;
  balanceCents: number | null;
  bankCategory: string | null;
  allocations: Allocation[];
  /** Id of the matching transaction in another bank account, when this is a transfer between them. */
  transferWith: string | null;
}

export interface BudgetGroup { id: string; name: string }

export interface BudgetAccount {
  id: string;
  groupId: string;
  name: string;
  kind: BudgetKind;
  auto: { cadence: Cadence; amountCents: number } | null;
  /** Explicit amounts by month (YYYY-MM); these win over the auto amount. */
  budgets: Map<string, number>;
}

export const slugify = (name: string): string =>
  name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "");

const monthsPerCadence: Record<Cadence, number> = {
  yearly: 12, quarterly: 3, monthly: 1, fortnightly: 12 / 26, weekly: 12 / 52,
};

/** Monthly equivalent of an amount that recurs on a cadence. */
export const monthlyCents = (cadence: Cadence, amountCents: number): number =>
  Math.round(amountCents / monthsPerCadence[cadence]);

export class Model {
  readonly accounts: BankAccount[] = [];
  readonly transactions = new Map<string, Transaction>();
  readonly groups: BudgetGroup[] = [];
  readonly budgetAccounts: BudgetAccount[] = [];

  apply(e: StoredEvent): void {
    const d = e.data as any;
    switch (e.type) {
      case "BankAccountRegistered":
        this.accounts.push({
          id: d.accountId, slug: d.slug, name: d.name, bankCode: d.bankCode,
          openingDate: d.openingDate, openingBalanceCents: d.openingBalanceCents, statement: d.statement,
        });
        break;
      case "TransactionImported":
        this.transactions.set(d.transactionId, {
          id: d.transactionId, accountId: d.accountId, date: d.date, position: e.position,
          description: d.description, amountCents: d.amountCents, balanceCents: d.balanceCents,
          bankCategory: d.bankCategory, allocations: [], transferWith: null,
        });
        break;
      case "BudgetGroupCreated":
        this.groups.push({ id: d.groupId, name: d.name });
        break;
      case "BudgetAccountCreated":
        this.budgetAccounts.push({
          id: d.budgetAccountId, groupId: d.groupId, name: d.name, kind: d.kind,
          auto: d.auto ?? null, budgets: new Map(),
        });
        break;
      case "BudgetAccountMoved": {
        const a = this.budgetAccounts.find((b) => b.id === d.budgetAccountId);
        if (a) a.groupId = d.groupId;
        break;
      }
      case "BudgetAmountSet": {
        const a = this.budgetAccounts.find((b) => b.id === d.budgetAccountId);
        if (a) a.budgets.set(d.month, d.amountCents);
        break;
      }
      case "TransferLinked": {
        const a = this.transactions.get(d.transactionId);
        const b = this.transactions.get(d.counterpartTransactionId);
        if (a && b) { a.transferWith = b.id; b.transferWith = a.id; }
        break;
      }
      case "TransferUnlinked": {
        const a = this.transactions.get(d.transactionId);
        const b = this.transactions.get(d.counterpartTransactionId);
        if (a) a.transferWith = null;
        if (b) b.transferWith = null;
        break;
      }
      case "TransactionAssigned": {
        const t = this.transactions.get(d.transactionId);
        if (t) t.allocations = d.allocations;
        break;
      }
    }
  }

  /** A transaction is "assigned" once it has budget allocations or is linked as a transfer. */
  isAssigned(t: Transaction): boolean {
    return t.allocations.length > 0 || t.transferWith !== null;
  }

  accountBySlug(slug: string): BankAccount | undefined {
    return this.accounts.find((a) => a.slug === slug);
  }

  transactionsOf(accountId: string): Transaction[] {
    return [...this.transactions.values()].filter((t) => t.accountId === accountId);
  }

  balanceCents(account: BankAccount): number {
    return this.transactionsOf(account.id).reduce((sum, t) => sum + t.amountCents, account.openingBalanceCents);
  }

  /** Budgeted amount for a month: explicit amount, else the auto monthly amount, else 0. */
  budgetedCents(a: BudgetAccount, month: string): number {
    return a.budgets.get(month) ?? (a.auto ? monthlyCents(a.auto.cadence, a.auto.amountCents) : 0);
  }
}
