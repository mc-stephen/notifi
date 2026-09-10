"use client";

import { useState } from "react";
import { type ColumnDef, type Table as ReactTable } from "@tanstack/react-table";
import { DataTable } from "@/components/custom/data-table";
import { PageHeader } from "@/components/custom/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuCheckboxItem,
} from "@/components/ui/dropdown-menu";
import { format } from "date-fns";
import Markdown from "react-markdown";
import rehypeRaw from "rehype-raw";
import rehypeSanitize from "rehype-sanitize";
import {
  Check,
  CheckCheck,
  Trash2,
  UserPlus,
  UserMinus,
  UserCog,
  Link,
  Link2Off,
  KeyRound,
  FolderPlus,
  CreditCard,
  LogIn,
  Megaphone,
  Sparkles,
  Rocket,
  X,
  Search,
  Columns3,
  type LucideIcon,
} from "lucide-react";
import { useInAppNotifications } from "@/hooks";
import type { InAppNotification, InAppNotificationType } from "@/lib/types";
import { cn } from "@/lib/utils";

const TYPE_META: Record<InAppNotificationType, { icon: LucideIcon; tile: string }> = {
  team_add: { icon: UserPlus, tile: "bg-success/10 text-success" },
  team_remove: { icon: UserMinus, tile: "bg-destructive/10 text-destructive" },
  role_change: { icon: UserCog, tile: "bg-info/10 text-info" },
  provider_add: { icon: Link, tile: "bg-primary/10 text-primary" },
  provider_delete: { icon: Link2Off, tile: "bg-destructive/10 text-destructive" },
  api_key_created: { icon: KeyRound, tile: "bg-warning/10 text-warning" },
  api_key_revoked: { icon: KeyRound, tile: "bg-muted-foreground/10 text-muted-foreground" },
  project_created: { icon: FolderPlus, tile: "bg-info/10 text-info" },
  billing_change: { icon: CreditCard, tile: "bg-warning/10 text-warning" },
  system: { icon: Check, tile: "bg-muted text-muted-foreground" },
  new_login: { icon: LogIn, tile: "bg-info/10 text-info" },
  marketing: { icon: Megaphone, tile: "bg-primary/10 text-primary" },
  billing: { icon: CreditCard, tile: "bg-warning/10 text-warning" },
  welcome: { icon: Sparkles, tile: "bg-success/10 text-success" },
  update: { icon: Rocket, tile: "bg-info/10 text-info" },
};

export default function InAppNotificationsPage() {
  const { notifications, unreadCount, markRead, markAllRead, deleteNotification } = useInAppNotifications();
  const [selected, setSelected] = useState<InAppNotification | null>(null);
  const [tableInstance, setTableInstance] = useState<ReactTable<InAppNotification> | null>(null);

  const handleRowClick = (n: InAppNotification) => {
    setSelected(n);
    if (!n.read) markRead(n.id, true);
  };

  const columns: ColumnDef<InAppNotification, unknown>[] = [
    {
      accessorKey: "type",
      header: "Type",
      cell: ({ row }) => {
        const meta = TYPE_META[row.original.type];
        const Icon = meta.icon;
        return (
          <span className={cn("flex size-8 items-center justify-center rounded-lg", meta.tile)}>
            <Icon className="size-4" />
          </span>
        );
      },
      enableHiding: false,
      size: 56,
    },
    {
      id: "notification",
      accessorFn: (row) => `${row.title} ${row.content}`,
      header: "Notification",
      cell: ({ row }) => {
        const n = row.original;
        return (
          <div className="max-w-[400px]">
            <div className={cn("flex items-center gap-2 text-sm", !n.read && "font-semibold")}>
              {n.title}
              {!n.read && <span className="size-1.5 shrink-0 rounded-full bg-primary" />}
            </div>
            <div className="truncate text-xs text-muted-foreground">{n.content}</div>
          </div>
        );
      },
    },
    {
      accessorKey: "createdAt",
      header: () => <span className="text-right">Time</span>,
      cell: ({ row }) => (
        <span className="whitespace-nowrap text-xs text-muted-foreground text-right">
          {format(new Date(row.original.createdAt), "MMM d, HH:mm")}
        </span>
      ),
    },
    {
      accessorKey: "read",
      header: () => <span className="text-right">State</span>,
      cell: ({ row }) => (
        <div className="flex justify-end">
          <Badge variant={row.original.read ? "secondary" : "default"} className={!row.original.read ? "bg-primary/15 text-primary border-primary/20" : ""}>
            {row.original.read ? "Read" : "New"}
          </Badge>
        </div>
      ),
    },
    {
      id: "actions",
      enableHiding: false,
      size: 72,
      cell: ({ row }) => {
        const n = row.original;
        return (
          <div className="flex items-center justify-end gap-1">
            <Button
              variant="ghost"
              size="icon-xs"
              className="text-muted-foreground"
              onClick={() => markRead(n.id, !n.read)}
            >
              <span className="sr-only">{n.read ? "Mark unread" : "Mark read"}</span>
              {n.read ? <CheckCheck className="size-3.5" /> : <Check className="size-3.5" />}
            </Button>
            <Button
              variant="ghost"
              size="icon-xs"
              className="text-muted-foreground hover:text-destructive"
              onClick={() => {
                deleteNotification(n.id);
                if (selected?.id === n.id) setSelected(null);
              }}
            >
              <span className="sr-only">Delete</span>
              <Trash2 className="size-3.5" />
            </Button>
          </div>
        );
      },
    },
  ];

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-6">
      <PageHeader
        title="Notifications"
        description={`${unreadCount} unread · ${notifications.length} total`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Notifications" }]}
        actions={
          <div className="flex items-center gap-2">
            {tableInstance && (
              <>
                <div className="relative max-w-sm">
                  <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
                  <Input
                    placeholder="Search notifications..."
                    value={(tableInstance.getColumn("notification")?.getFilterValue() as string) ?? ""}
                    onChange={(e) => tableInstance.getColumn("notification")?.setFilterValue(e.target.value)}
                    className="pl-8 h-8 w-56 text-sm"
                  />
                </div>
                <DropdownMenu>
                  <DropdownMenuTrigger render={<Button variant="outline" size="sm" className="gap-1.5" />}>
                    <Columns3 className="size-3.5" />
                    <span className="hidden sm:inline">Columns</span>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" className="w-40">
                    <DropdownMenuLabel>Toggle columns</DropdownMenuLabel>
                    <DropdownMenuSeparator />
                    {tableInstance.getAllLeafColumns()
                      .filter((col) => col.id !== "select")
                      .map((col) => (
                        <DropdownMenuCheckboxItem
                          key={col.id}
                          checked={col.getIsVisible()}
                          onCheckedChange={(value) => col.toggleVisibility(!!value)}
                        >
                          {typeof col.columnDef.header === "string" ? col.columnDef.header : col.id}
                        </DropdownMenuCheckboxItem>
                      ))}
                  </DropdownMenuContent>
                </DropdownMenu>
              </>
            )}
            {unreadCount > 0 && (
              <Button size="sm" variant="outline" className="gap-1.5" onClick={markAllRead}>
                <Check className="size-3.5" /> Mark all read
              </Button>
            )}
          </div>
        }
      />

      <div className="flex min-h-0 flex-1 items-stretch gap-6">
        <div className="min-w-0 flex-1 overflow-y-auto">
          <DataTable
            columns={columns}
            data={notifications}
            searchKey="notification"
            pageSize={20}
            emptyMessage="No notifications yet."
            tableClassName="bg-card"
            stripeRows
            onRowClick={handleRowClick}
            hideToolbar
            renderToolbar={(t) => {
              if (!tableInstance) setTableInstance(t);
              return null;
            }}
          />
        </div>

        {selected && (
          <NotificationDetailPanel
            key={selected.id}
            notification={selected}
            onClose={() => setSelected(null)}
            onToggleRead={() => markRead(selected.id, !selected.read)}
            onDelete={() => {
              deleteNotification(selected.id);
              setSelected(null);
            }}
          />
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Detail side panel
// ---------------------------------------------------------------------------

function NotificationDetailPanel({
  notification,
  onClose,
  onToggleRead,
  onDelete,
}: {
  notification: InAppNotification;
  onClose: () => void;
  onToggleRead: () => void;
  onDelete: () => void;
}) {
  const meta = TYPE_META[notification.type];
  const Icon = meta.icon;

  return (
    <aside className="flex min-h-0 w-96 shrink-0 flex-col overflow-hidden rounded-lg border bg-card text-card-foreground shadow-sm animate-in slide-in-from-right-3 fade-in duration-200">
      {/* Header */}
      <div className="border-b p-4">
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-3 min-w-0">
            <span className={cn("flex size-10 shrink-0 items-center justify-center rounded-lg", meta.tile)}>
              <Icon className="size-5" />
            </span>
            <div className="min-w-0">
              <h3 className="truncate font-medium text-base text-foreground">
                {notification.title}
              </h3>
              <p className="text-xs text-muted-foreground mt-0.5">
                {notification.origin === "system" ? "System" : "Admin"} · {format(new Date(notification.createdAt), "MMM d, HH:mm")}
              </p>
            </div>
          </div>
          <button
            type="button"
            aria-label="Close details"
            className="rounded-md p-1.5 text-muted-foreground hover:bg-muted hover:text-foreground"
            onClick={onClose}
          >
            <X className="size-4" />
          </button>
        </div>
      </div>

      {/* Body */}
      <div className="flex-1 min-h-0 overflow-y-auto p-4">
        <div className="prose prose-sm prose-neutral dark:prose-invert max-w-none">
          <Markdown rehypePlugins={[rehypeRaw, rehypeSanitize]}>
            {notification.content}
          </Markdown>
        </div>
      </div>

      {/* Actions */}
      <div className="border-t p-4 flex items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          className="gap-1.5"
          onClick={onToggleRead}
        >
          {notification.read ? <Check className="size-3.5" /> : <CheckCheck className="size-3.5" />}
          {notification.read ? "Mark unread" : "Mark read"}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="gap-1.5 text-destructive hover:text-destructive hover:bg-destructive/10"
          onClick={onDelete}
        >
          <Trash2 className="size-3.5" /> Delete
        </Button>
      </div>
    </aside>
  );
}
