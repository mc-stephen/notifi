import { Bell, Send, Globe, BarChart3 } from "lucide-react";

const CLIENT_LOGOS = [
  "Discord",
  "Mailchimp",
  "Grammarly",
  "Intercom",
  "Square",
  "Dropbox",
  "Slack",
  "Figma",
];

const FEATURES = [
  {
    icon: Send,
    title: "Multi-channel delivery",
    description: "Email, SMS, Push and Webhooks through a single API.",
  },
  {
    icon: Globe,
    title: "Global routing",
    description: "Per-region delivery, automatic retries and fallbacks.",
  },
  {
    icon: BarChart3,
    title: "Real-time analytics",
    description: "Track every event from send to click.",
  },
];

export function AuthHeader() {
  return (
    <div className="relative hidden flex-col justify-between overflow-hidden bg-gradient-to-br from-[oklch(0.30_0.042_202.8)] via-[oklch(0.345_0.046_202.8)] to-[oklch(0.40_0.05_202.8)] p-8 dark:from-[oklch(0.44_0.06_202.8)] dark:via-[oklch(0.48_0.065_202.8)] dark:to-[oklch(0.54_0.07_202.8)] lg:flex lg:w-1/2">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_50%_25%,rgba(255,255,255,0.10),transparent_60%)]" />

      <div className="relative flex items-center gap-2">
        <div className="flex size-8 items-center justify-center rounded-lg bg-primary-foreground text-primary">
          <Bell className="size-4" />
        </div>
        <span className="text-lg font-bold text-primary-foreground">Notifi</span>
      </div>

      <div className="relative space-y-8">
        <div className="space-y-4">
          <h1 className="text-3xl font-bold tracking-tight text-primary-foreground lg:text-4xl">
            Revolutionize how you reach your users
          </h1>
          <p className="text-lg text-primary-foreground/70">
            Send notifications across every channel with one API.
            Enterprise-ready. Developer-first.
          </p>
        </div>

        <div className="space-y-4">
          {FEATURES.map((feature) => (
            <div key={feature.title} className="flex items-start gap-3">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary-foreground/10">
                <feature.icon className="size-4 text-primary-foreground" />
              </div>
              <div>
                <p className="text-sm font-semibold text-primary-foreground">
                  {feature.title}
                </p>
                <p className="text-sm text-primary-foreground/60">
                  {feature.description}
                </p>
              </div>
            </div>
          ))}
        </div>

        <div className="flex items-center gap-3 text-xs font-medium tracking-wide text-primary-foreground/50">
          <span>99.99% uptime</span>
          <span className="size-1 rounded-full bg-primary-foreground/30" />
          <span>10M+ messages/day</span>
          <span className="size-1 rounded-full bg-primary-foreground/30" />
          <span>6 channels</span>
        </div>
      </div>

      <div className="relative">
        <div className="flex items-center gap-4">
          <div className="h-px flex-1 bg-primary-foreground/20" />
          <span className="text-xs font-semibold tracking-widest text-primary-foreground/60">
            JOIN 10,000+ TEAMS
          </span>
          <div className="h-px flex-1 bg-primary-foreground/20" />
        </div>
        <div className="mt-5 grid grid-cols-4 gap-x-8 gap-y-3">
          {CLIENT_LOGOS.map((logo) => (
            <span
              key={logo}
              className="text-center text-sm font-semibold tracking-wide text-primary-foreground/40"
            >
              {logo}
            </span>
          ))}
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
