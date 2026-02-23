"use client";

import { useEffect, useState } from "react";
import { usePathname } from "next/navigation";
import { DarkReachLogo } from "./darkreach-logo";
import { MobileNav } from "./mobile-nav";
import { Menu } from "lucide-react";
import Link from "next/link";

const navLinks = [
  { label: "Platform", path: "/platform" },
  { label: "Research", path: "/research" },
  { label: "Operators", path: "/operators" },
  { label: "About", path: "/about" },
  { label: "Blog", path: "/blog" },
];

export function Navbar() {
  const [scrolled, setScrolled] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);
  const pathname = usePathname();

  useEffect(() => {
    const handleScroll = () => setScrolled(window.scrollY > 20);
    window.addEventListener("scroll", handleScroll, { passive: true });
    return () => window.removeEventListener("scroll", handleScroll);
  }, []);

  const isActive = (path: string) =>
    pathname === path || pathname.startsWith(path + "/");

  return (
    <>
      <nav
        className={`fixed top-0 left-0 right-0 z-50 transition-all duration-300 ${
          scrolled
            ? "bg-background/95 backdrop-blur-md border-b border-border shadow-[0_1px_12px_rgba(99,102,241,0.06)]"
            : "bg-transparent"
        }`}
      >
        <div className="mx-auto max-w-7xl flex items-center justify-between px-6 sm:px-8 lg:px-12 h-16">
          <Link href="/" className="flex items-center gap-2">
            <DarkReachLogo size={28} />
            <span className="text-foreground font-semibold text-lg">darkreach</span>
          </Link>

          <div className="hidden md:flex items-center gap-8">
            {navLinks.map((link) => (
              <Link
                key={link.path}
                href={link.path}
                className={`relative text-sm pb-0.5 transition-colors ${
                  isActive(link.path)
                    ? "text-foreground"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                {link.label}
                {isActive(link.path) && (
                  <span className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-1 h-1 bg-accent-purple rounded-full" />
                )}
              </Link>
            ))}
          </div>

          <div className="flex items-center gap-4">
            <Link
              href="/#waitlist"
              className="hidden sm:inline-flex items-center px-5 py-1.5 rounded-full bg-accent-purple text-white text-sm font-medium hover:bg-accent-purple/90 transition-colors"
            >
              Join Waitlist
            </Link>
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
