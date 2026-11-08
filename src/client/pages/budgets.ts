import type { AutoAssignRuleView, BudgetAccountView, BudgetView } from "../../core/budgets.ts";
import { toCents } from "../../core/money.ts";
import { send } from "../api.ts";
import { currentMonth, esc, formatCents } from "../dom.ts";

const cadences = ["yearly", "quarterly", "monthly", "fortnightly", "weekly"] as const;
const cadenceLabel: Record<string, string> = {
  yearly: "Yearly", quarterly: "Quarterly", monthly: "Monthly", fortnightly: "Fortnightly", weekly: "Weekly",
};

export async function renderBudgets(root: HTMLElement, url: URL): Promise<void> {
  const month = url.searchParams.get("month") || currentMonth();
  const [budget, rules] = await Promise.all([
    send<BudgetView>("GetBudget", { month }),
    send<AutoAssignRuleView[]>("GetAutoAssignRules"),
  ]);
  const refresh = () => renderBudgets(root, url);

  root.innerHTML = `
    <div class="page-head">
      <h2>Budgets</h2>
      <form class="month-form"><label>Month <input type="month" name="month" value="${month}"></label></form>
    </div>
    <section class="summary" aria-label="Summary">
      <div><span class="muted">Income last month</span><strong class="amount" data-summary="income">${formatCents(budget.incomeLastMonthCents)}</strong></div>
      <div><span class="muted">Budgeted</span><strong class="amount" data-summary="budgeted">${formatCents(budget.budgetedCents)}</strong></div>
      <div><span class="muted">To budget</span><strong class="amount" data-summary="to-budget">${formatCents(budget.toBudgetCents)}</strong></div>
    </section>
    <p role="status"></p>
    <div class="groups">${budget.groups.map((g) => groupHtml(g)).join("")}</div>
    ${budget.groups.length === 0 ? `<p class="muted">No budget groups yet.</p>` : ""}

    <section class="rules" aria-labelledby="rules-heading">
      <h3 id="rules-heading">Auto assign rules</h3>
      <p class="muted">Unassigned transactions whose description contains the text are assigned to the budget account, then wait for your approval.</p>
      <table class="rules-table">
        <tbody>
          ${rules
            .map(
              (r) => `<tr data-rule="${r.id}"><td class="pattern">${esc(r.pattern)}</td><td>${esc(r.groupName)}: ${esc(r.budgetAccountName)}</td>
                <td><button type="button" class="delete-rule secondary" aria-label="Delete rule ${esc(r.pattern)}">Delete</button></td></tr>`,
            )
            .join("")}
        </tbody>
      </table>
      ${rules.length === 0 ? `<p class="muted">No rules yet.</p>` : ""}
      <form class="inline" id="add-rule">
        <label>Text to match <input name="pattern" autocomplete="off" required></label>
        <label>Rule budget account <select name="budgetAccountId" required>
          ${budget.groups.flatMap((g) => g.accounts.map((a) => `<option value="${a.id}">${esc(g.name)}: ${esc(a.name)}</option>`)).join("")}
        </select></label>
        <button type="submit">Add Rule</button>
      </form>
    </section>

    <div class="forms">
      <form class="inline" id="add-group">
        <h3>Add group</h3>
        <label>Group name <input name="name" autocomplete="off" required></label>
        <button type="submit">Add Group</button>
      </form>
      <form class="inline" id="add-budget-account">
        <h3>Add budget account</h3>
        <label>Budget account name <input name="name" autocomplete="off" required></label>
        <label>Group <select name="groupId" required>
          ${budget.groups.map((g) => `<option value="${g.id}">${esc(g.name)}</option>`).join("")}
        </select></label>
        <label>Type <select name="kind"><option value="expense">Expense</option><option value="income">Income</option></select></label>
        <label>Auto monthly <select name="cadence">
          <option value="">Off</option>
          ${cadences.map((c) => `<option value="${c}">${cadenceLabel[c]}</option>`).join("")}
        </select></label>
        <label>Auto amount <input name="amount" inputmode="decimal" placeholder="0.00"></label>
        <button type="submit">Add Budget Account</button>
      </form>
    </div>`;

  const status = root.querySelector<HTMLElement>("[role=status]")!;
  const run = async (action: () => Promise<unknown>) => {
    try {
      await action();
      await refresh();
    } catch (err) {
      status.textContent = (err as Error).message;
    }
  };

  root.querySelector<HTMLInputElement>("input[type=month]")!.addEventListener("change", (e) => {
    history.pushState(null, "", `/budgets?month=${(e.target as HTMLInputElement).value}`);
    dispatchEvent(new PopStateEvent("popstate"));
  });

  root.querySelector<HTMLFormElement>("#add-group")!.addEventListener("submit", (e) => {
    e.preventDefault();
    const form = e.target as HTMLFormElement;
    void run(() => send("CreateBudgetGroup", { name: new FormData(form).get("name") }));
  });

  root.querySelector<HTMLFormElement>("#add-budget-account")!.addEventListener("submit", (e) => {
    e.preventDefault();
    const data = new FormData(e.target as HTMLFormElement);
    const cadence = String(data.get("cadence") ?? "");
    void run(() =>
      send("CreateBudgetAccount", {
        name: data.get("name"),
        groupId: data.get("groupId"),
        kind: data.get("kind"),
        auto: cadence ? { cadence, amountCents: toCents(String(data.get("amount") ?? "")) } : undefined,
      }),
    );
  });

  root.querySelector<HTMLFormElement>("#add-rule")!.addEventListener("submit", async (e) => {
    e.preventDefault();
    const data = new FormData(e.target as HTMLFormElement);
    try {
      const { autoAssigned } = await send<{ autoAssigned: number }>("CreateAutoAssignRule", {
        pattern: data.get("pattern"),
        budgetAccountId: data.get("budgetAccountId"),
      });
      await refresh();
      root.querySelector<HTMLElement>("[role=status]")!.textContent =
        `Rule added. ${autoAssigned} transactions assigned, waiting for approval.`;
    } catch (err) {
      status.textContent = (err as Error).message;
    }
  });
  for (const button of root.querySelectorAll<HTMLButtonElement>("button.delete-rule")) {
    button.addEventListener("click", () =>
      void run(() => send("DeleteAutoAssignRule", { ruleId: button.closest<HTMLElement>("[data-rule]")!.dataset["rule"] })),
    );
  }

  // Edit this month's budgeted amount.
  for (const input of root.querySelectorAll<HTMLInputElement>("input.budgeted")) {
    input.addEventListener("change", () =>
      void run(() => send("SetBudgetAmount", { budgetAccountId: input.dataset["id"], month, amountCents: toCents(input.value) })),
    );
  }

  // Drag and drop budget accounts between groups.
  for (const item of root.querySelectorAll<HTMLElement>("[data-budget-account]")) {
    item.addEventListener("dragstart", (e) => {
      e.dataTransfer!.setData("text/plain", item.dataset["budgetAccount"]!);
      e.dataTransfer!.effectAllowed = "move";
    });
  }
  for (const group of root.querySelectorAll<HTMLElement>("[data-group]")) {
    group.addEventListener("dragover", (e) => {
      e.preventDefault();
      group.classList.add("drop-target");
    });
    group.addEventListener("dragleave", () => group.classList.remove("drop-target"));
    group.addEventListener("drop", (e) => {
      e.preventDefault();
      group.classList.remove("drop-target");
      const budgetAccountId = e.dataTransfer!.getData("text/plain");
      if (budgetAccountId) void run(() => send("MoveBudgetAccount", { budgetAccountId, groupId: group.dataset["group"] }));
    });
  }
}

function groupHtml(g: BudgetView["groups"][number]): string {
  return `
    <section class="group card" data-group="${g.id}" aria-label="${esc(g.name)}">
      <h3>${esc(g.name)}</h3>
      <ul class="plain budget-accounts">
        ${g.accounts.map(accountHtml).join("")}
      </ul>
      ${g.accounts.length === 0 ? `<p class="muted drop-hint">Drop budget accounts here</p>` : ""}
    </section>`;
}

function accountHtml(a: BudgetAccountView): string {
  const auto = a.auto ? `<span class="muted">auto ${cadenceLabel[a.auto.cadence]!.toLowerCase()} ${formatCents(a.auto.amountCents)}</span>` : "";
  const budgeted =
    a.kind === "income"
      ? `<span class="muted">Income</span>`
      : `<input class="budgeted" data-id="${a.id}" aria-label="Budgeted for ${esc(a.name)}" inputmode="decimal" value="${(a.budgetedCents / 100).toFixed(2)}">`;
  return `
    <li draggable="true" data-budget-account="${a.id}">
      <span class="name">${esc(a.name)}</span> ${auto}
      <span class="spacer"></span>
      ${budgeted}
      <span class="amount activity" title="Assigned this month">${formatCents(a.activityCents)}</span>
    </li>`;
}
