"use client";

import { useState, useCallback } from "react";
import { useRouter } from "next/navigation";
import { type ColumnDef } from "@tanstack/react-table";
import { DataTable } from "@/components/custom/data-table";
import { PageHeader } from "@/components/custom/page-header";
import { useRecipients, type RecipientFilters } from "@/hooks/use-recipients";
import type { Recipient } from "@/lib/types";
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
import { MoreHorizontal, Eye, Trash2, Filter, ArrowUpDown, User, Smartphone } from "lucide-react";

const ALL_TAGS = ["vip", "trial", "premium", "enterprise", "active", "inactive", "churned"];
const ALL_SEGMENTS = ["onboarding", "engaged", "dormant", "power-user", "at-risk"];

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

export default function RecipientsPage() {
  const router = useRouter();
  const [filters, setFilters] = useState<RecipientFilters>({});
  const [selectedIds] = useState<string[]>([]);
  const [deleteDialog, setDeleteDialog] = useState<Recipient | null>(null);

  const toggleFilter = useCallback(
    <T extends string>(key: keyof RecipientFilters, value: T) => {
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

  const { items, total, pageSize } = useRecipients(filters, 1, 20);

  const columns: ColumnDef<Recipient, unknown>[] = [
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
      accessorKey: "name",
      header: "Name",
      cell: ({ row }) => (
        <div className="flex items-center gap-2">
          <div className="flex size-7 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-medium">
            {row.original.name.split(" ").map((n) => n[0]).join("")}
          </div>
          <div className="min-w-0">
            <div className="text-sm font-medium truncate">{row.original.name}</div>
            <div className="text-xs text-muted-foreground truncate">{row.original.email}</div>
          </div>
        </div>
      ),
    },
    {
      accessorKey: "id",
      header: "ID",
      cell: ({ row }) => (
        <span className="font-mono text-xs">{row.original.id}</span>
      ),
    },
    {
      accessorKey: "phone",
      header: "Phone",
      cell: ({ row }) => (
        <span className="text-xs text-muted-foreground">{row.original.phone ?? "—"}</span>
      ),
    },
    {
      accessorKey: "tags",
      header: "Tags",
      cell: ({ row }) => (
        <div className="flex flex-wrap gap-1">
          {(row.original.tags ?? []).slice(0, 2).map((tag) => (
            <Badge key={tag} variant="secondary" className="text-[10px] px-1.5 h-4">{tag}</Badge>
          ))}
          {(row.original.tags ?? []).length > 2 && (
            <Badge variant="secondary" className="text-[10px] px-1.5 h-4">+{(row.original.tags ?? []).length - 2}</Badge>
          )}
        </div>
      ),
    },
    {
      accessorKey: "segments",
      header: "Segments",
      cell: ({ row }) => (
        <div className="flex flex-wrap gap-1">
          {(row.original.segments ?? []).slice(0, 2).map((seg) => (
            <Badge key={seg} variant="outline" className="text-[10px] px-1.5 h-4">{seg}</Badge>
          ))}
          {(row.original.segments ?? []).length > 2 && (
            <Badge variant="outline" className="text-[10px] px-1.5 h-4">+{(row.original.segments ?? []).length - 2}</Badge>
          )}
        </div>
      ),
    },
    {
      accessorKey: "language",
      header: "Lang",
      cell: ({ row }) => (
        <span className="text-xs uppercase">{row.original.language ?? "—"}</span>
      ),
    },
    {
      accessorKey: "devices",
      header: ({ column }) => (
        <Button variant="ghost" size="sm" onClick={column.getToggleSortingHandler()} className="gap-1 -ml-2">
          Devices
          <ArrowUpDown className="size-3" />
        </Button>
      ),
      cell: ({ row }) => (
        <div className="flex items-center gap-1 text-xs text-muted-foreground">
          <Smartphone className="size-3" />
          {row.original.devices?.length ?? 0}
        </div>
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
        const r = row.original;
        return (
          <DropdownMenu>
            <DropdownMenuTrigger render={<Button variant="ghost" size="icon-xs" />}>
              <MoreHorizontal className="size-3.5" />
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => router.push(`/recipients/${r.id}`)}>
                <Eye className="size-3.5" /> View profile
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem className="text-destructive" onClick={() => setDeleteDialog(r)}>
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
        title="Recipients"
        description={`${total.toLocaleString()} total recipients`}
        breadcrumbs={[{ label: "Dashboard", href: "/" }, { label: "Recipients" }]}
        actions={
          <Button size="sm">
            <User className="size-3.5 mr-1" /> Add recipient
          </Button>
        }
      />

      <div className="flex items-center gap-2 flex-wrap">
        <FilterDropdown
          label="Tags"
          options={ALL_TAGS.map((t) => ({ value: t, label: t.charAt(0).toUpperCase() + t.slice(1) }))}
          selected={filters.tags ?? []}
          onToggle={(v) => toggleFilter("tags", v)}
        />
        <FilterDropdown
          label="Segments"
          options={ALL_SEGMENTS.map((s) => ({ value: s, label: s.charAt(0).toUpperCase() + s.slice(1) }))}
          selected={filters.segments ?? []}
          onToggle={(v) => toggleFilter("segments", v)}
        />
        {(filters.tags?.length || filters.segments?.length) ? (
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
        searchKey="name"
        searchPlaceholder="Search by name or email..."
        pageSize={pageSize}
        bulkActions={
          <Button variant="outline" size="sm" className="gap-1.5 text-destructive">
            <Trash2 className="size-3" />
            Delete ({selectedIds.length})
          </Button>
        }
      />

      <Dialog open={!!deleteDialog} onOpenChange={() => setDeleteDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete recipient</DialogTitle>
            <DialogDescription>
              Permanently delete {deleteDialog?.name} ({deleteDialog?.id})? This cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialog(null)}>Cancel</Button>
            <Button variant="destructive" onClick={() => setDeleteDialog(null)}>Delete</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
