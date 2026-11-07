import { After, AfterAll, Before, BeforeAll, setDefaultTimeout } from "@cucumber/cucumber";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import type { Server } from "node:http";
import type { AddressInfo } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { chromium, type Browser, type Page } from "playwright";
import { createServer as createVite, type ViteDevServer } from "vite";
import { createApp } from "../../src/core/app.ts";
import type { Request } from "../../src/core/mediator.ts";
import { startServer } from "../../src/server/server.ts";

setDefaultTimeout(15_000);

/**
 * Real SPA (Vite), real API, real SQLite file in a temp dir, real browser.
 * All test data is made up; nothing here reads the real data/ folder.
 */
export const env = {
  browser: undefined as unknown as Browser,
  vite: undefined as unknown as ViteDevServer,
  api: undefined as unknown as Server,
  app: undefined as unknown as ReturnType<typeof createApp>,
  dir: "",
  dataDir: "",
  page: undefined as unknown as Page,
  baseUrl: "",
  /** Accounts declared in Given steps, written to accounts.json when first needed. */
  accounts: [] as Record<string, unknown>[],

  send: <R extends Request>(request: R) => env.app.mediator.send(request),

  /** Write a file under the temp data dir (e.g. a downloaded bank statement). */
  writeData(relativePath: string, content: string) {
    const path = join(env.dataDir, relativePath);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, content);
  },
  writeAccountsFile() {
    env.writeData("accounts.json", JSON.stringify({ accounts: env.accounts }));
  },
};

BeforeAll(async () => {
  env.browser = await chromium.launch();
});

AfterAll(async () => {
  await env.browser.close();
});

Before(async () => {
  env.dir = mkdtempSync(join(tmpdir(), "our-budget-"));
  env.dataDir = join(env.dir, "data");
  env.accounts = [];
  mkdirSync(env.dataDir, { recursive: true });
  env.app = createApp({ dbPath: join(env.dataDir, "databases", "our-budget.db"), dataDir: env.dataDir });
  env.api = await startServer(env.app.mediator, 0);
  const apiPort = (env.api.address() as AddressInfo).port;
  env.vite = await createVite({
    configFile: false,
    root: "src/client",
    logLevel: "silent",
    appType: "spa",
    server: { port: 5173, proxy: { "/api/": `http://127.0.0.1:${apiPort}` } },
  });
  await env.vite.listen();
  env.baseUrl = env.vite.resolvedUrls!.local[0]!.replace(/\/$/, "");
  env.page = await env.browser.newPage();
});

After(async () => {
  await env.page.close();
  await env.vite.close();
  await new Promise((r) => env.api.close(r));
  env.app.store.close();
  rmSync(env.dir, { recursive: true, force: true });
});
