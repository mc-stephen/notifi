"use client";

import { useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import { type ColumnDef } from "@tanstack/react-table";
import { DataTable } from "@/components/custom/data-table";
import { PageHeader } from "@/components/custom/page-header";
import { StatusBadge } from "@/components/custom/status-badge";
import { PriorityBadge } from "@/components/custom/priority-badge";
import { ChannelBadge } from "@/components/custom/channel-badge";
import { useNotifications, type NotificationFilters } from "@/hooks/use-notifications";
import type { Notification, NotificationChannel, NotificationStatus, NotificationPriority } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuCheckboxItem,
  DropdownMenuLabel,
} from "@/components/ui/dropdown-menu";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { format } from "date-fns";
import {
  MoreHorizontal,
  RotateCcw,
  X,
  Copy,
  Eye,
  Trash2,
  Filter,
  ArrowUpDown,
} from "lucide-react";

const CHANNELS: NotificationChannel[] = ["email", "sms", "push-android", "push-ios", "web-push", "webhook"];
const STATUSES: NotificationStatus[] = ["queued", "processing", "sent", "delivered", "opened", "clicked", "failed", "cancelled", "retrying"];
const PRIORITIES: NotificationPriority[] = ["low", "normal", "high", "urgent"];

function FilterDropdown<T extends string>({
  label,
  options,
  selected,
  onToggle,
}: {
  label: string;
  options: { value: T; label: string }[];
  selected: T[];
  onToggle: (value: T) => void;
}) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger render={<Button variant="outline" size="sm" className="gap-1.5 h-8" />}>
        <Filter className="size-3" />
        {label}
        {selected.length > 0 && (
          <Badge variant="secondary" className="ml-1 h-4 px-1 text-[10px]">{selected.length}</Badge>
        )}
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-44">
        <DropdownMenuLabel>{label}</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {options.map((opt) => (
          <DropdownMenuCheckboxItem
            key={opt.value}
            checked={selected.includes(opt.value)}
            onCheckedChange={() => onToggle(opt.value)}
          >
            {opt.label}
          </DropdownMenuCheckboxItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

export default function NotificationsPage() {
  const router = useRouter();
  const [filters, setFilters] = useState<NotificationFilters>({});
  const [selectedIds] = useState<string[]>([]);
  const [actionDialog, setActionDialog] = useState<{ type: string; notification: Notification } | null>(null);

  const toggleFilter = useCallback(
    <T extends string>(key: keyof NotificationFilters, value: T) => {
      setFilters((prev) => {
        const current = (prev[key] ?? []) as T[];
        const next = current.includes(value)
          ? current.filter((v) => v !== value)
          : [...current, value];
        return { ...prev, [key]: next.length > 0 ? next : undefined };
      });
    },
    [],
  );

  const { items, total, pageSize } = useNotifications(filters, undefined, 1, 20);

  const columns: ColumnDef<Notification, unknown>[] = [
    {
      id: "select",
      header: ({ table }) => (
        <Checkbox
          checked={table.getIsAllPageRowsSelected()}
          onCheckedChange={(value) => table.toggleAllPageRowsSelected(!!value)}
        />
      ),
      cell: ({ row }) => (
        <Checkbox
          checked={row.getIsSelected()}
          onCheckedChange={(value) => row.toggleSelected(!!value)}
        />
      ),
      enableSorting: false,
      enableHiding: false,
      size: 40,
    },
    {
      accessorKey: "id",
      header: "ID",
      cell: ({ row }) => (
        <span className="font-mono text-xs">{row.original.id}</span>
      ),
    },
    {
      accessorKey: "channel",
      header: "Channel",
      cell: ({ row }) => <ChannelBadge channel={row.original.channel} showIcon />,
    },
    {
      accessorKey: "subject",
      header: "Subject",
      cell: ({ row }) => (
        <span className="truncate max-w-[200px] text-sm">
          {row.original.subject ?? row.original.body}
        </span>
      ),
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => <StatusBadge status={row.original.status} />,
    },
    {
      accessorKey: "priority",
      header: ({ column }) => (
        <Button variant="ghost" size="sm" onClick={column.getToggleSortingHandler()} className="gap-1 -ml-2">
          Priority
          <ArrowUpDown className="size-3" />
        </Button>
      ),
      cell: ({ row }) => <PriorityBadge priority={row.original.priority} />,
    },
    {
      accessorKey: "recipientId",
      header: "Recipient",
      cell: ({ row }) => (
        <span className="font-mono text-xs">{row.original.recipientId}</span>
      ),
    },
    {
      accessorKey: "createdAt",
      header: ({ column }) => (
        <Button variant="ghost" size="sm" onClick={column.getToggleSortingHandler()} className="gap-1 -ml-2">
          Created
          <ArrowUpDown className="size-3" />
        </Button>
      ),
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {format(new Date(row.original.createdAt), "MMM d, HH:mm")}
        </span>
      ),
    },
    {
      id: "actions",
      enableHiding: false,
      size: 40,
      cell: ({ row }) => {
        const n = row.original;
        return (
          <DropdownMenu>
            <DropdownMenuTrigger render={<Button variant="ghost" size="icon-xs" />}>
              <MoreHorizontal className="size-3.5" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => router.push(`/deliveries/${n.id}`)}>
                <Eye className="size-3.5" /> View details
              </DropdownMenuItem>
              {n.status === "failed" && (
                <DropdownMenuItem onClick={() => setActionDialog({ type: "retry", notification: n })}>
                  <RotateCcw className="size-3.5" /> Retry
                </DropdownMenuItem>
              )}
              {n.status !== "cancelled" && n.status !== "failed" && (
                <DropdownMenuItem onClick={() => setActionDialog({ type: "cancel", notification: n })}>
                  <X className="size-3.5" /> Cancel
                </DropdownMenuItem>
              )}
              <DropdownMenuItem onClick={() => setActionDialog({ type: "duplicate", notification: n })}>
                <Copy className="size-3.5" /> Duplicate
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem className="text-destructive" onClick={() => setActionDialog({ type: "delete", notification: n })}>
                <Trash2 className="size-3.5" /> Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        );
      },
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="Deliveries"
        description={`${total.toLocaleString()} total deliveries`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Deliveries" }]}
        actions={
          <Button size="sm">
            Send notification
          </Button>
        }
      />

      {/* Filters */}
      <div className="flex items-center gap-2 flex-wrap">
        <FilterDropdown
          label="Status"
          options={STATUSES.map((s) => ({ value: s, label: s.charAt(0).toUpperCase() + s.slice(1) }))}
          selected={filters.status ?? []}
          onToggle={(v) => toggleFilter("status", v)}
        />
        <FilterDropdown
          label="Channel"
          options={CHANNELS.map((c) => ({ value: c, label: c }))}
          selected={filters.channel ?? []}
          onToggle={(v) => toggleFilter("channel", v)}
        />
        <FilterDropdown
          label="Priority"
          options={PRIORITIES.map((p) => ({ value: p, label: p.charAt(0).toUpperCase() + p.slice(1) }))}
          selected={filters.priority ?? []}
          onToggle={(v) => toggleFilter("priority", v)}
        />
        {(filters.status?.length || filters.channel?.length || filters.priority?.length) ? (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setFilters({})}
            className="text-xs text-muted-foreground"
          >
            Clear filters
          </Button>
        ) : null}
      </div>

      <DataTable
        columns={columns}
        data={items}
        searchKey="id"
        searchPlaceholder="Search by ID..."
        pageSize={pageSize}
        bulkActions={
          <Button variant="outline" size="sm" className="gap-1.5 text-destructive">
            <Trash2 className="size-3" />
            Delete ({selectedIds.length})
          </Button>
        }
      />

      {/* Action Dialog */}
      <Dialog open={!!actionDialog} onOpenChange={() => setActionDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {actionDialog?.type === "retry" && "Retry notification"}
              {actionDialog?.type === "cancel" && "Cancel notification"}
              {actionDialog?.type === "duplicate" && "Duplicate notification"}
              {actionDialog?.type === "delete" && "Delete notification"}
            </DialogTitle>
            <DialogDescription>
              {actionDialog?.type === "retry" && `Retry sending ${actionDialog?.notification.id}?`}
              {actionDialog?.type === "cancel" && `Cancel ${actionDialog?.notification.id}? This cannot be undone.`}
              {actionDialog?.type === "duplicate" && `Create a copy of ${actionDialog?.notification.id}?`}
              {actionDialog?.type === "delete" && `Permanently delete ${actionDialog?.notification.id}? This cannot be undone.`}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setActionDialog(null)}>Cancel</Button>
            <Button
              variant={actionDialog?.type === "delete" ? "destructive" : "default"}
              onClick={() => setActionDialog(null)}
            >
              Confirm
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
