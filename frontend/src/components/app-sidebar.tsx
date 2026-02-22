"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  BarChart3,
  BookOpen,
  Bot,
  ClipboardList,
  FileText,
  FolderKanban,
  Globe,
  LayoutDashboard,
  LogOut,
  Rocket,
  Search,
  ShieldCheck,
  Table,
  Trophy,
  User,
  Zap,
  MessageCircle,
  ExternalLink,
} from "lucide-react";
import { useWs } from "@/contexts/websocket-context";
import { useAuth } from "@/contexts/auth-context";
import { cn } from "@/lib/utils";
import { DarkReachLogo } from "@/components/darkreach-logo";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  useSidebar,
} from "@/components/ui/sidebar";

type NavItem = {
  title: string;
  href: string;
  icon: React.ComponentType<{ className?: string }>;
  badge?: number;
};

type NavSection = {
  title: string;
  items: NavItem[];
};

function NavBadge({ count, pulse }: { count: number; pulse?: boolean }) {
  return (
    <span
      className={cn(
        "ml-auto flex h-5 min-w-5 items-center justify-center rounded-full px-1.5",
        "bg-indigo-500/15 text-indigo-300 text-[11px] font-semibold tabular-nums",
        "ring-1 ring-indigo-500/20",
        "group-data-[collapsible=icon]:hidden",
        pulse && "animate-pulse"
      )}
    >
      {count}
    </span>
  );
}

export function AppSidebar() {
  const pathname = usePathname();
  const { searches, agentTasks } = useWs();
  const { user, role, signOut } = useAuth();
  const { state } = useSidebar();
  const collapsed = state === "collapsed";

  const runningCount = searches.filter((s) => s.status === "running").length;
  const activeAgentCount = agentTasks.filter(
    (t) => t.status === "in_progress"
  ).length;

  const isAdmin = role === "admin";

  const initials = user?.email
    ? user.email
        .split("@")[0]
        .split(/[._-]/)
        .slice(0, 2)
        .map((s) => s[0]?.toUpperCase() ?? "")
        .join("")
    : "?";

  function isActive(href: string) {
    return href === "/" ? pathname === "/" : pathname.startsWith(href);
  }

  const operatorItems: NavItem[] = [
    { title: "Dashboard", href: "/", icon: LayoutDashboard },
    { title: "Browse", href: "/browse", icon: Table },
    { title: "My Nodes", href: "/my-nodes", icon: Globe },
    { title: "Leaderboard", href: "/leaderboard", icon: Trophy },
    { title: "Account", href: "/account", icon: User },
  ];

  const adminSections: NavSection[] = [
    {
      title: "Discovery",
      items: [
        {
          title: "Searches",
          href: "/searches",
          icon: Search,
          badge: runningCount || undefined,
        },
        { title: "Verification", href: "/verification", icon: ShieldCheck },
        { title: "Projects", href: "/projects", icon: FolderKanban },
      ],
    },
    {
      title: "Operations",
      items: [
        { title: "Network", href: "/network", icon: Globe },
        { title: "Strategy", href: "/strategy", icon: Zap },
        {
          title: "Agents",
          href: "/agents",
          icon: Bot,
          badge: activeAgentCount || undefined,
        },
      ],
    },
    {
      title: "System",
      items: [
        { title: "Observability", href: "/performance", icon: BarChart3 },
        { title: "Logs", href: "/logs", icon: FileText },
        { title: "Releases", href: "/releases", icon: Rocket },
        { title: "Audit Log", href: "/audit", icon: ClipboardList },
      ],
    },
  ];

  function renderNavItem(item: NavItem) {
    const active = isActive(item.href);
    return (
      <SidebarMenuItem key={item.href}>
        <SidebarMenuButton
          asChild
          isActive={active}
          tooltip={item.title}
          className={cn(
            "group/nav relative transition-all duration-200",
            active && [
              "bg-gradient-to-r from-indigo-500/[0.12] via-indigo-500/[0.06] to-transparent",
              "text-indigo-50",
              "hover:from-indigo-500/[0.18] hover:via-indigo-500/[0.08]",
            ]
          )}
        >
          <Link href={item.href}>
            {/* Active indicator bar */}
            {active && (
              <span className="absolute left-0 top-1.5 bottom-1.5 w-[3px] rounded-full bg-indigo-500 shadow-[0_0_8px_rgba(99,102,241,0.5)] group-data-[collapsible=icon]:left-0 group-data-[collapsible=icon]:top-2 group-data-[collapsible=icon]:bottom-2" />
            )}
            <item.icon
              className={cn(
                "transition-colors duration-200",
                active
                  ? "text-indigo-400"
                  : "text-zinc-500 group-hover/nav:text-zinc-300"
              )}
            />
            <span
              className={cn(
                "transition-colors duration-200",
                active
                  ? "font-medium text-indigo-50"
                  : "text-zinc-400 group-hover/nav:text-zinc-200"
              )}
            >
              {item.title}
            </span>
            {item.badge != null && (
              <NavBadge
                count={item.badge}
                pulse={item.title === "Searches"}
              />
            )}
          </Link>
        </SidebarMenuButton>
      </SidebarMenuItem>
    );
  }

  return (
    <Sidebar side="left" collapsible="icon">
      {/* ── Header ── */}
      <SidebarHeader className="h-12 justify-center border-b border-white/[0.06]">
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="lg"
              asChild
              tooltip="darkreach"
              className="[&>svg]:size-auto hover:bg-transparent"
            >
              <Link href="/" className="gap-2.5">
                <div
                  className={cn(
                    "relative flex items-center justify-center shrink-0",
                    !collapsed && "drop-shadow-[0_0_10px_rgba(99,102,241,0.3)]"
                  )}
                >
                  <DarkReachLogo size={28} className="text-indigo-400" />
                </div>
                <span className="text-[15px] font-semibold tracking-tight bg-gradient-to-r from-white to-zinc-400 bg-clip-text text-transparent">
                  darkreach
                </span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      {/* ── Content ── */}
      <SidebarContent className="px-1.5">
        {/* Operator section */}
        <SidebarGroup className="pt-3 pb-1">
          <SidebarGroupContent>
            <SidebarMenu className="gap-0.5">
              {operatorItems.map(renderNavItem)}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        {/* Admin sections */}
        {isAdmin &&
          adminSections.map((section) => (
            <SidebarGroup key={section.title} className="pt-1 pb-1">
              <SidebarGroupLabel className="px-3 mb-1 text-[10px] font-semibold uppercase tracking-[0.08em] text-zinc-600">
                {section.title}
              </SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu className="gap-0.5">
                  {section.items.map(renderNavItem)}
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          ))}
      </SidebarContent>

      {/* ── Footer ── */}
      <SidebarFooter className="border-t border-white/[0.06] pt-2">
        {/* External links */}
        <SidebarMenu className="gap-0.5">
          <SidebarMenuItem>
            <SidebarMenuButton
              asChild
              tooltip="Docs"
              className="group/nav text-zinc-500 hover:text-zinc-300 transition-colors duration-200"
            >
              <Link href="/docs" className="gap-2">
                <BookOpen className="text-zinc-600 group-hover/nav:text-zinc-400 transition-colors duration-200" />
                <span>Docs</span>
                <ExternalLink className="ml-auto size-3 opacity-0 group-hover/nav:opacity-40 transition-opacity duration-200 group-data-[collapsible=icon]:hidden" />
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <SidebarMenuButton
              asChild
              tooltip="Discord"
              className="group/nav text-zinc-500 hover:text-zinc-300 transition-colors duration-200"
            >
              <a
                href="https://discord.gg/2Khf4t8M33"
                target="_blank"
                rel="noopener noreferrer"
                className="gap-2"
              >
                <MessageCircle className="text-zinc-600 group-hover/nav:text-zinc-400 transition-colors duration-200" />
                <span>Discord</span>
                <ExternalLink className="ml-auto size-3 opacity-0 group-hover/nav:opacity-40 transition-opacity duration-200 group-data-[collapsible=icon]:hidden" />
              </a>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>

        {/* User section */}
        {user && (
          <>
            <div className="mx-2 my-1 h-px bg-gradient-to-r from-white/[0.06] via-white/[0.04] to-transparent group-data-[collapsible=icon]:hidden" />
            <SidebarMenu className="gap-0.5">
              <SidebarMenuItem>
                <SidebarMenuButton
                  size="lg"
                  tooltip={user.email?.split("@")[0] ?? "User"}
                  className="cursor-default hover:bg-white/[0.03] transition-colors duration-200"
                >
                  {/* Avatar */}
                  <div className="relative flex size-8 items-center justify-center rounded-lg bg-gradient-to-br from-indigo-500 to-violet-600 text-[11px] font-bold text-white shrink-0 shadow-[0_0_12px_rgba(99,102,241,0.25)]">
                    {initials}
                  </div>
                  <div className="flex flex-col min-w-0 gap-0.5">
                    <div className="flex items-center gap-1.5">
                      <span className="text-sm font-medium text-zinc-200 truncate">
                        {user.email?.split("@")[0] ?? "User"}
                      </span>
                      {role && (
                        <span
                          className={cn(
                            "px-1.5 py-0.5 rounded text-[9px] font-bold uppercase tracking-wider leading-none shrink-0",
                            role === "admin"
                              ? "bg-indigo-500/15 text-indigo-400 ring-1 ring-indigo-500/20"
                              : "bg-emerald-500/15 text-emerald-400 ring-1 ring-emerald-500/20"
                          )}
                        >
                          {role}
                        </span>
                      )}
                    </div>
                    <span className="text-[11px] text-zinc-600 truncate">
                      {user.email}
                    </span>
                  </div>
                </SidebarMenuButton>
              </SidebarMenuItem>
              <SidebarMenuItem>
                <SidebarMenuButton
                  onClick={() => signOut()}
                  tooltip="Sign out"
                  className="group/nav text-zinc-500 hover:text-red-400 transition-colors duration-200"
                >
                  <LogOut className="text-zinc-600 group-hover/nav:text-red-400/70 transition-colors duration-200" />
                  <span>Sign out</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </>
        )}
      </SidebarFooter>

      <SidebarRail />
    </Sidebar>
  );
}
