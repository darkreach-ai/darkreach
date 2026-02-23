export interface PrimeEntry {
  id: number;
  form: string;
  expression: string;
  digits: number;
  discovered_at: string;
}

export const FALLBACK_ENTRIES: PrimeEntry[] = [
  { id: 1, form: "kbn", expression: "3 \u00b7 2^59973 + 1", digits: 18055, discovered_at: "2026-02-18T14:22:00Z" },
  { id: 2, form: "palindromic", expression: "10^502 + R(501)^rev + 1", digits: 503, discovered_at: "2026-02-18T13:05:00Z" },
  { id: 3, form: "kbn", expression: "3 \u00b7 2^59941 - 1", digits: 18046, discovered_at: "2026-02-18T11:30:00Z" },
];

export function timeAgo(iso: string): string {
  const secs = Math.floor((Date.now() - new Date(iso).getTime()) / 1000);
  if (secs < 60) return "just now";
  if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
  if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
  return `${Math.floor(secs / 86400)}d ago`;
}

export function formLabel(form: string): string {
  const labels: Record<string, string> = {
    kbn: "k\u00b7b^n\u00b11",
    palindromic: "Palindromic",
    factorial: "Factorial",
    primorial: "Primorial",
    twin: "Twin",
    sophie_germain: "Sophie Germain",
    cullen_woodall: "Cullen/Woodall",
    carol_kynea: "Carol/Kynea",
    gen_fermat: "Gen. Fermat",
    repunit: "Repunit",
    wagstaff: "Wagstaff",
    near_repdigit: "Near-Repdigit",
  };
  return labels[form] ?? form;
}

export function formColor(form: string): string {
  const colors: Record<string, string> = {
    kbn: "bg-indigo-500/10 text-indigo-400 border-indigo-500/20",
    palindromic: "bg-emerald-500/10 text-emerald-400 border-emerald-500/20",
    factorial: "bg-amber-500/10 text-amber-400 border-amber-500/20",
    primorial: "bg-violet-500/10 text-violet-400 border-violet-500/20",
    twin: "bg-cyan-500/10 text-cyan-400 border-cyan-500/20",
    sophie_germain: "bg-rose-500/10 text-rose-400 border-rose-500/20",
    cullen_woodall: "bg-orange-500/10 text-orange-400 border-orange-500/20",
    carol_kynea: "bg-pink-500/10 text-pink-400 border-pink-500/20",
    gen_fermat: "bg-teal-500/10 text-teal-400 border-teal-500/20",
    repunit: "bg-sky-500/10 text-sky-400 border-sky-500/20",
    wagstaff: "bg-purple-500/10 text-purple-400 border-purple-500/20",
    near_repdigit: "bg-lime-500/10 text-lime-400 border-lime-500/20",
  };
  return colors[form] ?? "bg-accent-purple/10 text-accent-purple border-accent-purple/20";
}
