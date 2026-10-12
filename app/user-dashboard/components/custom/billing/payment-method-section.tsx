import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { CreditCard, Pencil } from "lucide-react";

export type PaymentMethod = {
  id: string;
  brand: string;
  last4: string;
  expiry: string;
};

type PaymentMethodSectionProps = {
  methods: PaymentMethod[];
  onEdit?: (method: PaymentMethod) => void;
};

/**
 * Payment methods for the project. Empty until a payment provider lands —
 * renders an honest empty state instead of placeholder card numbers.
 */
export function PaymentMethodSection({ methods, onEdit }: PaymentMethodSectionProps) {
  return (
    <div className="space-y-4">
      <div>
        <h3 className="font-semibold">Payment Method</h3>
        <p className="mt-0.5 text-xs text-muted-foreground">
          {methods.length === 0
            ? "No payment method on file."
            : "Active payment method for this project."}
        </p>
      </div>
      {methods.length === 0 ? (
        <div className="rounded-lg border border-dashed px-4 py-6 text-center">
          <CreditCard className="mx-auto size-5 text-muted-foreground" />
          <p className="mt-2 text-sm font-medium">No payment method yet</p>
          <p className="mt-0.5 text-xs text-muted-foreground">
            Online payments land with the billing provider.
          </p>
        </div>
      ) : (
        <div className="space-y-2">
          {methods.map((m) => (
            <div
              key={m.id}
              className="flex items-center gap-3 rounded-lg border px-3 py-2.5"
            >
              <span className="rounded border px-1.5 py-0.5 font-mono text-[10px] font-bold uppercase">
                {m.brand}
              </span>
              <div className="min-w-0 flex-1">
                <p className="truncate text-sm font-medium">
                  {m.brand} ending in {m.last4}
                </p>
                <p className="text-xs text-muted-foreground">
                  Expires {m.expiry}
                </p>
              </div>
              <Badge variant="secondary" className="text-[10px]">
                Active
              </Badge>
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5"
                onClick={() => onEdit?.(m)}
              >
                <Pencil className="size-3" /> Edit
              </Button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
