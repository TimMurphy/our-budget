import { Given, Then, When } from "@cucumber/cucumber";
import assert from "node:assert/strict";
import type { CreateAutoAssignRule, GetBudget } from "../../src/core/budgets.ts";
import { env } from "../support/world.ts";
import { budgetAccountIdByName } from "../budgets/budgets.steps.ts";
import { lastImport } from "./accounts.steps.ts";

const row = (text: string) => env.page.locator("tr[data-id]").filter({ hasText: text });
const rowDescriptions = () => env.page.locator("tr[data-id] td:nth-child(2)").allTextContents();

Given("an auto assign rule {string} for {string}", async (pattern: string, budgetAccount: string) => {
  await env.send<CreateAutoAssignRule>({
    type: "CreateAutoAssignRule",
    pattern,
    budgetAccountId: await budgetAccountIdByName(budgetAccount),
  });
});

When(
  "I assign {string} to {string} and always assign {string}",
  async (description: string, budgetAccount: string, pattern: string) => {
    await row(description).getByRole("button", { name: /Assign|Change/ }).click();
    const form = env.page.locator(".assign-form");
    const option = form.locator("option", { hasText: new RegExp(`: ${budgetAccount}$`) });
    await form.getByLabel("Budget account", { exact: true }).selectOption((await option.getAttribute("value"))!);
    await form.getByLabel("Always assign similar transactions").check();
    await form.getByLabel("Text to match").fill(pattern);
    await form.getByRole("button", { name: "Save" }).click();
  },
);

Then("the transactions in the {string} filter are:", async (label: string, table: { raw(): string[][] }) => {
  await env.page.getByRole("navigation", { name: "Transaction filter" }).getByRole("link", { name: label, exact: true }).click();
  await env.page.waitForFunction((l) => document.querySelector(".tabs a[aria-current]")?.textContent === l, label);
  await env.page.locator("table.transactions").waitFor();
  assert.deepEqual(await rowDescriptions(), table.raw().map((r) => r[0]));
});

Then("there are no transactions in the {string} filter", async (label: string) => {
  await env.page.getByRole("navigation", { name: "Transaction filter" }).getByRole("link", { name: label, exact: true }).click();
  await env.page.waitForFunction((l) => document.querySelector(".tabs a[aria-current]")?.textContent === l, label);
  await env.page.getByText("No transactions.").waitFor();
});

Then("{string} is pending approval for {string}", async (description: string, budgetAccount: string) => {
  const cell = row(description).locator(".assigned");
  await cell.getByText("Pending approval").waitFor();
  assert.ok((await cell.innerText()).includes(budgetAccount));
});

When("I approve {string}", async (description: string) => {
  await row(description).getByRole("button", { name: "Approve" }).click();
  await row(description).waitFor({ state: "detached" });
});

When("I approve all", async () => {
  await env.page.getByRole("button", { name: "Approve all" }).click();
  await env.page.getByText("No transactions.").waitFor();
});

Then("{int} transaction was imported and {int} auto assigned", (imported: number, autoAssigned: number) => {
  assert.equal(lastImport.imported, imported);
  assert.equal(lastImport.autoAssigned, autoAssigned);
});

Then("the November 2026 activity of {string} is {float}", async (name: string, expected: number) => {
  const view = await env.send<GetBudget>({ type: "GetBudget", month: "2026-11" });
  const account = view.groups.flatMap((g) => g.accounts).find((a) => a.name === name);
  assert.equal(account?.activityCents, Math.round(expected * 100) || 0);
});

When("I add the auto assign rule {string} for {string}", async (pattern: string, budgetAccount: string) => {
  const form = env.page.locator("#add-rule");
  await form.getByLabel("Text to match").fill(pattern);
  const option = form.locator("option", { hasText: new RegExp(`: ${budgetAccount}$`) });
  await form.getByLabel("Rule budget account").selectOption((await option.getAttribute("value"))!);
  await form.getByRole("button", { name: "Add Rule" }).click();
});

Then("the rules list contains {string}", async (pattern: string) => {
  await env.page.locator(".rules-table .pattern", { hasText: pattern }).waitFor();
});

When("I delete the auto assign rule {string}", async (pattern: string) => {
  await env.page.getByRole("button", { name: `Delete rule ${pattern}` }).click();
});

Then("the rules list does not contain {string}", async (pattern: string) => {
  await env.page.locator(".rules-table .pattern", { hasText: pattern }).waitFor({ state: "detached" });
});
