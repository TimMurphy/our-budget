import { Given, Then, When } from "@cucumber/cucumber";
import assert from "node:assert/strict";
import type { BudgetView, CreateBudgetAccount, CreateBudgetGroup, GetBudget } from "../../src/core/budgets.ts";
import { env } from "../support/world.ts";

const anyMonth = "2026-11";

async function view(): Promise<BudgetView> {
  return env.send<GetBudget>({ type: "GetBudget", month: anyMonth });
}

async function groupIdByName(name: string): Promise<string> {
  const g = (await view()).groups.find((x) => x.name === name);
  assert.ok(g, `no budget group ${name}`);
  return g.id;
}

export async function budgetAccountIdByName(name: string): Promise<string> {
  const a = (await view()).groups.flatMap((g) => g.accounts).find((x) => x.name === name);
  assert.ok(a, `no budget account ${name}`);
  return a.id;
}

Given("a budget group {string}", async (name: string) => {
  await env.send<CreateBudgetGroup>({ type: "CreateBudgetGroup", name });
});

Given("a budget account {string} in group {string}", async (name: string, group: string) => {
  await env.send<CreateBudgetAccount>({ type: "CreateBudgetAccount", name, groupId: await groupIdByName(group) });
});

Given("an income budget account {string} in group {string}", async (name: string, group: string) => {
  await env.send<CreateBudgetAccount>({ type: "CreateBudgetAccount", name, groupId: await groupIdByName(group), kind: "income" });
});

const group = (name: string) => env.page.locator(`section.group[aria-label="${name}"]`);

When("I add the group {string}", async (name: string) => {
  const form = env.page.locator("#add-group");
  await form.getByLabel("Group name").fill(name);
  await form.getByRole("button", { name: "Add Group" }).click();
  await group(name).waitFor();
});

When("I add the budget account {string} to group {string}", async (name: string, groupName: string) => {
  const form = env.page.locator("#add-budget-account");
  await form.getByLabel("Budget account name").fill(name);
  await form.getByLabel("Group").selectOption({ label: groupName });
  await form.getByRole("button", { name: "Add Budget Account" }).click();
  await env.page.locator("main").filter({ has: env.page.locator("[role=status]:not(:empty), li[data-budget-account]") }).first().waitFor();
});

When(
  "I add the budget account {string} to group {string} with auto {word} amount {float}",
  async (name: string, groupName: string, cadence: string, amount: number) => {
    const form = env.page.locator("#add-budget-account");
    await form.getByLabel("Auto monthly").selectOption(cadence);
    await form.getByLabel("Auto amount").fill(amount.toFixed(2));
    await form.getByLabel("Budget account name").fill(name);
    await form.getByLabel("Group").selectOption({ label: groupName });
    await form.getByRole("button", { name: "Add Budget Account" }).click();
    await env.page.locator(`li[data-budget-account]:has-text("${name}")`).waitFor();
  },
);

When("I drag budget account {string} to group {string}", async (name: string, groupName: string) => {
  await env.page.locator(`li[data-budget-account]:has-text("${name}")`).dragTo(group(groupName));
  await group(groupName).locator(`li:has-text("${name}")`).waitFor();
});

When("I set the budgeted amount for {string} to {float}", async (name: string, amount: number) => {
  const input = env.page.getByLabel(`Budgeted for ${name}`);
  await input.fill(amount.toFixed(2));
  await input.blur();
  await env.page.waitForFunction(
    ([n, v]) => (document.querySelector(`input[aria-label="Budgeted for ${n}"]`) as HTMLInputElement | null)?.value === v,
    [name, amount.toFixed(2)],
  );
});

Then("group {string} contains budget account {string}", async (groupName: string, name: string) => {
  await group(groupName).locator(`li:has-text("${name}")`).waitFor();
});

Then("group {string} does not contain budget account {string}", async (groupName: string, name: string) => {
  assert.equal(await group(groupName).locator(`li:has-text("${name}")`).count(), 0);
});

Then("the budgeted amount for {string} is {float}", async (name: string, amount: number) => {
  const input = env.page.getByLabel(`Budgeted for ${name}`);
  await input.waitFor();
  assert.equal(await input.inputValue(), amount.toFixed(2));
});

const summaryKey: Record<string, string> = { "Income last month": "income", Budgeted: "budgeted", "To budget": "to-budget" };
Then("the summary shows {string} of {string}", async (label: string, expected: string) => {
  await env.page.locator(`[data-summary="${summaryKey[label]}"]`).filter({ hasText: expected }).waitFor();
});
