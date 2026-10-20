"use client";

import { AlertCircle } from "lucide-react";
import { cn } from "@/lib/utils";

type ErrorBannerProps = {
  message: string;
  className?: string;
};

export function ErrorBanner({ message, className }: ErrorBannerProps) {
  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-lg border border-destructive/20 bg-destructive/5 p-3 text-sm text-destructive",
        className
      )}
      role="alert"
    >
      <AlertCircle className="size-4 shrink-0 mt-0.5" />
      <span>{message}</span>
    </div>
  );
}
