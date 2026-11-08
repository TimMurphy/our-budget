import { Given, Then } from "@cucumber/cucumber";
import assert from "node:assert/strict";
import { env } from "../support/world.ts";

Given("the browser language is {string}", async (locale: string) => {
  await env.page.close();
  env.page = await env.browser.newPage({ locale });
});

Given("the browser window is {int} pixels wide", async (width: number) => {
  await env.page.setViewportSize({ width, height: 800 });
});

const dateCell = (description: string) =>
  env.page.locator("tr[data-id]").filter({ hasText: description }).locator("td.date");

Then("the date of {string} is shown as {string}", async (description: string, expected: string) => {
  assert.equal((await dateCell(description).innerText()).trim(), expected);
});

Then("the date of {string} is on one line", async (description: string) => {
  const cell = dateCell(description);
  assert.equal(await cell.evaluate((el) => getComputedStyle(el).whiteSpace), "nowrap");
  // The text itself fits on one line (the cell's own height is the whole row's, so measure the text).
  const { height, line } = await cell.evaluate((el) => {
    const range = document.createRange();
    range.selectNodeContents(el);
    return { height: range.getBoundingClientRect().height, line: parseFloat(getComputedStyle(el).lineHeight) };
  });
  assert.ok(height <= line + 1, `date wrapped: text is ${height}px tall for a ${line}px line`);
});
