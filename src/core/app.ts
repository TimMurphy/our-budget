/** Application wiring: event store + mediator + read model. */
import { registerBankAccounts } from "./bank-accounts.ts";
import { registerBudgets } from "./budgets.ts";
import { EventStore } from "./event-store.ts";
import { Mediator } from "./mediator.ts";
import { Model } from "./model.ts";

export interface AppOptions {
  /** SQLite file, e.g. data/databases/our-budget.db */
  dbPath: string;
  /** Folder holding accounts.json and downloaded bank statements, e.g. data */
  dataDir: string;
}

export function createApp({ dbPath, dataDir }: AppOptions): { mediator: Mediator; store: EventStore } {
  const store = new EventStore(dbPath);
  const mediator = new Mediator();
  const model = new Model();
  store.readAll().forEach((e) => model.apply(e)); // rebuild on start
  store.subscribe((e) => model.apply(e));
  registerBankAccounts(mediator, store, model, dataDir);
  registerBudgets(mediator, store, model);
  return { store, mediator };
}
