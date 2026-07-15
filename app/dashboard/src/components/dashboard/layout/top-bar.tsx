"use client";

import { useEnvironmentStore } from "@/store/environment-store";
import { cn } from "@/lib/utils";
import { OrgSelector } from "./org-selector";
import { EnvToggle } from "./env-toggle";
import { UserMenu } from "./user-menu";
import { Bell } from "lucide-react";

interface TopBarProps {
  onToggleSidebar: () => void;
  sidebarCollapsed: boolean;
}

export function TopBar({ onToggleSidebar, sidebarCollapsed }: TopBarProps) {
  const environment = useEnvironmentStore((s) => s.environment);
  const isProd = environment === "production";

  return (
    <header
      className={cn(
        "sticky top-0 z-30 flex h-12 items-center justify-between border-b bg-glass backdrop-blur-[var(--blur-glass)] px-4",
        isProd
          ? "border-b-red-500/20"
          : "border-b-emerald-500/20",
      )}
    >
      {/* Left */}
      <div className="flex items-center gap-3">
        <button
          onClick={onToggleSidebar}
          className="flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground hover:bg-secondary hover:text-foreground transition-colors lg:hidden"
          aria-label="Toggle sidebar"
        >
          <svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M4 6h16M4 12h16M4 18h16" />
          </svg>
        </button>

        <div className="h-5 w-px bg-border hidden sm:block" />

        <OrgSelector />
      </div>

      {/* Right */}
      <div className="flex items-center gap-2.5">
        <EnvToggle />

        <button className="relative flex h-8 w-8 items-center justify-center rounded-lg text-muted-foreground hover:bg-secondary hover:text-foreground transition-colors">
          <Bell className="h-4 w-4" />
          <span className="absolute right-2 top-1.5 h-2 w-2 rounded-full bg-red-500 ring-2 ring-background" />
        </button>

        <UserMenu />
      </div>

      {/* Environment accent strip */}
      <div
        className={cn(
          "absolute left-0 right-0 top-0 h-0.5",
          isProd ? "bg-env-production" : "bg-env-development",
        )}
      />
    </header>
  );
}
