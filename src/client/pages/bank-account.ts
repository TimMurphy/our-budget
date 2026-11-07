import type { GetTransactions, TransactionFilter, TransactionRow } from "../../core/bank-accounts.ts";
import type { BudgetView } from "../../core/budgets.ts";
import { toCents } from "../../core/money.ts";
import { send } from "../api.ts";
import { currentMonth, esc, formatCents } from "../dom.ts";

const filters: { key: TransactionFilter; label: string }[] = [
  { key: "all", label: "All transactions" },
  { key: "unassigned", label: "Unassigned" },
  { key: "assigned", label: "Assigned" },
];

export async function renderBankAccount(root: HTMLElement, url: URL, params: string[]): Promise<void> {
  const slug = params[0]!;
  const filter = (url.searchParams.get("filter") as TransactionFilter | null) ?? "all";
  const [{ account, transactions }, budget] = await Promise.all([
    send<NonNullable<GetTransactions["__result"]>>("GetTransactions", { slug, filter }),
    send<BudgetView>("GetBudget", { month: currentMonth() }),
  ]);
  const budgetAccounts = budget.groups.flatMap((g) => g.accounts.map((a) => ({ id: a.id, name: a.name, group: g.name })));

  root.innerHTML = `
    <h2>${esc(account.name)}</h2>
    <p class="muted">${esc(account.bankName)} · balance <span class="amount">${formatCents(account.balanceCents)}</span></p>
    <nav class="tabs" aria-label="Transaction filter">
      ${filters
        .map((f) => `<a href="/bank-accounts/${slug}${f.key === "all" ? "" : `?filter=${f.key}`}"${f.key === filter ? ' aria-current="page"' : ""}>${f.label}</a>`)
        .join("")}
    </nav>
    <table class="transactions">
      <thead><tr><th>Date</th><th>Description</th><th class="num">Amount</th><th>Assigned to</th><th></th></tr></thead>
      <tbody>${transactions.map((t) => row(t)).join("")}</tbody>
    </table>
    ${transactions.length === 0 ? `<p class="muted">No transactions.</p>` : ""}`;

  for (const tr of root.querySelectorAll<HTMLTableRowElement>("tr[data-id]")) {
    const t = transactions.find((x) => x.id === tr.dataset["id"])!;
    tr.querySelector("button.assign")!.addEventListener("click", () =>
      openAssign(tr, t, budgetAccounts, () => renderBankAccount(root, url, [slug])),
    );
  }
}

function row(t: TransactionRow): string {
  const assigned = t.allocations.length
    ? t.allocations
        .map((a) => `${esc(a.budgetAccountName)}${t.allocations.length > 1 ? ` ${formatCents(a.amountCents)}` : ""}`)
        .join(", ")
    : `<span class="muted">Unassigned</span>`;
  return `
    <tr data-id="${t.id}">
      <td>${t.date}</td>
      <td>${esc(t.description)}</td>
      <td class="num amount">${formatCents(t.amountCents)}</td>
      <td class="assigned">${assigned}</td>
      <td><button type="button" class="assign secondary">${t.allocations.length ? "Change" : "Assign"}</button></td>
    </tr>`;
}

function openAssign(
  tr: HTMLTableRowElement,
  t: TransactionRow,
  budgetAccounts: { id: string; name: string; group: string }[],
  done: () => Promise<void>,
): void {
  if (tr.nextElementSibling?.classList.contains("assign-row")) return;
  const options = (selected?: string) =>
    `<option value="">Choose a budget account</option>` +
    budgetAccounts
      .map((b) => `<option value="${b.id}"${b.id === selected ? " selected" : ""}>${esc(b.group)}: ${esc(b.name)}</option>`)
      .join("");
  const line = (id?: string, cents?: number) => `
    <div class="split">
      <select aria-label="Budget account">${options(id)}</select>
      <input aria-label="Amount" inputmode="decimal" value="${cents === undefined ? "" : (cents / 100).toFixed(2)}">
    </div>`;
  const lines = t.allocations.length
    ? t.allocations.map((a) => line(a.budgetAccountId, a.amountCents)).join("")
    : line(undefined, t.amountCents);

  const panel = document.createElement("tr");
  panel.className = "assign-row";
  panel.innerHTML = `
    <td colspan="5">
      <form class="assign-form">
        <div class="splits">${lines}</div>
        <p>
          <button type="button" class="add-split secondary">Add split</button>
          <button type="submit">Save</button>
          <button type="button" class="cancel secondary">Cancel</button>
          <button type="button" class="clear secondary">Unassign</button>
        </p>
        <p role="status"></p>
      </form>
    </td>`;
  tr.after(panel);
  const form = panel.querySelector("form")!;
  const status = panel.querySelector<HTMLElement>("[role=status]")!;
  const splits = panel.querySelector(".splits")!;

  panel.querySelector(".add-split")!.addEventListener("click", () => splits.insertAdjacentHTML("beforeend", line()));
  panel.querySelector(".cancel")!.addEventListener("click", () => panel.remove());
  const save = async (allocations: { budgetAccountId: string; amountCents: number }[]) => {
    try {
      await send("AssignTransaction", { transactionId: t.id, allocations });
      await done();
    } catch (err) {
      status.textContent = (err as Error).message;
    }
  };
  panel.querySelector(".clear")!.addEventListener("click", () => void save([]));
  form.addEventListener("submit", (e) => {
    e.preventDefault();
    try {
      const allocations = [...splits.querySelectorAll(".split")].map((s) => ({
        budgetAccountId: s.querySelector("select")!.value,
        amountCents: toCents(s.querySelector("input")!.value),
      }));
      if (allocations.some((a) => !a.budgetAccountId)) throw new Error("Choose a budget account for every line");
      void save(allocations);
    } catch (err) {
      status.textContent = (err as Error).message;
    }
  });
}
