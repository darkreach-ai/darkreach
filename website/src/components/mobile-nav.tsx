"use client";

import { useEffect } from "react";
import { X, Github } from "lucide-react";
import Link from "next/link";
import { DarkReachLogo } from "./darkreach-logo";

const primaryLinks = [
  { label: "Platform", href: "/platform" },
  { label: "Research", href: "/research" },
  { label: "Operators", href: "/operators" },
  { label: "About", href: "/about" },
  { label: "Blog", href: "/blog" },
];

const secondaryLinks = [
  { label: "Status", href: "/status" },
  { label: "Leaderboard", href: "/leaderboard" },
  { label: "Docs", href: "/docs/getting-started" },
];

interface MobileNavProps {
  open: boolean;
  onClose: () => void;
}

export function MobileNav({ open, onClose }: MobileNavProps) {
  useEffect(() => {
    if (open) {
      document.body.style.overflow = "hidden";
    } else {
      document.body.style.overflow = "";
    }
    return () => {
      document.body.style.overflow = "";
    };
  }, [open]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 md:hidden">
      <div className="absolute inset-0 bg-black/60" onClick={onClose} />

      <div className="absolute inset-y-0 right-0 w-full max-w-sm bg-background border-l border-border overflow-y-auto">
        <div className="flex items-center justify-between px-6 h-16 border-b border-border">
          <div className="flex items-center gap-2">
            <DarkReachLogo size={24} />
            <span className="text-foreground font-semibold">darkreach</span>
          </div>
          <button
            onClick={onClose}
            className="text-muted-foreground hover:text-foreground transition-colors"
            aria-label="Close menu"
          >
            <X size={24} />
          </button>
        </div>

        <div className="px-6 py-6 space-y-6">
          {/* Primary links */}
          <div className="space-y-1">
            {primaryLinks.map((link) => (
              <Link
                key={link.href}
                href={link.href}
                onClick={onClose}
                className="block py-2.5 text-base font-medium text-foreground hover:text-accent-purple transition-colors"
              >
                {link.label}
              </Link>
            ))}
          </div>

          {/* Secondary links */}
          <div className="pt-4 border-t border-border space-y-1">
            {secondaryLinks.map((link) => (
              <Link
                key={link.href}
                href={link.href}
                onClick={onClose}
                className="block py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
              >
                {link.label}
              </Link>
            ))}
          </div>

          {/* Social links */}
          <div className="pt-4 border-t border-border space-y-3">
            <a
              href="https://discord.gg/2Khf4t8M33"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-2 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/></svg>
              Discord
            </a>
            <a
              href="https://github.com/darkreach-ai/darkreach"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-2 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              <Github size={16} />
              GitHub
            </a>
            <a
              href="https://x.com/darkreach_ai"
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-2 py-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/></svg>
              X / Twitter
            </a>
          </div>

          {/* CTA */}
          <Link
            href="/#waitlist"
            onClick={onClose}
            className="block w-full text-center px-4 py-2.5 rounded-lg bg-accent-purple text-white text-sm font-medium hover:bg-accent-purple/90 transition-colors"
          >
            Join Waitlist
          </Link>
        </div>
      </div>
    </div>
  );
}
