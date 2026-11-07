import type { BankAccountSummary } from "../../core/bank-accounts.ts";
import type { BudgetView } from "../../core/budgets.ts";
import { send } from "../api.ts";
import { currentMonth, esc, formatCents } from "../dom.ts";

export async function renderHome(root: HTMLElement): Promise<void> {
  const [accounts, budget] = await Promise.all([
    send<BankAccountSummary[]>("GetBankAccounts"),
    send<BudgetView>("GetBudget", { month: currentMonth() }),
  ]);
  root.innerHTML = `
    <div class="cards">
      <section class="card" aria-labelledby="bank-accounts-card">
        <h2 id="bank-accounts-card"><a href="/bank-accounts">Bank Accounts</a></h2>
        <ul class="plain">
          ${accounts
            .map((a) => `<li><a href="/bank-accounts/${a.slug}">${esc(a.name)}</a> <span class="amount">${formatCents(a.balanceCents)}</span></li>`)
            .join("")}
        </ul>
      </section>
      <section class="card" aria-labelledby="budgets-card">
        <h2 id="budgets-card"><a href="/budgets">Budgets</a></h2>
        <p>To budget this month <span class="amount">${formatCents(budget.toBudgetCents)}</span></p>
      </section>
    </div>`;
}
