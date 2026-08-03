"use client";

import type { LucideIcon } from "lucide-react";
import {
  Activity,
  Compass,
  FileQuestion,
  GraduationCap,
  History,
  Info,
  LayoutDashboard,
  Lightbulb,
  LogOut,
  Pencil,
  Play,
  Settings,
  Shield,
  Users,
} from "lucide-react";
import { Sheet, SheetContent, SheetTitle } from "@/components/ui/sheet";
import { Logo } from "@/components/Logo";
import { useAuth } from "@/hooks/useAuth";
import { ThemeSelector } from "@/components/ThemeSelector";
import { APP_VERSION } from "@/version";

export const DRAWER_WIDTH = 232;

const ICON_MAP: Record<string, LucideIcon> = {
  explore: Compass,
  take: Play,
  dashboard: LayoutDashboard,
  author: Pencil,
  grade: GraduationCap,
  "deep-dives": Lightbulb,
  about: Info,
  "admin-dashboard": Shield,
  "admin-users": Users,
  "admin-audit": History,
  "admin-health": Activity,
  "admin-settings": Settings,
};

interface SidebarProps {
  route: string;
  setRoute: (route: string) => void;
  desktop?: boolean;
  mobileOpen?: boolean;
  onClose?: () => void;
}

interface NavigationItem {
  id: string;
  label: string;
  icon: string;
  section: string;
}

function stringToColor(value: string): string {
  let hash = 0;
  for (let index = 0; index < value.length; index += 1) {
    hash = value.charCodeAt(index) + ((hash << 5) - hash);
  }
  return `hsl(${Math.abs(hash) % 360}, 55%, 45%)`;
}

export function Sidebar({
  route,
  setRoute,
  desktop = true,
  mobileOpen = false,
  onClose,
}: SidebarProps) {
  const { user, logout: logoutContext } = useAuth();

  const adminItems: NavigationItem[] =
    user?.role === "admin"
      ? [
          {
            id: "admin-dashboard",
            label: "Admin Console",
            icon: "admin-dashboard",
            section: "Admin",
          },
          {
            id: "admin-users",
            label: "Manage Users",
            icon: "admin-users",
            section: "Admin",
          },
          {
            id: "admin-audit",
            label: "Audit Logs",
            icon: "admin-audit",
            section: "Admin",
          },
          {
            id: "admin-health",
            label: "System Health",
            icon: "admin-health",
            section: "Admin",
          },
          {
            id: "admin-settings",
            label: "Platform Settings",
            icon: "admin-settings",
            section: "Admin",
          },
        ]
      : [];

  const items: NavigationItem[] = [
    {
      id: "learning",
      label: "Learning desk",
      icon: "explore",
      section: "Learn",
    },
    ...adminItems,
  ];
  const sections = ["Learn", ...(user?.role === "admin" ? ["Admin"] : [])];
  const initials =
    user?.displayName
      ?.split(" ")
      .map((name) => name[0])
      .join("")
      .toUpperCase()
      .slice(0, 2) ?? "?";
  const displayName = user?.displayName ?? "";

  async function handleLogout() {
    await logoutContext();
  }

  const navigation = (
    <div className="flex h-full flex-col bg-background">
      <div className="border-b border-border px-5 pb-4 pt-5">
        <Logo />
        <div className="mt-1 flex items-baseline gap-2">
          <span className="text-xs tracking-[0.12em] text-muted-foreground">
            AME
          </span>
          <span className="font-mono text-[11px] text-muted-foreground/70">
            v{APP_VERSION}
          </span>
        </div>
      </div>

      <nav
        aria-label="Main navigation"
        className="min-h-0 flex-1 overflow-y-auto py-3"
      >
        {sections.map((section) => {
          const sectionItems = items.filter((item) => item.section === section);
          if (!sectionItems.length) return null;
          return (
            <div key={section} className="mb-5">
              <p className="px-4 py-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
                {section}
              </p>
              <div className="space-y-0.5 px-2">
                {sectionItems.map((item) => {
                  const Icon = ICON_MAP[item.icon] ?? FileQuestion;
                  const selected = route === item.id;
                  return (
                    <button
                      key={item.id}
                      type="button"
                      aria-current={selected ? "page" : undefined}
                      onClick={() => setRoute(item.id)}
                      className={`flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-[13px] outline-none transition focus-visible:ring-2 focus-visible:ring-ring ${selected ? "bg-accent font-medium text-accent-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground"}`}
                    >
                      <Icon className="size-4 shrink-0" />
                      <span>{item.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>
          );
        })}
        <div className="mb-5">
          <div className="space-y-0.5 px-2">
            <a
              href="/about"
              className="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left text-[13px] text-muted-foreground outline-none transition hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
            >
              <Info className="size-4 shrink-0" />
              <span>About AME</span>
            </a>
          </div>
        </div>
      </nav>

      <div className="border-t border-border p-3">
        <div className="flex items-center gap-3">
          <div
            className="flex size-8 shrink-0 items-center justify-center rounded-full text-sm text-white"
            style={{
              backgroundColor: stringToColor(displayName || user?.email || "?"),
            }}
          >
            {initials}
          </div>
          <span className="min-w-0 flex-1 truncate text-sm font-medium">
            {displayName}
          </span>
          <button
            type="button"
            title="Sign out"
            aria-label="Sign out"
            onClick={handleLogout}
            className="inline-flex size-7 items-center justify-center rounded-lg text-muted-foreground outline-none hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
          >
            <LogOut className="size-4" />
          </button>
        </div>
        <ThemeSelector className="mt-3 min-w-0" />
      </div>
    </div>
  );

  return (
    <>
      <aside
        className={
          desktop
            ? "sticky top-0 z-20 hidden h-screen w-[232px] shrink-0 border-r border-border bg-background md:flex"
            : "hidden"
        }
      >
        {navigation}
      </aside>
      <Sheet open={mobileOpen} onOpenChange={(open) => !open && onClose?.()}>
        <SheetContent side="left" showCloseButton className="w-[232px] p-0">
          <SheetTitle className="sr-only">AME navigation</SheetTitle>
          {navigation}
        </SheetContent>
      </Sheet>
    </>
  );
}
