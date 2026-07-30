import Link from 'next/link';

const features = [
  {
    title: 'Multi-Channel',
    desc: 'Email, SMS, push, and webhooks from a single API.',
    href: '/docs/core-concepts',
  },
  {
    title: 'Developer-First',
    desc: 'TypeScript, Python, Go, Rust, and Java SDKs.',
    href: '/docs/sdk',
  },
  {
    title: 'Reliable Delivery',
    desc: 'At-least-once guarantees with automatic retries.',
    href: '/docs/core-concepts/notifications',
  },
  {
    title: 'Event-Driven',
    desc: 'Trigger notifications from any event in your system.',
    href: '/docs/events',
  },
  {
    title: 'Template Engine',
    desc: 'Dynamic templates with variable interpolation.',
    href: '/docs/templates',
  },
  {
    title: 'Observability',
    desc: 'Full delivery logs, metrics, and tracing.',
    href: '/docs/analytics',
  },
];

export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col">
      <section className="flex flex-col items-center justify-center px-4 pt-24 pb-16 text-center">
        <div className="flex flex-col items-center gap-4">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="text-fd-primary">
            <path d="M22 12h-4l-3 9L9 3l-3 9H2" />
          </svg>
          <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">
            Build with Notifi
          </h1>
          <p className="max-w-[560px] text-lg text-fd-muted-foreground">
            Everything you need to build, manage, and scale notification workflows across any channel.
          </p>
        </div>
        <div className="mt-8 flex flex-wrap justify-center gap-3">
          <Link
            href="/docs/getting-started"
            className="inline-flex items-center rounded-lg bg-fd-primary px-5 py-2.5 text-sm font-medium text-fd-primary-foreground hover:opacity-90"
          >
            Get Started
          </Link>
          <Link
            href="/docs/api-reference"
            className="inline-flex items-center rounded-lg border border-fd-border px-5 py-2.5 text-sm font-medium hover:bg-fd-accent"
          >
            API Reference
          </Link>
          <Link
            href="/docs/sdk"
            className="inline-flex items-center rounded-lg border border-fd-border px-5 py-2.5 text-sm font-medium hover:bg-fd-accent"
          >
            SDKs & Libraries
          </Link>
        </div>
      </section>

      <section className="mx-auto grid max-w-5xl grid-cols-1 gap-4 px-4 pb-24 sm:grid-cols-2 lg:grid-cols-3">
        {features.map((f) => (
          <Link
            key={f.title}
            href={f.href}
            className="group rounded-lg border border-fd-border bg-fd-card p-5 transition-colors hover:border-fd-primary/50 hover:bg-fd-accent"
          >
            <h3 className="font-semibold text-fd-foreground group-hover:text-fd-primary">
              {f.title}
            </h3>
            <p className="mt-1 text-sm text-fd-muted-foreground">{f.desc}</p>
          </Link>
        ))}
      </section>
    </main>
  );
}
