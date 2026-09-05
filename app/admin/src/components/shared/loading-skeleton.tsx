import { cn } from "@/lib/utils";

interface LoadingSkeletonProps {
  className?: string;
  rows?: number;
}

export function LoadingSkeleton({ className, rows = 5 }: LoadingSkeletonProps) {
  return (
    <div className={cn("space-y-3", className)}>
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="flex items-center gap-4">
          <div className="h-4 w-4 rounded bg-primary/10 animate-pulse" />
          <div className="h-4 flex-1 rounded bg-primary/10 animate-pulse" style={{ maxWidth: `${60 + Math.random() * 30}%` }} />
          <div className="h-4 w-20 rounded bg-primary/10 animate-pulse" />
        </div>
      ))}
    </div>
  );
}

export function CardSkeleton({ className }: { className?: string }) {
  return (
    <div className={cn("rounded-xl border bg-card p-5 space-y-3", className)}>
      <div className="h-4 w-24 rounded bg-primary/10 animate-pulse" />
      <div className="h-8 w-32 rounded bg-primary/10 animate-pulse" />
      <div className="h-3 w-20 rounded bg-primary/10 animate-pulse" />
    </div>
  );
}
