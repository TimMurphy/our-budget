import { Then, When } from "@cucumber/cucumber";
import assert from "node:assert/strict";
import { env } from "./support/world.ts";

When("I goto {}", async (path: string) => {
  await env.page.goto(env.baseUrl + path);
  await env.page.locator("main > *").first().waitFor();
});

Then("I should see the message {string}", async (message: string) => {
  await env.page.getByRole("status").filter({ hasText: message }).first().waitFor();
});

Then("I should see the error {string}", async (message: string) => {
  assert.match(await env.page.getByRole("alert").first().innerText(), new RegExp(message));
});
