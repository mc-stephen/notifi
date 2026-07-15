"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { cn } from "@/lib/utils";
import {
  LayoutDashboard,
  ScrollText,
  Workflow,
  Users,
  PlugZap,
  Key,
  Settings,
  type LucideIcon,
} from "lucide-react";

const iconMap: Record<string, LucideIcon> = {
  LayoutDashboard,
  ScrollText,
  Workflow,
  Users,
  PlugZap,
  Key,
  Settings,
};

interface NavItemProps {
  label: string;
  href: string;
  icon: string;
  collapsed?: boolean;
}

export function NavItem({ label, href, icon, collapsed }: NavItemProps) {
  const pathname = usePathname();
  const Icon = iconMap[icon];
  const active =
    href === "/settings/api-keys"
      ? pathname.startsWith("/settings/api-keys")
      : pathname === href || pathname.startsWith(href + "/");

  return (
    <Link
      href={href}
      className={cn(
        "group relative flex items-center gap-3 rounded-lg px-3 py-2.5 text-sm font-medium transition-all duration-150",
        active
          ? "bg-sidebar-accent text-foreground"
          : "text-sidebar-foreground hover:bg-sidebar-hover hover:text-foreground",
        collapsed && "justify-center px-2",
      )}
    >
      {active && (
        <div className="absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-full bg-primary" />
      )}
      {Icon && (
        <Icon
          className={cn(
            "h-5 w-5 shrink-0 transition-colors",
            active ? "text-primary" : "text-sidebar-foreground group-hover:text-foreground",
          )}
        />
      )}
      {!collapsed && <span>{label}</span>}
    </Link>
  );
}
