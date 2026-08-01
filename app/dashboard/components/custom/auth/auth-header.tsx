import { Bell, Quote } from "lucide-react";

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

export function AuthHeader() {
  return (
    <div className="relative hidden flex-col justify-between overflow-hidden bg-gradient-to-br from-primary via-primary/85 to-primary/60 p-8 lg:flex lg:w-1/2">
      <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_at_50%_25%,rgba(255,255,255,0.15),transparent_60%)]" />

      <div className="relative flex items-center gap-2">
        <div className="flex size-8 items-center justify-center rounded-lg bg-primary-foreground text-primary">
          <Bell className="size-4" />
        </div>
        <span className="text-lg font-bold text-primary-foreground">Notifi</span>
      </div>

      <div className="relative space-y-4">
        <h1 className="text-3xl font-bold tracking-tight text-primary-foreground lg:text-4xl">
          Revolutionize how you reach your users
        </h1>
        <p className="text-lg text-primary-foreground/70">
          Send notifications across every channel with one API.
          Enterprise-ready. Developer-first.
        </p>

        <div className="relative mt-8 rounded-2xl bg-primary-foreground/10 p-6 backdrop-blur-sm">
          <Quote className="absolute -top-3 left-4 size-8 text-primary-foreground/30" />
          <p className="leading-relaxed text-primary-foreground/90">
            Notifi has completely transformed how we notify our users. It&apos;s
            reliable, efficient, and ensures our releases are always top-notch.
          </p>
          <div className="mt-4 flex items-center gap-3">
            <div className="flex size-10 items-center justify-center rounded-full bg-primary-foreground/20 text-sm font-semibold text-primary-foreground">
              SM
            </div>
            <div>
              <p className="text-sm font-semibold text-primary-foreground">
                Sarah Mitchell
              </p>
              <p className="text-sm text-primary-foreground/60">
                Head of Engineering, DevCore
              </p>
            </div>
          </div>
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
        <div className="mt-6 grid grid-cols-4 gap-x-6 gap-y-4">
          {CLIENT_LOGOS.map((logo) => (
            <span
              key={logo}
              className="text-center text-sm font-semibold text-primary-foreground/50"
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
