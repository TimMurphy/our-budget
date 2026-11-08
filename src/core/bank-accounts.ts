/** Bank accounts: registering them from data/accounts.json, importing statements, queries. */
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { findBank, knownBanks, type Bank } from "./banks.ts";
import type { EventStore } from "./event-store.ts";
import type { Mediator, Request } from "./mediator.ts";
import { slugify, type Model, type Transaction } from "./model.ts";
import { toCents } from "./money.ts";

export interface SyncBankAccounts extends Request<{ registered: string[] }> { type: "SyncBankAccounts" }

export interface ImportTransactions extends Request<{ imported: number; skipped: number }> {
  type: "ImportTransactions";
  /** Import just this account; omit for all. */
  slug?: string;
}

export interface GetKnownBanks extends Request<Pick<Bank, "code" | "name">[]> { type: "GetKnownBanks" }

export interface BankAccountSummary {
  id: string;
  slug: string;
  name: string;
  bankName: string;
  openingDate: string;
  openingBalanceCents: number;
  balanceCents: number;
  transactionCount: number;
  unassignedCount: number;
}
export interface GetBankAccounts extends Request<BankAccountSummary[]> { type: "GetBankAccounts" }

export interface TransactionRow {
  id: string;
  date: string;
  description: string;
  amountCents: number;
  balanceCents: number | null;
  bankCategory: string | null;
  allocations: { budgetAccountId: string; budgetAccountName: string; amountCents: number }[];
  /** Set when this is one side of a transfer between two of our bank accounts. */
  transfer: { transactionId: string; accountName: string; accountSlug: string } | null;
}
export type TransactionFilter = "all" | "unassigned" | "assigned";
export interface GetTransactions extends Request<{ account: BankAccountSummary; transactions: TransactionRow[] }> {
  type: "GetTransactions";
  slug: string;
  filter?: TransactionFilter;
}

interface AccountsFile {
  accounts: { name: string; bank: string; openingDate: string; openingBalance: number | string; statement: string }[];
}

/** Stable id for a statement line; the n-th identical line in a file gets a different id. */
const fingerprint = (accountId: string, line: { date: string; description: string; amountCents: number; balanceCents: number | null }, nth: number) =>
  createHash("sha1")
    .update([accountId, line.date, line.description, line.amountCents, line.balanceCents ?? "", nth].join("|"))
    .digest("hex")
    .slice(0, 16);

export function registerBankAccounts(mediator: Mediator, store: EventStore, model: Model, dataDir: string): void {
  const summary = (a: Model["accounts"][number]): BankAccountSummary => {
    const txs = model.transactionsOf(a.id);
    return {
      id: a.id, slug: a.slug, name: a.name, bankName: findBank(a.bankCode)?.name ?? a.bankCode,
      openingDate: a.openingDate, openingBalanceCents: a.openingBalanceCents,
      balanceCents: model.balanceCents(a), transactionCount: txs.length,
      unassignedCount: txs.filter((t) => !model.isAssigned(t)).length,
    };
  };

  mediator.register<SyncBankAccounts>("SyncBankAccounts", () => {
    const path = join(dataDir, "accounts.json");
    if (!existsSync(path)) return { registered: [] };
    const file = JSON.parse(readFileSync(path, "utf8")) as AccountsFile;
    const registered: string[] = [];
    for (const a of file.accounts) {
      const slug = slugify(a.name);
      if (model.accountBySlug(slug)) continue;
      if (!findBank(a.bank)) throw new Error(`Unknown bank '${a.bank}' for account '${a.name}'`);
      if (!/^\d{4}-\d{2}-\d{2}$/.test(a.openingDate)) throw new Error(`Opening date for '${a.name}' must be YYYY-MM-DD`);
      const accountId = crypto.randomUUID();
      store.append(`bank-account-${accountId}`, 0, [
        {
          type: "BankAccountRegistered",
          data: {
            accountId, slug, name: a.name, bankCode: a.bank, openingDate: a.openingDate,
            openingBalanceCents: toCents(String(a.openingBalance)), statement: a.statement,
          },
        },
      ]);
      registered.push(a.name);
    }
    return { registered };
  });

  mediator.register<ImportTransactions>("ImportTransactions", (cmd) => {
    const accounts = cmd.slug ? model.accounts.filter((a) => a.slug === cmd.slug) : model.accounts;
    if (cmd.slug && accounts.length === 0) throw new Error(`Unknown bank account '${cmd.slug}'`);
    let imported = 0;
    let skipped = 0;
    for (const account of accounts) {
      const path = join(dataDir, account.statement);
      if (!existsSync(path)) continue; // nothing downloaded yet
      const lines = findBank(account.bankCode)!.parseStatement(readFileSync(path, "utf8"));
      const seen = new Map<string, number>();
      const events = [];
      for (const line of lines) {
        if (line.date <= account.openingDate) continue; // already in the opening balance
        const key = [line.date, line.description, line.amountCents, line.balanceCents].join("|");
        const nth = seen.get(key) ?? 0;
        seen.set(key, nth + 1);
        const transactionId = fingerprint(account.id, line, nth);
        if (model.transactions.has(transactionId)) { skipped++; continue; }
        events.push({
          type: "TransactionImported",
          data: {
            accountId: account.id, transactionId, date: line.date, description: line.description,
            amountCents: line.amountCents, balanceCents: line.balanceCents, bankCategory: line.category,
          },
        });
      }
      if (events.length) store.append(`transactions-${account.id}`, "any", events);
      imported += events.length;
    }
    return { imported, skipped };
  });

  mediator.register<GetKnownBanks>("GetKnownBanks", () => knownBanks.map(({ code, name }) => ({ code, name })));

  mediator.register<GetBankAccounts>("GetBankAccounts", () => model.accounts.map(summary));

  mediator.register<GetTransactions>("GetTransactions", (q) => {
    const account = model.accountBySlug(q.slug);
    if (!account) throw new Error("Bank account not found");
    const filter = q.filter ?? "all";
    const names = new Map(model.budgetAccounts.map((b) => [b.id, b.name]));
    const transferOf = (t: Transaction): TransactionRow["transfer"] => {
      const other = t.transferWith ? model.transactions.get(t.transferWith) : undefined;
      const otherAccount = other && model.accounts.find((a) => a.id === other.accountId);
      return other && otherAccount
        ? { transactionId: other.id, accountName: otherAccount.name, accountSlug: otherAccount.slug }
        : null;
    };
    const rows = model
      .transactionsOf(account.id)
      .filter((t) => filter === "all" || (filter === "assigned") === model.isAssigned(t))
      .sort((a: Transaction, b: Transaction) => b.date.localeCompare(a.date) || b.position - a.position)
      .map((t): TransactionRow => ({
        id: t.id, date: t.date, description: t.description, amountCents: t.amountCents,
        balanceCents: t.balanceCents, bankCategory: t.bankCategory,
        allocations: t.allocations.map((al) => ({ ...al, budgetAccountName: names.get(al.budgetAccountId) ?? "?" })),
        transfer: transferOf(t),
      }));
    return { account: summary(account), transactions: rows };
  });
}
