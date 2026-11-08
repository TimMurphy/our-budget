import type { GetTransactions, TransactionFilter, TransactionRow } from "../../core/bank-accounts.ts";
import type { BudgetView, TransferCandidate } from "../../core/budgets.ts";
import { toCents } from "../../core/money.ts";
import { send } from "../api.ts";
import { currentMonth, esc, formatCents, formatDate } from "../dom.ts";

const NEW = "__new__";

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
  const groups = budget.groups.map((g) => ({ id: g.id, name: g.name }));

  root.innerHTML = `
    <h2>${esc(account.name)}</h2>
    <p class="muted">${esc(account.bankName)} · balance <span class="amount">${formatCents(account.balanceCents)}</span></p>
    <nav class="tabs" aria-label="Transaction filter">
      ${filters
        .map((f) => `<a href="/bank-accounts/${slug}${f.key === "all" ? "" : `?filter=${f.key}`}"${f.key === filter ? ' aria-current="page"' : ""}>${f.label}</a>`)
        .join("")}
    </nav>
    <div class="table-scroll"><table class="transactions">
      <thead><tr><th class="date">Date</th><th>Description</th><th class="num">Amount</th><th>Assigned to</th><th></th></tr></thead>
      <tbody>${transactions.map((t) => row(t)).join("")}</tbody>
    </table></div>
    ${transactions.length === 0 ? `<p class="muted">No transactions.</p>` : ""}`;

  for (const tr of root.querySelectorAll<HTMLTableRowElement>("tr[data-id]")) {
    const t = transactions.find((x) => x.id === tr.dataset["id"])!;
    const again = () => renderBankAccount(root, url, [slug]);
    tr.querySelector("button.assign")?.addEventListener("click", () => openAssign(tr, t, budgetAccounts, groups, again));
    tr.querySelector("button.transfer")?.addEventListener("click", () => void openTransfer(tr, t, again));
    tr.querySelector("button.unlink")?.addEventListener("click", async () => {
      await send("UnlinkTransfer", { transactionId: t.id });
      await again();
    });
  }
}

function row(t: TransactionRow): string {
  if (t.transfer) {
    const direction = t.amountCents < 0 ? "to" : "from";
    return `
    <tr data-id="${t.id}">
      <td class="date">${formatDate(t.date)}</td>
      <td>${esc(t.description)}</td>
      <td class="num amount">${formatCents(t.amountCents)}</td>
      <td class="assigned">Transfer ${direction} <a href="/bank-accounts/${t.transfer.accountSlug}">${esc(t.transfer.accountName)}</a></td>
      <td><button type="button" class="unlink secondary">Unlink transfer</button></td>
    </tr>`;
  }
  const assigned = t.allocations.length
    ? t.allocations
        .map((a) => `${esc(a.budgetAccountName)}${t.allocations.length > 1 ? ` ${formatCents(a.amountCents)}` : ""}`)
        .join(", ")
    : `<span class="muted">Unassigned</span>`;
  return `
    <tr data-id="${t.id}">
      <td class="date">${formatDate(t.date)}</td>
      <td>${esc(t.description)}</td>
      <td class="num amount">${formatCents(t.amountCents)}</td>
      <td class="assigned">${assigned}</td>
      <td class="actions">
        <button type="button" class="assign secondary">${t.allocations.length ? "Change" : "Assign"}</button>
        <button type="button" class="transfer secondary">Transfer</button>
      </td>
    </tr>`;
}

function openAssign(
  tr: HTMLTableRowElement,
  t: TransactionRow,
  budgetAccounts: { id: string; name: string; group: string }[],
  groups: { id: string; name: string }[],
  done: () => Promise<void>,
): void {
  if (tr.nextElementSibling?.classList.contains("assign-row")) return;
  const options = (selected?: string) =>
    `<option value="">Choose a budget account</option>` +
    budgetAccounts
      .map((b) => `<option value="${b.id}"${b.id === selected ? " selected" : ""}>${esc(b.group)}: ${esc(b.name)}</option>`)
      .join("") +
    `<option value="${NEW}">New budget account…</option>`;
  const groupOptions =
    groups.map((g) => `<option value="${g.id}">${esc(g.name)}</option>`).join("") +
    `<option value="${NEW}"${groups.length ? "" : " selected"}>New group…</option>`;
  const line = (id?: string, cents?: number) => `
    <div class="split">
      <select aria-label="Budget account">${options(id)}</select>
      <input aria-label="Amount" inputmode="decimal" value="${cents === undefined ? "" : (cents / 100).toFixed(2)}">
      <div class="new-account" hidden>
        <label>New account name <input name="newName" autocomplete="off"></label>
        <label>New account group <select name="newGroup">${groupOptions}</select></label>
        <label class="new-group-name"${groups.length ? " hidden" : ""}>New group name <input name="newGroupName" autocomplete="off"></label>
      </div>
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

  // Show the "new budget account" fields when asked for; likewise "new group".
  form.addEventListener("change", (e) => {
    const target = e.target as HTMLSelectElement;
    const split = target.closest(".split");
    if (!split) return;
    if (target.matches("[aria-label='Budget account']")) {
      split.querySelector<HTMLElement>(".new-account")!.hidden = target.value !== NEW;
    }
    if (target.name === "newGroup") {
      split.querySelector<HTMLElement>(".new-group-name")!.hidden = target.value !== NEW;
    }
  });

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
    void (async () => {
      try {
        const newGroups = new Map<string, string>(); // name -> id, so two lines share a new group
        const allocations: { budgetAccountId: string; amountCents: number }[] = [];
        const lines = [...splits.querySelectorAll<HTMLElement>(".split")].map((s) => ({
          choice: s.querySelector<HTMLSelectElement>("select[aria-label='Budget account']")!.value,
          amountCents: toCents(s.querySelector<HTMLInputElement>("input[aria-label='Amount']")!.value),
          name: s.querySelector<HTMLInputElement>("input[name=newName]")!.value.trim(),
          group: s.querySelector<HTMLSelectElement>("select[name=newGroup]")!.value,
          groupName: s.querySelector<HTMLInputElement>("input[name=newGroupName]")!.value.trim(),
        }));
        if (lines.some((l) => !l.choice)) throw new Error("Choose a budget account for every line");
        if (lines.some((l) => l.choice === NEW && !l.name)) throw new Error("Enter a name for the new budget account");

        for (const l of lines) {
          if (l.choice !== NEW) {
            allocations.push({ budgetAccountId: l.choice, amountCents: l.amountCents });
            continue;
          }
          let groupId = l.group;
          if (groupId === NEW) {
            groupId = newGroups.get(l.groupName.toLowerCase()) ?? "";
            if (!groupId) {
              ({ id: groupId } = await send<{ id: string }>("CreateBudgetGroup", { name: l.groupName }));
              newGroups.set(l.groupName.toLowerCase(), groupId);
            }
          }
          const { id } = await send<{ id: string }>("CreateBudgetAccount", { name: l.name, groupId });
          allocations.push({ budgetAccountId: id, amountCents: l.amountCents });
        }
        await save(allocations);
      } catch (err) {
        status.textContent = (err as Error).message;
      }
    })();
  });
}

async function openTransfer(tr: HTMLTableRowElement, t: TransactionRow, done: () => Promise<void>): Promise<void> {
  tr.parentElement?.querySelector(".assign-row")?.remove();
  const candidates = await send<TransferCandidate[]>("GetTransferCandidates", { transactionId: t.id });
  const panel = document.createElement("tr");
  panel.className = "assign-row";
  const direction = t.amountCents < 0 ? "went to" : "came from";
  panel.innerHTML = `
    <td colspan="5">
      <form class="transfer-form">
        ${
          candidates.length
            ? `<label>This ${direction} another bank account. Matching transaction
                <select aria-label="Matching transaction">
                  ${candidates
                    .map((c) => `<option value="${c.transactionId}">${esc(c.accountName)}: ${formatDate(c.date)} ${esc(c.description)} ${formatCents(c.amountCents)}</option>`)
                    .join("")}
                </select></label>
               <p><button type="submit">Link transfer</button>
               <button type="button" class="cancel secondary">Cancel</button></p>`
            : `<p class="muted">No matching transaction in another bank account. Import its statement and try again.</p>
               <p><button type="button" class="cancel secondary">Close</button></p>`
        }
        <p role="status"></p>
      </form>
    </td>`;
  tr.after(panel);
  panel.querySelector(".cancel")!.addEventListener("click", () => panel.remove());
  panel.querySelector("form")!.addEventListener("submit", async (e) => {
    e.preventDefault();
    try {
      await send("LinkTransfer", { transactionId: t.id, counterpartTransactionId: panel.querySelector("select")!.value });
      await done();
    } catch (err) {
      panel.querySelector<HTMLElement>("[role=status]")!.textContent = (err as Error).message;
    }
  });
}
