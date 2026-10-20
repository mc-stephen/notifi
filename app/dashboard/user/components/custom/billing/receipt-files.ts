import type { Receipt } from "@/lib/types";
import { format } from "date-fns";

export function invoiceLabel(r: Receipt): string {
  const short = r.id.replace(/^rcpt_/, "").slice(-6).toUpperCase();
  return `Invoice #${short} – ${format(new Date(r.date), "MMM yyyy")}`;
}

export function receiptAmountLabel(r: Receipt): string {
  if (r.amountCents === null) return "Custom";
  const value = r.amountCents / 100;
  return `$${value.toLocaleString(undefined, { maximumFractionDigits: value % 1 === 0 ? 0 : 2 })}`;
}

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

function receiptLines(r: Receipt, projectName: string): string[] {
  return [
    `Receipt: ${r.id}`,
    `Date: ${new Date(r.date).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" })}`,
    `Project: ${projectName}`,
    `Plan: ${r.planName}`,
    `Billing cycle: ${r.billingCycle}`,
    `Amount: ${receiptAmountLabel(r)}${r.amountCents !== null ? ` ${r.currency}` : ""}`,
    `Status: ${r.status}`,
  ];
}

/** Single-receipt PDF, generated client-side (jspdf loads on demand). */
export async function downloadReceiptPdf(receipt: Receipt, projectName: string) {
  const { jsPDF } = await import("jspdf");
  const doc = new jsPDF({ unit: "mm", format: "a4" });
  const left = 20;
  let y = 24;

  doc.setFont("helvetica", "bold");
  doc.setFontSize(22);
  doc.setTextColor(13, 84, 79);
  doc.text("Notifi", left, y);
  y += 8;
  doc.setFontSize(12);
  doc.setTextColor(60, 60, 60);
  doc.text("Payment receipt", left, y);
  y += 6;
  doc.setDrawColor(200, 200, 200);
  doc.line(left, y, 190, y);
  y += 10;

  doc.setFont("helvetica", "normal");
  doc.setFontSize(11);
  doc.setTextColor(20, 20, 20);
  for (const line of receiptLines(receipt, projectName)) {
    doc.text(line, left, y);
    y += 7;
  }

  y += 6;
  doc.setFontSize(9);
  doc.setTextColor(120, 120, 120);
  doc.text("Thank you for using Notifi.", left, y);
  doc.text(
    `Generated ${new Date().toLocaleString()}. Amounts derived from the plan catalog.`,
    left,
    y + 5,
  );

  doc.save(`receipt-${receipt.id}.pdf`);
}

/** Full-statement PDF: every receipt as ledger lines. */
export async function downloadStatementPdf(receipts: Receipt[], projectName: string) {
  const { jsPDF } = await import("jspdf");
  const doc = new jsPDF({ unit: "mm", format: "a4" });
  const left = 20;
  let y = 24;

  doc.setFont("helvetica", "bold");
  doc.setFontSize(22);
  doc.setTextColor(13, 84, 79);
  doc.text("Notifi", left, y);
  y += 8;
  doc.setFontSize(12);
  doc.setTextColor(60, 60, 60);
  doc.text(`Billing statement — ${projectName}`, left, y);
  y += 6;
  doc.setDrawColor(200, 200, 200);
  doc.line(left, y, 190, y);
  y += 10;

  doc.setFont("helvetica", "normal");
  doc.setFontSize(10);
  doc.setTextColor(20, 20, 20);
  for (const r of receipts) {
    if (y > 275) {
      doc.addPage();
      y = 24;
    }
    const date = new Date(r.date).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
    doc.text(
      `${date}  ·  ${r.planName} (${r.billingCycle})  ·  ${receiptAmountLabel(r)}${r.amountCents !== null ? ` ${r.currency}` : ""}  ·  ${r.status}`,
      left,
      y,
    );
    y += 7;
  }

  doc.save(`statement-${projectName.toLowerCase().replace(/[^a-z0-9]+/g, "-")}.pdf`);
}

/** Whole history as CSV. */
export function downloadReceiptsCsv(receipts: Receipt[]) {
  const header = "id,date,plan,cycle,amount_cents,currency,status";
  const rows = receipts.map((r) =>
    [r.id, r.date, `"${r.planName}"`, r.billingCycle, r.amountCents ?? "", r.currency, r.status].join(","),
  );
  downloadBlob(new Blob([[header, ...rows].join("\n")], { type: "text/csv" }), "billing-history.csv");
}
