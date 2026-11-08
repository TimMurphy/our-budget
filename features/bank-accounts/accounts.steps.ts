import { Given, Then, When } from "@cucumber/cucumber";
import assert from "node:assert/strict";
import type { GetBankAccounts, ImportTransactions, SyncBankAccounts } from "../../src/core/bank-accounts.ts";
import { env } from "../support/world.ts";

const MONTHS = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
const banks: Record<string, string> = { NAB: "nab", Westpac: "wbc" };
const statementPath = (name: string) => `bank-transactions/${name.toLowerCase().replace(/\W+/g, "-")}.csv`;
const cell = (v: string) => (/[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);

/** "30 Oct 2026" -> "2026-10-30" */
function isoDate(text: string): string {
  const [d, m, y] = text.split(" ") as [string, string, string];
  return `${y}-${String(MONTHS.indexOf(m.toLowerCase()) + 1).padStart(2, "0")}-${d.padStart(2, "0")}`;
}

export let lastImport: { imported: number; skipped: number; autoAssigned: number } = { imported: 0, skipped: 0, autoAssigned: 0 };

Given(
  "a {word} account {string} with opening balance {float} at close of business {string}",
  (bank: string, name: string, balance: number, date: string) => {
    env.accounts.push({
      name, bank: banks[bank], openingDate: isoDate(date), openingBalance: balance, statement: statementPath(name),
    });
  },
);

Given(
  "the downloaded statement for {string} contains:",
  (name: string, table: { raw(): string[][] }) => {
    env.writeData(statementPath(name), table.raw().map((r) => r.map(cell).join(",")).join("\n") + "\n");
  },
);
When("the transactions are imported", async () => {
  env.writeAccountsFile();
  await env.send<SyncBankAccounts>({ type: "SyncBankAccounts" });
  lastImport = await env.send<ImportTransactions>({ type: "ImportTransactions" });
});

const accountNamed = async (name: string) => {
  const account = (await env.send<GetBankAccounts>({ type: "GetBankAccounts" })).find((a) => a.name === name);
  assert.ok(account, `no bank account called ${name}`);
  return account;
};

Then("{string} has {int} transaction(s)", async (name: string, count: number) => {
  assert.equal((await accountNamed(name)).transactionCount, count);
});

Then("{string} has a balance of {float}", async (name: string, balance: number) => {
  assert.equal((await accountNamed(name)).balanceCents, Math.round(balance * 100));
});

Then("{int} transaction(s) was/were imported and {int} skipped", (imported: number, skipped: number) => {
  assert.deepEqual({ imported: lastImport.imported, skipped: lastImport.skipped }, { imported, skipped });
});
