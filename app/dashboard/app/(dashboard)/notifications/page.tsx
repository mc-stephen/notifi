"use client";

import { useState } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { DataTable } from "@/components/custom/data-table";
import { PageHeader } from "@/components/custom/page-header";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { format } from "date-fns";
import { Check, CheckCheck, Trash2, UserPlus, UserMinus, UserCog, Link, Link2Off, KeyRound, FolderPlus, CreditCard, type LucideIcon } from "lucide-react";
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
};

export default function InAppNotificationsPage() {
  const notifications = useInAppNotifications();
  const [read, setRead] = useState<Record<string, boolean>>(() =>
    Object.fromEntries(notifications.map((n) => [n.id, !n.read])),
  );
  const [deleted, setDeleted] = useState<Set<string>>(() => new Set());

  const visible = notifications.filter((n) => !deleted.has(n.id));
  const unread = visible.filter((n) => !read[n.id]).length;

  const markAllRead = () =>
    setRead(() => Object.fromEntries(visible.map((n) => [n.id, true])));

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
      accessorFn: (row) => `${row.title} ${row.message}`,
      header: "Notification",
      cell: ({ row }) => {
        const n = row.original;
        return (
          <div className="max-w-[320px]">
            <div className={cn("flex items-center gap-2 text-sm", !read[n.id] && "font-semibold")}>
              {n.title}
              {!read[n.id] && <span className="size-1.5 shrink-0 rounded-full bg-primary" />}
            </div>
            <div className="truncate text-xs text-muted-foreground">{n.message}</div>
          </div>
        );
      },
    },
    {
      accessorKey: "actorName",
      header: "Actor",
      cell: ({ row }) => (
        <span className="text-sm text-muted-foreground">{row.original.actorName ?? "—"}</span>
      ),
    },
    {
      accessorKey: "projectName",
      header: "Project",
      cell: ({ row }) => <span className="text-sm">{row.original.projectName}</span>,
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
      cell: ({ row }) => {
        const isRead = read[row.original.id];
        return (
          <div className="flex justify-end">
            <Badge variant={isRead ? "secondary" : "default"} className={!isRead ? "bg-primary/15 text-primary border-primary/20" : ""}>
              {isRead ? "Read" : "New"}
            </Badge>
          </div>
        );
      },
    },
    {
      id: "actions",
      enableHiding: false,
      size: 72,
      cell: ({ row }) => {
        const n = row.original;
        const isRead = read[n.id];
        return (
          <div className="flex items-center justify-end gap-1">
            <Button
              variant="ghost"
              size="icon-xs"
              className="text-muted-foreground"
              onClick={() => setRead((prev) => ({ ...prev, [n.id]: !prev[n.id] }))}
            >
              <span className="sr-only">{isRead ? "Mark unread" : "Mark read"}</span>
              {isRead ? <CheckCheck className="size-3.5" /> : <Check className="size-3.5" />}
            </Button>
            <Button
              variant="ghost"
              size="icon-xs"
              className="text-muted-foreground hover:text-destructive"
              onClick={() =>
                setDeleted((prev) => new Set(prev).add(n.id))
              }
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
    <div className="space-y-6">
      <PageHeader
        title="Notifications"
        description={`${unread} unread · ${visible.length} total`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Notifications" }]}
        actions={
          unread > 0 ? (
            <Button size="sm" variant="outline" className="gap-1.5" onClick={markAllRead}>
              <Check className="size-3.5" /> Mark all read
            </Button>
          ) : undefined
        }
      />

      <DataTable
        columns={columns}
        data={visible}
        searchKey="notification"
        searchPlaceholder="Search notifications..."
        pageSize={20}
        emptyMessage="No notifications yet."
      />
    </div>
  );
}
