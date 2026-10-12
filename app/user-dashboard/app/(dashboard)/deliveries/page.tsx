"use client";

import { useMemo, useState } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { DataTable } from "@/components/custom/data-table";
import { PageHeader } from "@/components/custom/page-header";
import { useSends } from "@/hooks/use-sends";
import type { SendStatus } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuCheckboxItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
} from "@/components/ui/dropdown-menu";
import { format } from "date-fns";
import { Filter, Loader2, AlertTriangle } from "lucide-react";

const CHANNELS = ["email", "sms", "push", "chat"];
const STATUSES: SendStatus[] = ["pending", "sending", "completed", "failed"];

const STATUS_STYLES: Record<SendStatus, string> = {
  pending: "",
  sending: "border-info/20 bg-info/15 text-info",
  completed: "border-success/20 bg-success/15 text-success",
  failed: "border-destructive/20 bg-destructive/15 text-destructive",
};

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

type SendFilters = {
  status?: SendStatus[];
  channel?: string[];
};

export default function DeliveriesPage() {
  const { sends, loading, error } = useSends();
  const [filters, setFilters] = useState<SendFilters>({});

  const toggleFilter = <T extends string>(key: keyof SendFilters, value: T) => {
    setFilters((prev) => {
      const current = (prev[key] ?? []) as T[];
      const next = current.includes(value)
        ? current.filter((v) => v !== value)
        : [...current, value];
      return { ...prev, [key]: next.length > 0 ? next : undefined };
    });
  };

  const items = useMemo(() => {
    return sends.filter((s) => {
      if (filters.status?.length && !filters.status.includes(s.status)) return false;
      if (filters.channel?.length && !filters.channel.includes(s.channel)) return false;
      return true;
    });
  }, [sends, filters]);

  const columns: ColumnDef<(typeof items)[number], unknown>[] = [
    {
      accessorKey: "id",
      header: "ID",
      cell: ({ row }) => (
        <span className="font-mono text-xs" title={row.original.id}>
          {row.original.id.slice(0, 13)}…
        </span>
      ),
    },
    {
      accessorKey: "channel",
      header: "Channel",
      cell: ({ row }) => (
        <Badge variant="secondary" className="font-mono text-[10px]">
          {row.original.channel}
        </Badge>
      ),
    },
    {
      accessorKey: "title",
      header: "Subject",
      cell: ({ row }) => (
        <span className="truncate max-w-[240px] text-sm" title={row.original.title}>
          {row.original.title}
        </span>
      ),
    },
    {
      accessorKey: "status",
      header: "Status",
      cell: ({ row }) => (
        <Badge variant={row.original.status === "completed" ? "default" : "secondary"} className={`text-[10px] font-medium capitalize ${STATUS_STYLES[row.original.status]}`}>
          {row.original.status}
        </Badge>
      ),
    },
    {
      accessorKey: "recipients",
      header: "Recipients",
      cell: ({ row }) => (
        <span className="text-sm tabular-nums">{row.original.recipients.length}</span>
      ),
    },
    {
      accessorKey: "createdAt",
      header: "Created",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground whitespace-nowrap">
          {format(new Date(row.original.createdAt), "MMM d, HH:mm")}
        </span>
      ),
    },
  ];

  return (
    <div className="space-y-6">
      <PageHeader
        title="Deliveries"
        description={`${sends.length} total deliveries`}
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
        {(filters.status?.length || filters.channel?.length) ? (
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

      {loading ? (
        <div className="overflow-hidden rounded-lg border bg-card">
          <div className="flex items-center justify-center py-16 text-muted-foreground">
            <Loader2 className="size-5 animate-spin mr-2" /> Loading deliveries…
          </div>
        </div>
      ) : error ? (
        <div className="overflow-hidden rounded-lg border bg-card">
          <div className="flex flex-col items-center justify-center gap-2 py-16 text-center">
            <AlertTriangle className="size-6 text-warning" />
            <p className="text-sm text-muted-foreground">{error}</p>
          </div>
        </div>
      ) : (
        <DataTable
          columns={columns}
          data={items}
          searchPlaceholder="Search deliveries..."
          emptyMessage="No deliveries yet. Send your first notification via POST /v1/notifications."
        />
      )}
    </div>
  );
}
