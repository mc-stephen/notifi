// Ported from app/user-dashboard/components/custom/data-table.tsx — the
// user-side table, reused here so both surfaces stay visually identical.
// Keep in sync manually. Admin-only deltas: no "use client" (Astro island
// boundary instead), no columns-visibility menu (fixed columns), plus the
// server-mode props below for server-paginated lists.
import { useState } from "react";
import {
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
  getPaginationRowModel,
  flexRender,
  type ColumnDef,
  type SortingState,
  type ColumnFiltersState,
  type Table as ReactTable,
} from "@tanstack/react-table";
import { cn } from "../../lib/utils";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "./table";
import { Button } from "./button";
import { Input } from "./input";
import { Search, ChevronDown, ChevronUp, ChevronsUpDown } from "lucide-react";

const TABLE_PAGE_SIZES = [10, 20, 50];

type DataTableProps<TData, TValue> = {
  columns: ColumnDef<TData, TValue>[];
  data: TData[];
  searchPlaceholder?: string;
  searchKey?: string;
  pageSize?: number;
  bulkActions?: React.ReactNode | ((selectedRows: TData[]) => React.ReactNode);
  emptyMessage?: string;
  onRowClick?: (row: TData) => void;
  rowActions?: (row: TData) => React.ReactNode;
  stripeRows?: boolean;
  tableClassName?: string;
  hideToolbar?: boolean;
  renderToolbar?: (table: ReactTable<TData>) => React.ReactNode;
  // -- server mode (server-paginated lists): paging + total come from the
  // API, sorting is disabled (sorting one page is a lie).
  manualPagination?: boolean;
  pageCount?: number;
  pageIndex?: number;
  onPageChange?: (index: number) => void;
  onPageSizeChange?: (size: number) => void;
  totalCount?: number;
  // -- external search (server-filtered lists): controlled input driving
  // parent queries instead of the internal filter.
  searchValue?: string;
  onSearchChange?: (value: string) => void;
};

function SortIcon({ sorted }: { sorted: false | "asc" | "desc" }) {
  if (sorted === "asc") return <ChevronUp className="size-3" />;
  if (sorted === "desc") return <ChevronDown className="size-3" />;
  return <ChevronsUpDown className="size-3 opacity-50" />;
}

export function DataTable<TData, TValue>({
  columns,
  data,
  searchPlaceholder = "Search...",
  searchKey,
  pageSize: initialPageSize = 20,
  bulkActions,
  emptyMessage = "No results found.",
  onRowClick,
  rowActions,
  stripeRows = false,
  tableClassName,
  hideToolbar = false,
  renderToolbar,
  manualPagination = false,
  pageCount = 1,
  pageIndex = 0,
  onPageChange,
  onPageSizeChange,
  totalCount,
  searchValue,
  onSearchChange,
}: DataTableProps<TData, TValue>) {
  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnFilters, setColumnFilters] = useState<ColumnFiltersState>([]);
  const [globalFilter, setGlobalFilter] = useState("");
  const [rowSelection, setRowSelection] = useState({});
  const [pageSize, setPageSize] = useState(initialPageSize);

  const externalSearch = onSearchChange !== undefined;

  const table = useReactTable({
    data,
    columns,
    manualPagination,
    pageCount: manualPagination ? pageCount : undefined,
    state: {
      sorting,
      columnFilters,
      globalFilter,
      rowSelection,
      pagination: manualPagination ? { pageIndex, pageSize } : { pageIndex: 0, pageSize },
    },
    onSortingChange: setSorting,
    onColumnFiltersChange: setColumnFilters,
    onGlobalFilterChange: setGlobalFilter,
    onRowSelectionChange: setRowSelection,
    onPaginationChange: (updater) => {
      const current = manualPagination ? { pageIndex, pageSize } : { pageIndex: 0, pageSize };
      const next = typeof updater === "function" ? updater(current) : updater;
      if (manualPagination) {
        onPageChange?.(next.pageIndex);
        if (next.pageSize !== pageSize) {
          setPageSize(next.pageSize);
          onPageSizeChange?.(next.pageSize);
        }
      } else {
        setPageSize(next.pageSize);
      }
    },
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getPaginationRowModel: getPaginationRowModel(),
    enableRowSelection: true,
    enableSorting: !manualPagination,
  });

  const selectedCount = Object.keys(rowSelection).length;
  const currentPage = manualPagination ? pageIndex : table.getState().pagination.pageIndex;
  const pageTotal = manualPagination ? pageCount : table.getPageCount();
  const total = manualPagination ? (totalCount ?? data.length) : table.getFilteredRowModel().rows.length;

  return (
    <div className="space-y-3">
      {/* Toolbar */}
      {renderToolbar && renderToolbar(table)}
      {!hideToolbar && !renderToolbar && (
        <div className="flex items-center justify-between gap-3">
          <div className="flex items-center gap-2 flex-1">
            {externalSearch ? (
              <div className="relative max-w-sm flex-1">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
                <Input
                  placeholder={searchPlaceholder}
                  value={searchValue ?? ""}
                  onChange={(e) => onSearchChange?.(e.target.value)}
                  className="pl-8 h-8 text-sm"
                />
              </div>
            ) : searchKey ? (
              <div className="relative max-w-sm flex-1">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
                <Input
                  placeholder={searchPlaceholder}
                  value={(table.getColumn(searchKey)?.getFilterValue() as string) ?? ""}
                  onChange={(e) => table.getColumn(searchKey)?.setFilterValue(e.target.value)}
                  className="pl-8 h-8 text-sm"
                />
              </div>
            ) : (
              <div className="relative max-w-sm flex-1">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
                <Input
                  placeholder={searchPlaceholder}
                  value={globalFilter}
                  onChange={(e) => setGlobalFilter(e.target.value)}
                  className="pl-8 h-8 text-sm"
                />
              </div>
            )}
          </div>

          <div className="flex items-center gap-2">
            {selectedCount > 0 &&
              (typeof bulkActions === "function"
                ? bulkActions(
                    table.getSelectedRowModel().rows.map((row) => row.original),
                  )
                : bulkActions)}
          </div>
        </div>
      )}

      {/* Table */}
      <div className={cn("overflow-hidden rounded-lg border", tableClassName)}>
        <Table>
          <TableHeader>
            {table.getHeaderGroups().map((headerGroup) => (
              <TableRow key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <TableHead
                    key={header.id}
                    className={cn(
                      header.column.getCanSort() && "cursor-pointer select-none",
                    )}
                    onClick={header.column.getToggleSortingHandler()}
                  >
                    {header.isPlaceholder ? null : (
                      <div className="flex items-center gap-1">
                        {flexRender(header.column.columnDef.header, header.getContext())}
                        {header.column.getCanSort() && <SortIcon sorted={header.column.getIsSorted()} />}
                      </div>
                    )}
                  </TableHead>
                ))}
              </TableRow>
            ))}
          </TableHeader>
          <TableBody>
            {table.getRowModel().rows.length === 0 ? (
              <TableRow>
                <TableCell colSpan={columns.length + (rowActions ? 1 : 0)} className="h-24 text-center text-muted-foreground">
                  {emptyMessage}
                </TableCell>
              </TableRow>
            ) : (
              table.getRowModel().rows.map((row) => (
                <TableRow
                  key={row.id}
                  data-state={row.getIsSelected() && "selected"}
                  className={cn(
                    onRowClick && "cursor-pointer",
                    stripeRows && (row.index % 2 === 0 ? "bg-card" : "bg-muted/40"),
                  )}
                  onClick={onRowClick ? () => onRowClick(row.original) : undefined}
                >
                  {row.getVisibleCells().map((cell) => (
                    <TableCell key={cell.id}>
                      {flexRender(cell.column.columnDef.cell, cell.getContext())}
                    </TableCell>
                  ))}
                  {rowActions && (
                    <TableCell className="w-[72px] text-right">
                      {rowActions(row.original)}
                    </TableCell>
                  )}
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      {/* Pagination */}
      <div className="flex items-center justify-between text-sm text-muted-foreground">
        <div className="flex items-center gap-2">
          <span>{total} total</span>
          <span>·</span>
          <select
            value={pageSize}
            onChange={(e) => {
              const size = Number(e.target.value);
              if (manualPagination) {
                table.setPageSize(size);
              } else {
                setPageSize(size);
              }
            }}
            className="border bg-transparent rounded px-1.5 py-0.5 text-xs"
          >
            {TABLE_PAGE_SIZES.map((size) => (
              <option key={size} value={size}>{size} / page</option>
            ))}
          </select>
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="outline"
            size="sm"
            onClick={() => table.previousPage()}
            disabled={!table.getCanPreviousPage()}
          >
            Previous
          </Button>
          <span className="px-2 text-xs">
            Page {currentPage + 1} of {pageTotal}
          </span>
          <Button
            variant="outline"
            size="sm"
            onClick={() => table.nextPage()}
            disabled={!table.getCanNextPage()}
          >
            Next
          </Button>
        </div>
      </div>
    </div>
  );
}
