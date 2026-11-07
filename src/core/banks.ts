/** Known banks. Each reads its own downloaded statement format (CSV). */
import { parseCsvRecords } from "./csv.ts";
import { toCents } from "./money.ts";

export interface StatementLine {
  /** ISO date, YYYY-MM-DD */
  date: string;
  description: string;
  /** Negative = money out of the account. */
  amountCents: number;
  /** Account balance after this line, when the statement says. */
  balanceCents: number | null;
  /** The bank's own category, if any. */
  category: string | null;
}

export interface Bank {
  /** Short, stable code stored on events, e.g. "nab". */
  readonly code: string;
  readonly name: string;
  /** Returns lines oldest first. Lines on the same day keep the order they happened in. */
  parseStatement(csv: string): StatementLine[];
}

const MONTHS = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];
const iso = (y: number, m: number, d: number) =>
  `${String(y).padStart(4, "0")}-${String(m).padStart(2, "0")}-${String(d).padStart(2, "0")}`;

/** "04 Nov 26" -> 2026-11-04 */
function parseNabDate(text: string): string {
  const m = /^(\d{1,2}) ([A-Za-z]{3}) (\d{2}|\d{4})$/.exec(text.trim());
  const month = m ? MONTHS.indexOf(m[2]!.toLowerCase()) + 1 : 0;
  if (!m || !month) throw new Error(`Unrecognised NAB date '${text}'`);
  const year = m[3]!.length === 2 ? 2000 + Number(m[3]) : Number(m[3]);
  return iso(year, month, Number(m[1]));
}

/** "06/11/2026" -> 2026-11-06 */
function parseWestpacDate(text: string): string {
  const m = /^(\d{1,2})\/(\d{1,2})\/(\d{4})$/.exec(text.trim());
  if (!m) throw new Error(`Unrecognised Westpac date '${text}'`);
  return iso(Number(m[3]), Number(m[2]), Number(m[1]));
}

const optionalCents = (text: string | undefined) => (text ? toCents(text) : null);

export const nab: Bank = {
  code: "nab",
  name: "National Australia Bank",
  // Columns: Date, Amount (signed), Transaction Details, Balance, Category. Newest first.
  parseStatement: (csv) =>
    parseCsvRecords(csv)
      .map((r) => ({
        date: parseNabDate(r["Date"] ?? ""),
        description: r["Transaction Details"] ?? "",
        amountCents: toCents(r["Amount"] ?? ""),
        balanceCents: optionalCents(r["Balance"]),
        category: r["Category"] || null,
      }))
      .reverse(),
};

export const westpac: Bank = {
  code: "wbc",
  name: "Westpac Banking Corporation",
  // Columns: Date, Narrative, Debit Amount, Credit Amount, Balance, Categories. Newest first.
  parseStatement: (csv) =>
    parseCsvRecords(csv)
      .map((r) => ({
        date: parseWestpacDate(r["Date"] ?? ""),
        description: r["Narrative"] ?? "",
        amountCents: (optionalCents(r["Credit Amount"]) ?? 0) - (optionalCents(r["Debit Amount"]) ?? 0),
        balanceCents: optionalCents(r["Balance"]),
        category: r["Categories"] || null,
      }))
      .reverse(),
};

export const knownBanks: readonly Bank[] = [nab, westpac];

export const findBank = (code: string): Bank | undefined => knownBanks.find((b) => b.code === code);
