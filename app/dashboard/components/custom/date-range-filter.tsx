"use client";

import { cn } from "@/lib/utils";

export const DATE_RANGES = [
  { label: "Today", value: "today" },
  { label: "7 days", value: "7d" },
  { label: "30 days", value: "30d" },
  { label: "This month", value: "month" },
  { label: "This quarter", value: "quarter" },
  { label: "This year", value: "year" },
] as const;

export type DateRange = (typeof DATE_RANGES)[number]["value"];

type DateRangeFilterProps = {
  value: DateRange;
  onChange: (value: DateRange) => void;
  className?: string;
};

export function DateRangeFilter({
  value,
  onChange,
  className,
}: DateRangeFilterProps) {
  return (
    <div
      className={cn(
        "inline-flex items-center gap-1 rounded-lg border bg-muted p-0.5",
        className,
      )}
    >
      {DATE_RANGES.map((range) => (
        <button
          key={range.value}
          onClick={() => onChange(range.value)}
          className={cn(
            "rounded-md px-3 py-1.5 text-xs font-medium transition-all",
            "hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            value === range.value
              ? "bg-background text-foreground shadow-sm"
              : "text-muted-foreground",
          )}
        >
          {range.label}
        </button>
      ))}
    </div>
  );
}
