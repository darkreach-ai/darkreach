"use client";

import { useEffect, useState } from "react";
import { usePathname } from "next/navigation";
import { DarkReachLogo } from "./darkreach-logo";
import { MobileNav } from "./mobile-nav";
import { ChevronDown, Github, Menu } from "lucide-react";
import Link from "next/link";

const docsLinks = [
  { label: "Getting Started", href: "/docs/getting-started" },
  { label: "Architecture", href: "/docs/architecture" },
  { label: "Prime Forms", href: "/docs/prime-forms" },
  { label: "AI Engine", href: "/docs/ai-engine" },
  { label: "Projects & Campaigns", href: "/docs/projects" },
  { label: "Network & Operators", href: "/docs/network" },
  { label: "Verification", href: "/docs/verification" },
  { label: "API Reference", href: "/docs/api" },
  { label: "Contributing", href: "/docs/contributing" },
];

const downloadLinks = [
  { label: "Download", href: "/download" },
  { label: "Coordinator Setup", href: "/download/server" },
  { label: "Worker Deployment", href: "/download/worker" },
];

function NavDropdown({
  label,
  links,
  active,
}: {
  label: string;
  links: { label: string; href: string }[];
  active: boolean;
}) {
  const [open, setOpen] = useState(false);

  return (
    <div
      className="relative"
      onMouseEnter={() => setOpen(true)}
      onMouseLeave={() => setOpen(false)}
    >
      <button
        className={`flex items-center gap-1 text-sm transition-colors ${
          active ? "text-foreground" : "text-muted-foreground hover:text-foreground"
        }`}
      >
        {label}
        <ChevronDown size={14} />
      </button>
      {active && (
        <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-orange" />
      )}
      {open && (
        <div className="absolute top-full left-0 pt-2 z-50">
          <div className="bg-card border border-border rounded-md py-1 min-w-[180px] shadow-lg">
            {links.map((link) => (
              <Link
                key={link.href}
                href={link.href}
                className="block px-4 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-background transition-colors"
              >
                {link.label}
              </Link>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export function Navbar() {
  const [scrolled, setScrolled] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const pathname = usePathname();

  useEffect(() => {
    const handleScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  const isActive = (path: string) => pathname === path;
  const isActivePrefix = (prefix: string) => pathname.startsWith(prefix);

  return (
    <>
      <nav
        className={`fixed top-0 left-0 right-0 z-50 transition-colors duration-200 ${
          scrolled
            ? "bg-background/95 backdrop-blur-sm border-b border-border"
            : "bg-transparent"
        }`}
      >
        <div className="mx-auto max-w-7xl flex items-center justify-between px-6 sm:px-8 lg:px-12 h-16">
          <Link href="/" className="flex items-center gap-2">
            <DarkReachLogo size={28} />
            <span className="text-foreground font-semibold text-lg">darkreach</span>
          </Link>

          <div className="hidden md:flex items-center gap-8">
            <Link
              href="/about"
              className={`relative text-sm pb-0.5 transition-colors ${
                isActive("/about")
                  ? "text-foreground"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              About
              {isActive("/about") && (
                <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-orange" />
              )}
            </Link>

            <NavDropdown
              label="Docs"
              links={docsLinks}
              active={isActivePrefix("/docs")}
            />

            <NavDropdown
              label="Download"
              links={downloadLinks}
              active={isActivePrefix("/download")}
            />

            <Link
              href="/blog"
              className={`relative text-sm pb-0.5 transition-colors ${
                isActive("/blog")
                  ? "text-foreground"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              Blog
              {isActive("/blog") && (
                <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-orange" />
              )}
            </Link>

            <Link
              href="/status"
              className={`relative text-sm pb-0.5 transition-colors ${
                isActive("/status")
                  ? "text-foreground"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              Status
              {isActive("/status") && (
                <span className="absolute bottom-0 left-0 right-0 h-0.5 bg-accent-orange" />
              )}
            </Link>
          </div>

          <div className="flex items-center gap-4">
            <a
              href="https://discord.gg/2Khf4t8M33"
              target="_blank"
              rel="noopener noreferrer"
              className="text-muted-foreground hover:text-foreground transition-colors"
              aria-label="Discord"
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/></svg>
            </a>
            <a
              href="https://github.com/darkreach-ai/darkreach"
              target="_blank"
              rel="noopener noreferrer"
              className="text-muted-foreground hover:text-foreground transition-colors"
              aria-label="GitHub"
            >
              <Github size={20} />
            </a>
            <a
              href="https://app.darkreach.ai"
              className="hidden sm:inline-flex items-center px-4 py-1.5 rounded-md bg-accent-purple text-white text-sm font-medium hover:opacity-90 transition-opacity"
            >
              Open Dashboard
            </a>
            <button
              className="md:hidden text-muted-foreground hover:text-foreground transition-colors"
              onClick={() => setMobileOpen(true)}
              aria-label="Open menu"
            >
              <Menu size={24} />
            </button>
          </div>
        </div>
      </nav>

      <MobileNav open={mobileOpen} onClose={() => setMobileOpen(false)} />
    </>
  );
}
