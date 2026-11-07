import { Given, Then, When } from "@cucumber/cucumber";
import assert from "node:assert/strict";
import type { GetBankAccounts, GetTransactions } from "../../src/core/bank-accounts.ts";
import type { AssignTransaction } from "../../src/core/budgets.ts";
import { env } from "../support/world.ts";
import { budgetAccountIdByName } from "../budgets/budgets.steps.ts";

const row = (description: string) => env.page.locator("tr[data-id]").filter({ hasText: description });

/** Find an imported transaction by description, across all bank accounts. */
async function transactionId(description: string): Promise<{ id: string; amountCents: number }> {
  const accounts = await env.send<GetBankAccounts>({ type: "GetBankAccounts" });
  for (const a of accounts) {
    const result = await env.send<GetTransactions>({ type: "GetTransactions", slug: a.slug });
    const t = result.transactions.find((x) => x.description === description);
    if (t) return { id: t.id, amountCents: t.amountCents };
  }
  throw new Error(`no transaction '${description}'`);
}

Given("the transaction {string} is assigned to {string}", async (description: string, budgetAccount: string) => {
  const tx = await transactionId(description);
  await env.send<AssignTransaction>({
    type: "AssignTransaction",
    transactionId: tx.id,
    allocations: [{ budgetAccountId: await budgetAccountIdByName(budgetAccount), amountCents: tx.amountCents }],
  });
});

Then("I should see the transactions:", async (table: { raw(): string[][] }) => {
  const expected = table.raw().map((r) => r[0]);
  await env.page.locator("tr[data-id]").first().waitFor();
  const actual = await env.page.locator("tr[data-id] td:nth-child(2)").allTextContents();
  assert.deepEqual(actual, expected);
});

When("I choose the {string} filter", async (label: string) => {
  await env.page.getByRole("navigation", { name: "Transaction filter" }).getByRole("link", { name: label, exact: true }).click();
  await env.page.waitForFunction((l) => document.querySelector(".tabs a[aria-current]")?.textContent === l, label);
});

const openAssign = async (description: string) => {
  await row(description).getByRole("button", { name: /Assign|Change/ }).click();
  return env.page.locator(".assign-form");
};

When("I assign {string} to {string}", async (description: string, budgetAccount: string) => {
  const form = await openAssign(description);
  const option = form.locator("option", { hasText: new RegExp(`: ${budgetAccount}$`) });
  await form.getByLabel("Budget account").selectOption((await option.getAttribute("value"))!);
  await form.getByRole("button", { name: "Save" }).click();
});

When("I split {string} across:", async (description: string, table: { raw(): string[][] }) => {
  const form = await openAssign(description);
  const lines = table.raw();
  for (let i = 1; i < lines.length; i++) await form.getByRole("button", { name: "Add split" }).click();
  for (const [i, [name, amount]] of lines.entries()) {
    const split = form.locator(".split").nth(i);
    const option = split.locator("option", { hasText: new RegExp(`: ${name}$`) });
    await split.getByLabel("Budget account").selectOption((await option.getAttribute("value"))!);
    await split.getByLabel("Amount").fill(amount!);
  }
  await form.getByRole("button", { name: "Save" }).click();
});

When("I unassign {string}", async (description: string) => {
  const form = await openAssign(description);
  await form.getByRole("button", { name: "Unassign" }).click();
});

Then("{string} is assigned to {string}", async (description: string, text: string) => {
  await env.page.waitForFunction(
    ([d, t]) => [...document.querySelectorAll("tr[data-id]")].some((tr) => tr.textContent?.includes(d!) && tr.querySelector(".assigned")?.textContent?.trim() === t),
    [description, text],
  );
});

Then("{string} is not assigned", async (description: string) => {
  await row(description).locator(".assigned", { hasText: "Unassigned" }).waitFor();
});
