import { Bell } from "lucide-react";

export function AuthHeader() {
  return (
    <div className="relative hidden flex-col justify-between bg-gradient-to-br from-primary/10 via-primary/5 to-background p-8 lg:flex lg:w-1/2">
      <div className="flex items-center gap-2">
        <div className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <Bell className="size-4" />
        </div>
        <span className="text-lg font-bold">Notifi</span>
      </div>

      <div className="space-y-4">
        <h1 className="text-3xl font-bold tracking-tight lg:text-4xl">
          Notification Platform
          <br />
          as a Service
        </h1>
        <p className="text-lg text-muted-foreground">
          Send notifications across multiple channels with one API.
          Enterprise-ready. Developer-first.
        </p>
      </div>

      <div className="space-y-2 text-sm text-muted-foreground">
        <div className="flex items-center gap-2">
          <div className="size-1.5 rounded-full bg-success" />
          <span>99.9% uptime guaranteed</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="size-1.5 rounded-full bg-success" />
          <span>SOC 2 Type II compliant</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="size-1.5 rounded-full bg-success" />
          <span>GDPR ready</span>
        </div>
      </div>
    </div>
  );
}

export function AuthHeaderMobile() {
  return (
    <div className="mb-8 flex items-center gap-2 lg:hidden">
      <div className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
        <Bell className="size-4" />
      </div>
      <span className="text-lg font-bold">Notifi</span>
    </div>
  );
}
