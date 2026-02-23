export interface DocNavItem {
  title: string;
  href: string;
}

export interface DocNavSection {
  title: string;
  items: DocNavItem[];
}

export const docsNav: DocNavSection[] = [
  {
    title: "Getting Started",
    items: [
      { title: "Overview", href: "/docs" },
      { title: "Quick Start", href: "/docs/getting-started" },
      { title: "Architecture", href: "/docs/architecture" },
    ],
  },
  {
    title: "Platform",
    items: [
      { title: "AI Engine", href: "/docs/ai-engine" },
      { title: "Network & Operators", href: "/docs/network" },
      { title: "Projects & Campaigns", href: "/docs/projects" },
      { title: "API Reference", href: "/docs/api" },
    ],
  },
  {
    title: "Research",
    items: [
      { title: "Prime Forms", href: "/docs/prime-forms" },
      { title: "Verification", href: "/docs/verification" },
    ],
  },
  {
    title: "Community",
    items: [
      { title: "Contributing", href: "/docs/contributing" },
    ],
  },
];
