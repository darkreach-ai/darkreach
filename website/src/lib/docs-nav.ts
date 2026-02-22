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
    title: "Overview",
    items: [
      { title: "Documentation", href: "/docs" },
      { title: "Getting Started", href: "/docs/getting-started" },
      { title: "Architecture", href: "/docs/architecture" },
    ],
  },
  {
    title: "Guides",
    items: [
      { title: "Prime Forms", href: "/docs/prime-forms" },
      { title: "AI Engine", href: "/docs/ai-engine" },
      { title: "Projects & Campaigns", href: "/docs/projects" },
      { title: "Network & Operators", href: "/docs/network" },
      { title: "Verification", href: "/docs/verification" },
    ],
  },
  {
    title: "Reference",
    items: [
      { title: "API Reference", href: "/docs/api" },
      { title: "Contributing", href: "/docs/contributing" },
    ],
  },
];
