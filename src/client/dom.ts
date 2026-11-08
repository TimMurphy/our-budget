export const esc = (s: unknown) => String(s).replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

export { formatCents } from "../core/money.ts";

export const currentMonth = () => {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
};

/** Shows an error message in the page instead of leaving it blank. */
export function showError(root: HTMLElement, err: unknown): void {
  root.innerHTML = `<p role="alert">${esc((err as Error).message)}</p>`;
}

export function navigate(url: string): void {
  history.pushState(null, "", url);
  dispatchEvent(new PopStateEvent("popstate"));
}

/** ISO date (YYYY-MM-DD) in the browser's locale. Parsed as a local date so the day never shifts. */
export function formatDate(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number) as [number, number, number];
  return new Date(y, m - 1, d).toLocaleDateString();
}
