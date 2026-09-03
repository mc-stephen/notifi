import type {
  NotificationChannel,
  NotificationStatus,
  NotificationPriority,
  Role,
  Environment,
  Plan,
  HealthStatus,
  DevicePlatform,
  NotificationSettingsCategory,
} from "./types";
import {
  LayoutDashboard,
  Bell,
  Users,
  FileText,
  Megaphone,
  Clock,
  Radio,
  BarChart3,
  Webhook,
  KeyRound,
  Package,
  ScrollText,
  UserCog,
  CreditCard,
  Link,
  Settings,
  HelpCircle,
  Inbox,
  type LucideIcon,
} from "lucide-react";

export type NavItem = {
  label: string;
  href: string;
  icon: LucideIcon;
  badge?: string;
};

export type NavGroup = {
  label: string;
  items: NavItem[];
};

export const NAV_GROUPS: NavGroup[] = [
  {
    label: "Core",
    items: [
      { label: "Dashboard", href: "/", icon: LayoutDashboard },
      { label: "Deliveries", href: "/deliveries", icon: Bell },
      { label: "Recipients", href: "/recipients", icon: Users },
      { label: "Templates", href: "/templates", icon: FileText },
    ],
  },
  {
    label: "Campaigns",
    items: [
      { label: "Campaigns", href: "/campaigns", icon: Megaphone },
      { label: "Schedules", href: "/schedules", icon: Clock },
    ],
  },
  {
    label: "Channels",
    items: [
      { label: "Channels", href: "/channels", icon: Radio },
      { label: "Providers", href: "/providers", icon: Link },
    ],
  },
  {
    label: "Insights",
    items: [
      { label: "Analytics", href: "/analytics", icon: BarChart3 },
      { label: "Logs", href: "/logs", icon: ScrollText },
    ],
  },
  {
    label: "Connect",
    items: [
      { label: "Webhooks", href: "/webhooks", icon: Webhook },
      { label: "API Keys", href: "/api-keys", icon: KeyRound },
      { label: "SDKs", href: "/sdk", icon: Package },
    ],
  },
  {
    label: "Workspace",
    items: [
      { label: "Team", href: "/team", icon: UserCog },
      { label: "Billing", href: "/billing", icon: CreditCard },
    ],
  },
  {
    label: "System",
    items: [
      { label: "Settings", href: "/settings", icon: Settings },
      { label: "Notifications", href: "/notifications", icon: Inbox },
      { label: "Support", href: "/support", icon: HelpCircle },
    ],
  },
];

export const CHANNEL_LABELS: Record<NotificationChannel, string> = {
  email: "Email",
  sms: "SMS",
  "push-android": "Android Push",
  "push-ios": "Apple Push",
  "web-push": "Web Push",
  linux: "Linux",
  macos: "macOS",
  rcs: "RCS",
  whatsapp: "WhatsApp",
  slack: "Slack",
  discord: "Discord",
  teams: "Microsoft Teams",
  telegram: "Telegram",
  webhook: "Webhook",
};

export const CHANNEL_ICONS: Record<NotificationChannel, string> = {
  email: "Mail",
  sms: "MessageSquare",
  "push-android": "Smartphone",
  "push-ios": "Smartphone",
  "web-push": "Globe",
  linux: "Monitor",
  macos: "Apple",
  rcs: "MessageCircle",
  whatsapp: "Phone",
  slack: "Hash",
  discord: "Gamepad2",
  teams: "Users",
  telegram: "Send",
  webhook: "Webhook",
};

export const STATUS_LABELS: Record<NotificationStatus, string> = {
  queued: "Queued",
  processing: "Processing",
  sent: "Sent",
  delivered: "Delivered",
  opened: "Opened",
  clicked: "Clicked",
  failed: "Failed",
  cancelled: "Cancelled",
  retrying: "Retrying",
};

export const STATUS_COLORS: Record<NotificationStatus, string> = {
  queued: "bg-info/15 text-info border-info/20",
  processing: "bg-warning/15 text-warning border-warning/20",
  sent: "bg-info/15 text-info border-info/20",
  delivered: "bg-success/15 text-success border-success/20",
  opened: "bg-success/15 text-success border-success/20",
  clicked: "bg-success/15 text-success border-success/20",
  failed: "bg-destructive/15 text-destructive border-destructive/20",
  cancelled: "bg-muted text-muted-foreground border-border",
  retrying: "bg-warning/15 text-warning border-warning/20",
};

export const PRIORITY_LABELS: Record<NotificationPriority, string> = {
  low: "Low",
  normal: "Normal",
  high: "High",
  urgent: "Urgent",
};

export const PRIORITY_COLORS: Record<NotificationPriority, string> = {
  low: "bg-muted text-muted-foreground border-border",
  normal: "bg-info/15 text-info border-info/20",
  high: "bg-warning/15 text-warning border-warning/20",
  urgent: "bg-destructive/15 text-destructive border-destructive/20",
};

export const ROLE_LABELS: Record<Role, string> = {
  owner: "Owner",
  admin: "Admin",
  developer: "Developer",
  viewer: "Viewer",
  billing: "Billing",
};

export const ROLE_COLORS: Record<Role, string> = {
  owner: "bg-primary/15 text-primary border-primary/20",
  admin: "bg-info/15 text-info border-info/20",
  developer: "bg-success/15 text-success border-success/20",
  viewer: "bg-muted text-muted-foreground border-border",
  billing: "bg-warning/15 text-warning border-warning/20",
};

export const ENVIRONMENT_LABELS: Record<Environment, string> = {
  development: "Development",
  production: "Production",
};

export const ENVIRONMENT_COLORS: Record<Environment, string> = {
  development: "bg-info/15 text-info",
  production: "bg-success/15 text-success",
};

export const ENVIRONMENT_DOTS: Record<Environment, string> = {
  development: "bg-info",
  production: "bg-success",
};

export const PLAN_LABELS: Record<Plan, string> = {
  free: "Free",
  starter: "Starter",
  pro: "Pro",
  enterprise: "Enterprise",
};

// TODO: wire to billing API when it lands — these are demo placeholders.
export const CURRENT_PLAN: Plan = "free";

export const PLAN_DESCRIPTIONS: Record<Plan, string> = {
  free: "10k credits/mo",
  starter: "100k credits/mo",
  pro: "Unlimited credits",
  enterprise: "Custom limits",
};

export const PLAN_UPGRADEABLE: Record<Plan, boolean> = {
  free: true,
  starter: true,
  pro: true,
  enterprise: false,
};

export const NOTIFICATION_SETTINGS: NotificationSettingsCategory[] = [
  {
    id: "team",
    label: "Team",
    description: "Changes to project membership and roles.",
    settings: [
      { key: "team_add", label: "Team member added", description: "When someone is added to this project.", email: true, inApp: true },
      { key: "team_remove", label: "Team member removed", description: "When someone is removed from this project.", email: true, inApp: true },
      { key: "team_role_change", label: "Role changed", description: "When a member's role is updated.", email: false, inApp: true },
    ],
  },
  {
    id: "integrations",
    label: "Integrations",
    description: "Channel providers and connections.",
    settings: [
      { key: "provider_add", label: "Provider added", description: "When a new channel provider is connected.", email: true, inApp: true },
      { key: "provider_delete", label: "Provider removed", description: "When a channel provider is disconnected.", email: true, inApp: true },
    ],
  },
  {
    id: "security",
    label: "Security",
    description: "API keys and access changes.",
    settings: [
      { key: "api_key_created", label: "API key created", description: "When a new API key is generated.", email: true, inApp: true },
      { key: "api_key_revoked", label: "API key revoked", description: "When an API key is revoked.", email: true, inApp: true },
    ],
  },
  {
    id: "projects",
    label: "Projects",
    description: "Project lifecycle events.",
    settings: [
      { key: "project_created", label: "Project created", description: "When a new project is created.", email: true, inApp: true },
    ],
  },
  {
    id: "billing",
    label: "Billing",
    description: "Invoices, payments, and plan changes.",
    settings: [
      { key: "billing_invoice_paid", label: "Invoice paid", description: "When an invoice is paid successfully.", email: true, inApp: true },
      { key: "billing_payment_failed", label: "Payment failed", description: "When a payment fails.", email: true, inApp: true },
      { key: "billing_plan_changed", label: "Plan changed", description: "When the workspace plan changes.", email: true, inApp: true },
    ],
  },
];

export const CURRENT_USER_ROLE: Role = "admin";

export const HEALTH_LABELS: Record<HealthStatus, string> = {
  healthy: "Healthy",
  degraded: "Degraded",
  down: "Down",
  unknown: "Unknown",
};

export const HEALTH_COLORS: Record<HealthStatus, string> = {
  healthy: "bg-success",
  degraded: "bg-warning",
  down: "bg-destructive",
  unknown: "bg-muted-foreground",
};

export const DEVICE_PLATFORM_LABELS: Record<DevicePlatform, string> = {
  android: "Android",
  ios: "iOS",
  ipados: "iPadOS",
  macos: "macOS",
  linux: "Linux",
  windows: "Windows",
  browser: "Browser",
};

export const TABLE_PAGE_SIZES = [10, 20, 50, 100] as const;
export const DEFAULT_PAGE_SIZE = 20;
