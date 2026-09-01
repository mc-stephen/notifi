"use client";

import { usePathname } from "next/navigation";
import Link from "next/link";
import { cn } from "@/lib/utils";
import { Tooltip, TooltipTrigger, TooltipContent } from "@/components/ui/tooltip";
import { Collapsible, CollapsibleTrigger, CollapsibleContent } from "@/components/ui/collapsible";
import { NAV_GROUPS } from "@/lib/constants";
import { useUIStore } from "@/store/ui-store";
import { ProjectSwitcher } from "./project-switcher";
import { PlanBanner } from "@/components/custom/plan-banner";
import {
  PanelLeftClose,
  PanelLeftOpen,
  ChevronRight,
} from "lucide-react";

function SidebarItem({ item, collapsed }: { item: (typeof NAV_GROUPS)[number]["items"][number]; collapsed: boolean }) {
  const pathname = usePathname();
  const isActive = pathname === item.href || (item.href !== "/" && pathname.startsWith(item.href));

  const link = (
    <Link
      href={item.href}
      className={cn(
        "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
        isActive
          ? "bg-primary/10 text-primary"
          : "text-sidebar-foreground/70 hover:bg-sidebar-accent/60 hover:text-sidebar-foreground",
        collapsed && "justify-center px-2",
      )}
    >
      <item.icon className="size-4 shrink-0" />
      {!collapsed && <span className="truncate">{item.label}</span>}
      {!collapsed && item.badge && (
        <span className="ml-auto flex size-5 items-center justify-center rounded-full bg-primary/10 text-[10px] font-semibold text-primary">
          {item.badge}
        </span>
      )}
    </Link>
  );

  if (collapsed) {
    return (
      <Tooltip>
        <TooltipTrigger render={link} />
        <TooltipContent side="right">{item.label}</TooltipContent>
      </Tooltip>
    );
  }

  return link;
}

function SidebarGroup({ group, collapsed }: { group: (typeof NAV_GROUPS)[number]; collapsed: boolean }) {
  if (collapsed) {
    return (
      <div className="space-y-1">
        {group.items.map((item) => (
          <SidebarItem key={item.href} item={item} collapsed={collapsed} />
        ))}
      </div>
    );
  }

  return (
    <Collapsible defaultOpen>
      <CollapsibleTrigger
        render={
          <button className="flex w-full items-center justify-between rounded-lg px-3 py-1.5 text-xs font-semibold uppercase tracking-wider text-sidebar-foreground/50 transition-colors hover:bg-sidebar-accent/40 hover:text-sidebar-foreground/70" />
        }
      >
        <span>{group.label}</span>
        <ChevronRight className="size-3 transition-transform group-data-[state=open]:rotate-90" />
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="space-y-0.5 mt-1">
          {group.items.map((item) => (
            <SidebarItem key={item.href} item={item} collapsed={collapsed} />
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

export function Sidebar({ className, mobile = false, onNavigate }: { className?: string; mobile?: boolean; onNavigate?: () => void }) {
  const { sidebarCollapsed, toggleSidebar } = useUIStore();
  const collapsed = !mobile && sidebarCollapsed;

  return (
    <aside
      className={cn(
        "relative flex h-full flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground transition-all duration-200",
        collapsed ? "w-16" : "w-60",
        className,
      )}
    >
      {/* Logo */}
      <div className={cn("flex h-14 items-center border-b border-sidebar-border px-3", collapsed && "justify-center")}>
        {!collapsed ? (
          <Link href="/" className="flex items-center gap-2 font-semibold text-base" onClick={onNavigate}>
            <div className="flex size-7 items-center justify-center rounded-lg bg-primary text-primary-foreground text-sm font-bold">
              N
            </div>
            <span className="truncate">Notifi</span>
          </Link>
        ) : (
          <Link href="/" onClick={onNavigate}>
            <div className="flex size-7 items-center justify-center rounded-lg bg-primary text-primary-foreground text-sm font-bold">
              N
            </div>
          </Link>
        )}
      </div>

      {/* Navigation — only this section scrolls */}
      <div className="flex-1 min-h-0 overflow-y-auto px-2 py-3">
        <nav className="space-y-4">
          {NAV_GROUPS.map((group) => (
            <SidebarGroup key={group.label} group={group} collapsed={collapsed} />
          ))}
        </nav>
      </div>

      {/* Pinned footer — project switcher + plan banner */}
      <div className="shrink-0 space-y-2 border-t border-sidebar-border px-2 pt-3 pb-2">
        <ProjectSwitcher collapsed={collapsed} />
        <PlanBanner collapsed={collapsed} />
      </div>

      {/* Seam toggle — straddles sidebar/topbar border */}
      {!mobile && (
        <Tooltip>
          <TooltipTrigger
            render={
              <button
                onClick={toggleSidebar}
                className="absolute right-0 top-[12px] z-50 flex size-7 translate-x-1/2 items-center justify-center rounded-full border border-sidebar-border bg-sidebar text-sidebar-foreground/50 transition-colors hover:bg-sidebar-accent/60 hover:text-sidebar-foreground"
              />
            }
          >
            {collapsed ? (
              <PanelLeftOpen className="size-4" />
            ) : (
              <PanelLeftClose className="size-4" />
            )}
          </TooltipTrigger>
          <TooltipContent side="right">
            {collapsed ? "Expand sidebar" : "Collapse sidebar"}
          </TooltipContent>
        </Tooltip>
      )}
    </aside>
  );
}
