import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

type TagCategory = "structural" | "proof" | "property" | "verification" | "record";

const categoryStyles: Record<TagCategory, string> = {
  structural: "bg-slate-500/15 text-slate-700 dark:text-slate-300 border-slate-500/25",
  proof: "bg-yellow-500/15 text-yellow-700 dark:text-yellow-300 border-yellow-500/25",
  property: "bg-purple-500/15 text-purple-700 dark:text-purple-300 border-purple-500/25",
  verification: "bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 border-emerald-500/25",
  record: "bg-amber-500/15 text-amber-700 dark:text-amber-300 border-amber-500/25",
};

const proofGreenStyle = "bg-green-500/15 text-green-700 dark:text-green-300 border-green-500/25";
const proofOrangeStyle = "bg-orange-500/15 text-orange-700 dark:text-orange-300 border-orange-500/25";

const structuralForms = new Set([
  "factorial", "kbn", "palindromic", "near_repdigit", "primorial",
  "cullen_woodall", "wagstaff", "carol_kynea", "twin", "sophie_germain",
  "repunit", "gen_fermat",
]);

function categorize(tag: string): TagCategory {
  if (structuralForms.has(tag)) return "structural";
  if (tag === "deterministic" || tag === "probabilistic" || tag === "prp-only") return "proof";
  if (tag.startsWith("verified-") || tag.startsWith("verified_")) return "verification";
  if (tag === "world-record" || tag === "project-record") return "record";
  if (["safe-prime", "sophie-germain", "palindromic", "twin-prime"].includes(tag)) return "property";
  return "structural";
}

function getStyle(tag: string): string {
  const category = categorize(tag);
  if (category === "proof") {
    if (tag === "deterministic") return proofGreenStyle;
    if (tag === "prp-only") return proofOrangeStyle;
    return categoryStyles.proof;
  }
  return categoryStyles[category];
}

interface TagChipProps {
  tag: string;
  className?: string;
  onClick?: () => void;
}

export function TagChip({ tag, className, onClick }: TagChipProps) {
  return (
    <Badge
      variant="outline"
      className={cn(
        "text-[10px] px-1.5 py-0 h-[18px] font-normal",
        getStyle(tag),
        onClick && "cursor-pointer hover:opacity-80",
        className,
      )}
      onClick={onClick}
    >
      {tag}
    </Badge>
  );
}

/** Color-coded dot for use in charts matching tag category. */
export function tagCategoryColor(tag: string): string {
  const category = categorize(tag);
  switch (category) {
    case "structural": return "#64748b";
    case "proof": return "#eab308";
    case "property": return "#a855f7";
    case "verification": return "#10b981";
    case "record": return "#f59e0b";
    default: return "#64748b";
  }
}
