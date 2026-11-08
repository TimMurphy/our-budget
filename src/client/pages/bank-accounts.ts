import type { BankAccountSummary, ImportTransactions } from "../../core/bank-accounts.ts";
import { send } from "../api.ts";
import { esc, formatCents } from "../dom.ts";

export async function renderBankAccounts(root: HTMLElement): Promise<void> {
  const accounts = await send<BankAccountSummary[]>("GetBankAccounts");
  root.innerHTML = `
    <div class="page-head">
      <h2>Bank Accounts</h2>
      <button type="button" id="import">Import now</button>
    </div>
    <p role="status"></p>
    <div class="cards">
      ${accounts
        .map(
          (a) => `
        <section class="card">
          <h3><a href="/bank-accounts/${a.slug}">${esc(a.name)}</a></h3>
          <p class="muted">${esc(a.bankName)}</p>
          <p class="amount big">${formatCents(a.balanceCents)}</p>
          <p class="muted">${a.transactionCount} transactions, ${a.unassignedCount} unassigned${a.pendingApprovalCount ? `, <a href="/bank-accounts/${a.slug}?filter=pending">${a.pendingApprovalCount} pending approval</a>` : ""}</p>
        </section>`,
        )
        .join("")}
    </div>`;

  const status = root.querySelector<HTMLElement>("[role=status]")!;
  root.querySelector("#import")!.addEventListener("click", async () => {
    try {
      const { imported, skipped } = await send<NonNullable<ImportTransactions["__result"]>>("ImportTransactions");
      const message = `Imported ${imported} new transactions (${skipped} already imported)`;
      if (imported) await renderBankAccounts(root);
      root.querySelector<HTMLElement>("[role=status]")!.textContent = message;
    } catch (err) {
      status.textContent = (err as Error).message;
    }
  });
}
