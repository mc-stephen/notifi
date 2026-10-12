"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Download, Loader2 } from "lucide-react";
import { DetailPanel } from "@/components/custom/detail-panel";
import type { Receipt } from "@/lib/types";
import { invoiceLabel, receiptAmountLabel } from "./receipt-files";

type InvoicePreviewProps = {
  receipt: Receipt;
  projectName: string;
  downloading: boolean;
  onDownload: () => void;
  onClose: () => void;
};

/**
 * Read-only invoice preview in the shared slide-over. Mirrors the
 * downloadable PDF content so preview and file always agree.
 */
export function InvoicePreview({
  receipt,
  projectName,
  downloading,
  onDownload,
  onClose,
}: InvoicePreviewProps) {
  const rows: [string, string][] = [
    ["Date", new Date(receipt.date).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" })],
    ["Project", projectName],
    ["Plan", receipt.planName],
    ["Billing cycle", receipt.billingCycle],
    ["Receipt ID", receipt.id],
  ];

  return (
    <DetailPanel
      onClose={onClose}
      header={
        <div className="flex items-center gap-2">
          <span className="truncate font-semibold text-foreground">
            {invoiceLabel(receipt)}
          </span>
          {receipt.status === "paid" ? (
            <Badge variant="secondary" className="shrink-0 border-success/20 bg-success/15 text-[11px] font-medium text-success">
              Paid
            </Badge>
          ) : (
            <Badge variant="secondary" className="shrink-0 border-destructive/20 bg-destructive/15 text-[11px] font-medium text-destructive">
              Failed
            </Badge>
          )}
        </div>
      }
    >
      <div className="min-h-0 flex-1 overflow-y-auto">
        <div className="space-y-4 px-5 py-4">
          <div className="rounded-lg border bg-muted/30 px-4 py-3 text-center">
            <p className="text-xs text-muted-foreground">Amount due</p>
            <p className="mt-0.5 text-3xl font-bold tracking-tight tabular-nums">
              {receiptAmountLabel(receipt)}
              {receipt.amountCents !== null && (
                <span className="ml-1 text-sm font-normal text-muted-foreground">
                  {receipt.currency}
                </span>
              )}
            </p>
          </div>
          <dl className="space-y-2.5">
            {rows.map(([label, value]) => (
              <div key={label} className="flex items-start justify-between gap-3 text-sm">
                <dt className="shrink-0 text-muted-foreground">{label}</dt>
                <dd className="min-w-0 text-right font-medium break-all">{value}</dd>
              </div>
            ))}
          </dl>
        </div>
      </div>
      <div className="border-t border-border bg-muted/30 px-5 py-3">
        <Button className="w-full gap-1.5" disabled={downloading} onClick={onDownload}>
          {downloading ? (
            <Loader2 className="size-4 animate-spin" />
          ) : (
            <Download className="size-4" />
          )}
          {downloading ? "Preparing…" : "Download PDF"}
        </Button>
      </div>
    </DetailPanel>
  );
}
