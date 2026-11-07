/** Money is integer cents everywhere. */

/** "1,234.56" or "-20" -> cents. Throws on anything else. */
export function toCents(text: string): number {
  const t = text.trim().replace(/[,$]/g, "");
  if (!/^-?\d+(\.\d{1,2})?$/.test(t)) throw new Error(`'${text}' is not an amount like 1234.56`);
  return Math.round(parseFloat(t) * 100);
}

export const formatCents = (cents: number): string =>
  (cents < 0 ? "-" : "") + "$" + (Math.abs(cents) / 100).toLocaleString("en-AU", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
