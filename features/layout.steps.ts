import { Then } from "@cucumber/cucumber";
import assert from "node:assert/strict";
import { env } from "./support/world.ts";

const rgb = (hex: string) => {
  const n = parseInt(hex.slice(1), 16);
  return `rgb(${n >> 16}, ${(n >> 8) & 255}, ${n & 255})`;
};
const style = (selector: string, property: "backgroundColor" | "color") =>
  env.page.locator(selector).first().evaluate((el, p) => getComputedStyle(el)[p], property);

Then("the top row is fixed to the top of the browser", async () => {
  const header = env.page.locator("header");
  assert.equal(await header.evaluate((el) => getComputedStyle(el).position), "fixed");
  await env.page.evaluate(() => {
    document.body.style.minHeight = "3000px";
    scrollTo(0, 1000);
  });
  assert.equal((await header.boundingBox())!.y, 0);
});

Then("the top left of the screen displays {string}", async (text: string) => {
  const box = (await env.page.locator("header").getByText(text, { exact: true }).boundingBox())!;
  assert.ok(box.x < env.page.viewportSize()!.width / 4, "starts in the left quarter");
  assert.ok(box.y < 56, "sits in the top row");
});

Then("the right side contains a navbar with {string} and {string}", async (a: string, b: string) => {
  const nav = env.page.locator("header nav");
  assert.ok((await nav.boundingBox())!.x > env.page.viewportSize()!.width / 2, "navbar is in the right half");
  await nav.getByRole("link", { name: a }).waitFor();
  await nav.getByRole("link", { name: b }).waitFor();
});

Then("the page background colour is {string}", async (hex: string) => {
  assert.equal(await style("body", "backgroundColor"), rgb(hex));
});

Then("the navbar and cards background colour is {string}", async (hex: string) => {
  for (const selector of ["header", "main .card"]) {
    assert.equal(await style(selector, "backgroundColor"), rgb(hex), selector);
  }
});

Then("the foreground colour is {string}", async (hex: string) => {
  assert.equal(await style("body", "color"), rgb(hex));
});

const card = (heading: string) =>
  env.page.locator("main .card").filter({ has: env.page.getByRole("heading", { name: heading }) });

Then("the body contains a card with heading {string}", async (heading: string) => {
  await card(heading).first().waitFor();
});

Then("the {string} card lists {string}", async (heading: string, text: string) => {
  await card(heading).first().getByText(text).waitFor();
});
