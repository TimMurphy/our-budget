import "./styles.css";
import { showError } from "./dom.ts";
import { renderBankAccounts } from "./pages/bank-accounts.ts";
import { renderBankAccount } from "./pages/bank-account.ts";
import { renderBudgets } from "./pages/budgets.ts";
import { renderHome } from "./pages/home.ts";

const app = document.getElementById("app")!;
app.innerHTML = `
  <header class="top-bar">
    <a class="brand" href="/">Our Budget</a>
    <nav class="navbar">
      <a href="/bank-accounts">Bank Accounts</a>
      <a href="/budgets">Budgets</a>
    </nav>
  </header>
  <main></main>`;
const main = app.querySelector("main")!;
const navLinks = [...app.querySelectorAll<HTMLAnchorElement>(".navbar a")];

type Page = (root: HTMLElement, url: URL, params: string[]) => void | Promise<void>;
const routes: [RegExp, Page][] = [
  [/^\/$/, renderHome],
  [/^\/bank-accounts$/, renderBankAccounts],
  [/^\/bank-accounts\/([^/]+)$/, renderBankAccount],
  [/^\/budgets$/, renderBudgets],
];

async function show(): Promise<void> {
  const url = new URL(location.href);
  for (const link of navLinks) {
    const section = link.pathname;
    link.toggleAttribute("aria-current", url.pathname === section || url.pathname.startsWith(section + "/"));
  }
  for (const [pattern, page] of routes) {
    const match = pattern.exec(url.pathname);
    if (!match) continue;
    main.replaceChildren();
    try {
      await page(main, url, match.slice(1).map(decodeURIComponent));
    } catch (err) {
      showError(main, err);
    }
    return;
  }
  main.innerHTML = `<h2>Page not found</h2>`;
}

// Client-side navigation for in-app links.
document.addEventListener("click", (e) => {
  const link = (e.target as Element).closest("a");
  if (!link || link.origin !== location.origin || e.metaKey || e.ctrlKey || link.target) return;
  e.preventDefault();
  history.pushState(null, "", link.pathname + link.search);
  void show();
});
addEventListener("popstate", () => void show());

void show();
