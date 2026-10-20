"use client";

import { useState } from "react";
import { type ColumnDef } from "@tanstack/react-table";
import { format } from "date-fns";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Download, FileDown, Loader2, Upload } from "lucide-react";
import { DataTable } from "@/components/custom/data-table";
import type { Receipt } from "@/lib/types";
import {
  downloadReceiptPdf,
  downloadReceiptsCsv,
  downloadStatementPdf,
  invoiceLabel,
  receiptAmountLabel,
} from "./receipt-files";
import { InvoicePreview } from "./invoice-preview";

const columns: ColumnDef<Receipt>[] = [
  {
    accessorKey: "id",
    header: "Invoice",
    cell: ({ row }) => (
      <span className="text-sm font-medium">{invoiceLabel(row.original)}</span>
    ),
  },
  {
    accessorKey: "date",
    header: "Billing Date",
    cell: ({ row }) => (
      <span className="whitespace-nowrap text-sm text-muted-foreground">
        {format(new Date(row.original.date), "MMM d, yyyy")}
      </span>
    ),
  },
  {
    accessorKey: "status",
    header: "Status",
    cell: ({ row }) =>
      row.original.status === "paid" ? (
        <Badge variant="secondary" className="border-success/20 bg-success/15 text-[11px] font-medium text-success">
          Paid
        </Badge>
      ) : (
        <Badge variant="secondary" className="border-destructive/20 bg-destructive/15 text-[11px] font-medium text-destructive">
          Failed
        </Badge>
    ),
  },
  {
    accessorKey: "amountCents",
    header: "Amount",
    cell: ({ row }) => (
      <span className="whitespace-nowrap text-sm tabular-nums">
        {row.original.amountCents !== null ? `${row.original.currency} ` : ""}
        {receiptAmountLabel(row.original)}
      </span>
    ),
  },
  {
    accessorKey: "planName",
    header: "Plan",
    cell: ({ row }) => <span className="text-sm">{row.original.planName}</span>,
  },
];

type InvoiceHistoryProps = {
  receipts: Receipt[];
  projectName: string;
};

/**
 * Billing history table with real file downloads: per-row receipt PDF,
 * whole-history CSV export, and full-statement PDF. Buttons disable with
 * tooltips while there is nothing to download.
 */
export function InvoiceHistory({ receipts, projectName }: InvoiceHistoryProps) {
  const [downloadingId, setDownloadingId] = useState<string | null>(null);
  const [working, setWorking] = useState<"csv" | "statement" | null>(null);
  const [selected, setSelected] = useState<Receipt | null>(null);
  const empty = receipts.length === 0;

  const handleRowDownload = async (receipt: Receipt) => {
    setDownloadingId(receipt.id);
    try {
      await downloadReceiptPdf(receipt, projectName);
    } finally {
      setDownloadingId(null);
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="flex items-center gap-2 font-semibold">
            Billing History
            <Badge variant="secondary" className="text-[10px] font-medium">
              {receipts.length} Total
            </Badge>
          </h3>
          <p className="mt-0.5 text-sm text-muted-foreground">
            Here you will see your billing history.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5"
            disabled={empty || working !== null}
            title={empty ? "No invoices to export yet" : "Download history as CSV"}
            onClick={async () => {
              setWorking("csv");
              try {
                downloadReceiptsCsv(receipts);
              } finally {
                setWorking(null);
              }
            }}
          >
            {working === "csv" ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <Upload className="size-3.5" />
            )}
            Export
          </Button>
          <Button
            size="sm"
            className="gap-1.5"
            disabled={empty || working !== null}
            title={empty ? "No invoices to download yet" : "Download full statement as PDF"}
            onClick={async () => {
              setWorking("statement");
              try {
                await downloadStatementPdf(receipts, projectName);
              } finally {
                setWorking(null);
              }
            }}
          >
            {working === "statement" ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <Download className="size-3.5" />
            )}
            Download
          </Button>
        </div>
      </div>
      <DataTable
        columns={columns}
        data={receipts}
        emptyMessage="No invoices yet."
        hideToolbar
        onRowClick={setSelected}
        rowActions={(receipt) => (
          <Button
            variant="ghost"
            size="icon-xs"
            aria-label={`Download receipt ${receipt.id}`}
            title={downloadingId === receipt.id ? "Preparing…" : "Download receipt PDF"}
            disabled={downloadingId !== null}
            onClick={(e) => {
              e.stopPropagation();
              void handleRowDownload(receipt);
            }}
          >
            {downloadingId === receipt.id ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <FileDown className="size-3.5" />
            )}
          </Button>
        )}
      />
      {selected && (
        <InvoicePreview
          key={selected.id}
          receipt={selected}
          projectName={projectName}
          downloading={downloadingId === selected.id}
          onDownload={() => void handleRowDownload(selected)}
          onClose={() => setSelected(null)}
        />
      )}
    </div>
  );
}
