"use client";

import { Eye, Globe, ShieldCheck, Code } from "lucide-react";
import { ScrollAnimate } from "./scroll-animate";

const values = [
  {
    icon: Eye,
    title: "Transparent by Default",
    description:
      "Every decision, every result, every algorithm is open for inspection. No black boxes, no hidden processes.",
  },
  {
    icon: Globe,
    title: "Compute for Everyone",
    description:
      "Anyone with a machine can contribute. Discoveries are credited to operators, not institutions.",
  },
  {
    icon: ShieldCheck,
    title: "Verifiable Results",
    description:
      "Deterministic proofs and primality certificates ensure every discovery is independently verifiable.",
  },
  {
    icon: Code,
    title: "Open Source Always",
    description:
      "MIT licensed from day one. The tools, algorithms, and infrastructure belong to everyone.",
  },
];

export function ValuesGrid() {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
      {values.map((value, i) => (
        <ScrollAnimate key={value.title} delay={i * 100}>
          <div className="flex gap-4">
            <div className="w-10 h-10 rounded-lg bg-accent-purple/10 flex items-center justify-center text-accent-purple shrink-0">
              <value.icon size={20} />
            </div>
            <div>
              <h3 className="text-foreground font-semibold mb-1">{value.title}</h3>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {value.description}
              </p>
            </div>
          </div>
        </ScrollAnimate>
      ))}
    </div>
  );
}
