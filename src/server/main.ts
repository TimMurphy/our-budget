import { mkdirSync } from "node:fs";
import { createApp } from "../core/app.ts";
import type { ImportTransactions, SyncBankAccounts } from "../core/bank-accounts.ts";
import { startServer } from "./server.ts";

const dataDir = "data";
mkdirSync(`${dataDir}/bank-transactions`, { recursive: true });
const { mediator } = createApp({ dbPath: `${dataDir}/databases/our-budget.db`, dataDir });

const importAll = async () => {
  const { imported } = await mediator.send<ImportTransactions>({ type: "ImportTransactions" });
  if (imported) console.log(`Imported ${imported} transactions`);
};

const { registered } = await mediator.send<SyncBankAccounts>({ type: "SyncBankAccounts" });
if (registered.length) console.log(`Registered bank accounts: ${registered.join(", ")}`);
await importAll();
setInterval(() => importAll().catch(console.error), 60 * 60 * 1000); // hourly

await startServer(mediator, 4001);
console.log("API on http://127.0.0.1:4001");
