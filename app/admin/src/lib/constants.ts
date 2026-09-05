import {
  LayoutDashboard,
  Users,
  Building2,
  LifeBuoy,
  Send,
  Bell,
  CreditCard,
  Server,
  Shield,
  Settings,
  type LucideIcon,
} from "lucide-react";

export type NavItem = {
  label: string;
  href: string;
  icon: LucideIcon;
  badge?: string | number;
};

export type NavGroup = {
  label: string;
  items: NavItem[];
};

export const NAV_STRUCTURE: NavGroup[] = [
  {
    label: "Overview",
    items: [{ label: "Overview", href: "/overview", icon: LayoutDashboard }],
  },
  {
    label: "Customers",
    items: [
      { label: "Users", href: "/users", icon: Users },
      { label: "Organizations", href: "/organizations", icon: Building2 },
    ],
  },
  {
    label: "Support",
    items: [
      { label: "Tickets", href: "/support", icon: LifeBuoy, badge: 3 },
    ],
  },
  {
    label: "Notifications",
    items: [
      { label: "Send", href: "/notifications/send", icon: Send },
      { label: "History", href: "/notifications", icon: Bell },
    ],
  },
  {
    label: "Billing",
    items: [
      { label: "Overview", href: "/billing", icon: CreditCard },
    ],
  },
  {
    label: "Platform",
    items: [
      { label: "Health", href: "/platform", icon: Server },
      { label: "Providers", href: "/platform/providers", icon: Server },
    ],
  },
  {
    label: "Security",
    items: [
      { label: "Audit Logs", href: "/security/audit-logs", icon: Shield },
    ],
  },
  {
    label: "Administration",
    items: [
      { label: "Admins", href: "/administration/admins", icon: Settings },
      { label: "Settings", href: "/administration/settings", icon: Settings },
    ],
  },
];

export const ACCOUNT_STATUSES = [
  "active",
  "restricted",
  "suspended",
  "banned",
  "deleted",
] as const;

export type AccountStatus = (typeof ACCOUNT_STATUSES)[number];

export const ACCOUNT_STATUS_COLORS: Record<AccountStatus, string> = {
  active: "bg-success/10 text-success border-success/20",
  restricted: "bg-warning/10 text-warning border-warning/20",
  suspended: "bg-destructive/10 text-destructive border-destructive/20",
  banned: "bg-destructive/10 text-destructive border-destructive/20",
  deleted: "bg-muted text-muted-foreground border-border",
};

export const TICKET_STATUSES = [
  "open",
  "pending",
  "waiting",
  "resolved",
  "closed",
] as const;

export type TicketStatus = (typeof TICKET_STATUSES)[number];

export const TICKET_STATUS_COLORS: Record<TicketStatus, string> = {
  open: "bg-info/10 text-info border-info/20",
  pending: "bg-warning/10 text-warning border-warning/20",
  waiting: "bg-muted text-muted-foreground border-border",
  resolved: "bg-success/10 text-success border-success/20",
  closed: "bg-muted text-muted-foreground border-border",
};

export const PRIORITY_COLORS: Record<string, string> = {
  low: "bg-muted text-muted-foreground border-border",
  normal: "bg-info/10 text-info border-info/20",
  high: "bg-warning/10 text-warning border-warning/20",
  urgent: "bg-destructive/10 text-destructive border-destructive/20",
};
