import type { ReactNode } from "react";
import { TriangleAlert, Info, OctagonX } from "lucide-react";
import { cn } from "@/lib/utils";

type PlanAlertTone = "warning" | "destructive" | "info";

const TONES: Record<
  PlanAlertTone,
  { box: string; icon: string; iconComponent: typeof Info }
> = {
  warning: {
    box: "border-warning/30 bg-warning/10",
    icon: "text-warning",
    iconComponent: TriangleAlert,
  },
  destructive: {
    box: "border-destructive/30 bg-destructive/10",
    icon: "text-destructive",
    iconComponent: OctagonX,
  },
  info: {
    box: "border-info/30 bg-info/10",
    icon: "text-info",
    iconComponent: Info,
  },
};

type PlanAlertProps = {
  tone?: PlanAlertTone;
  title: string;
  message: string;
  action?: ReactNode;
};

/**
 * Tinted callout box for subscription states and errors: icon + bold
 * title + message, optional action button on the right.
 */
export function PlanAlert({ tone = "warning", title, message, action }: PlanAlertProps) {
  const { box, icon, iconComponent: Icon } = TONES[tone];
  return (
    <div className={cn("flex items-center gap-3 rounded-lg border p-4", box)}>
      <Icon className={cn("size-5 shrink-0", icon)} />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-semibold text-foreground">{title}</p>
        <p className="mt-0.5 text-sm text-muted-foreground">{message}</p>
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </div>
  );
}
