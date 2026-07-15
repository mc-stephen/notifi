"use client";

import { cn } from "@/lib/utils";
import { NAV_ITEMS } from "@/lib/constants";
import { NavItem } from "./nav-item";
import { User, LifeBuoy, ChevronDown } from "lucide-react";

interface SidebarProps {
  collapsed: boolean;
}

export function Sidebar({ collapsed }: SidebarProps) {
  const mainNav = NAV_ITEMS.slice(0, 5);
  const bottomNav = NAV_ITEMS.slice(5);

  return (
    <aside
      className={cn(
        "flex h-full flex-col border-r border-border bg-sidebar transition-all duration-200",
        collapsed ? "w-16" : "w-60",
      )}
    >
      {/* Logo */}
      <div
        className={cn(
          "flex h-14 items-center gap-2.5 border-b border-border px-4",
          collapsed && "justify-center px-0",
        )}
      >
        <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary text-sm font-bold text-primary-foreground shadow-sm">
          N
        </span>
        {!collapsed && (
          <span className="text-base font-semibold tracking-tight">Notifi</span>
        )}
      </div>

      {/* Main nav */}
      <nav className="flex-1 space-y-0.5 p-2">
        {!collapsed && (
          <p className="px-3 pb-1 pt-3 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            Main
          </p>
        )}
        {mainNav.map((item) => (
          <NavItem
            key={item.href}
            label={item.label}
            href={item.href}
            icon={item.icon}
            collapsed={collapsed}
          />
        ))}

        {!collapsed && (
          <p className="px-3 pb-1 pt-4 text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
            Settings
          </p>
        )}
        {bottomNav.map((item) => (
          <NavItem
            key={item.href}
            label={item.label}
            href={item.href}
            icon={item.icon}
            collapsed={collapsed}
          />
        ))}
      </nav>

      {/* User section */}
      <div
        className={cn(
          "border-t border-border p-2",
          collapsed && "flex justify-center",
        )}
      >
        <div
          className={cn(
            "flex items-center gap-3 rounded-lg px-3 py-2.5 transition-colors hover:bg-sidebar-hover",
            collapsed && "justify-center px-2",
          )}
        >
          <div className="flex h-7 w-7 items-center justify-center rounded-full bg-primary/15 text-xs font-medium text-primary">
            JD
          </div>
          {!collapsed && (
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-foreground truncate">John Doe</p>
              <p className="text-[11px] text-muted-foreground truncate">Acme Corp</p>
            </div>
          )}
        </div>
      </div>
    </aside>
  );
}
