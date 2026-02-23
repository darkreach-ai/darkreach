import { DarkReachLogo } from "./darkreach-logo";
import Link from "next/link";

const columns = [
  {
    title: "Platform",
    links: [
      { label: "How It Works", href: "/platform#how-it-works" },
      { label: "Technology", href: "/platform#technology" },
      { label: "Status", href: "/status" },
      { label: "Docs", href: "/docs/getting-started" },
    ],
  },
  {
    title: "Research",
    links: [
      { label: "Prime Discovery", href: "/research" },
      { label: "For Universities", href: "/research#universities" },
      { label: "Initiatives", href: "/research#initiatives" },
    ],
  },
  {
    title: "Company",
    links: [
      { label: "About", href: "/about" },
      { label: "Blog", href: "/blog" },
      { label: "Careers", href: "/about#careers" },
      {
        label: "Open Source",
        href: "https://github.com/darkreach-ai/darkreach",
        external: true,
      },
    ],
  },
  {
    title: "Community",
    links: [
      {
        label: "GitHub",
        href: "https://github.com/darkreach-ai/darkreach",
        external: true,
      },
      { label: "Discord", href: "https://discord.gg/2Khf4t8M33", external: true },
      { label: "X / Twitter", href: "https://x.com/darkreach_ai", external: true },
      { label: "Leaderboard", href: "/leaderboard" },
    ],
  },
];

export function Footer() {
  return (
    <footer className="border-t border-border">
      <div className="mx-auto max-w-7xl px-6 sm:px-8 lg:px-12 py-16">
        <div className="grid grid-cols-2 md:grid-cols-6 gap-8">
          <div className="col-span-2">
            <div className="flex items-center gap-2 mb-4">
              <DarkReachLogo size={20} />
              <span className="text-foreground font-semibold">darkreach</span>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed max-w-xs">
              Building the world&apos;s biggest supercomputer.
            </p>
          </div>

          {columns.map((col) => (
            <div key={col.title}>
              <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">
                {col.title}
              </h3>
              <ul className="space-y-2">
                {col.links.map((link) => (
                  <li key={link.label}>
                    {"external" in link && link.external ? (
                      <a
                        href={link.href}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                      >
                        {link.label}
                      </a>
                    ) : (
                      <Link
                        href={link.href}
                        className="text-sm text-muted-foreground hover:text-foreground transition-colors"
                      >
                        {link.label}
                      </Link>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </div>

        <div className="mt-12 pt-8 border-t border-border flex flex-col sm:flex-row items-center justify-between gap-4">
          <span className="text-muted-foreground text-sm">
            &copy; {new Date().getFullYear()} darkreach.
          </span>
          <div className="flex items-center gap-6">
            <Link
              href="/privacy"
              className="text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              Privacy
            </Link>
            <a
              href="https://github.com/darkreach-ai/darkreach/blob/master/LICENSE"
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              MIT License
            </a>
          </div>
        </div>
      </div>
    </footer>
  );
}
