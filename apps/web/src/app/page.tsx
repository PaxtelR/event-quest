import Link from "next/link";

import { messages } from "@/i18n/en-US";

export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col items-center justify-center px-6 py-24 text-center">
      <span className="mb-6 rounded-full border border-border-strong bg-surface px-4 py-1.5 text-xs font-medium tracking-wide text-accent uppercase">
        {messages.common.appName}
      </span>
      <h1 className="max-w-2xl text-4xl font-semibold text-text-primary sm:text-5xl">
        {messages.home.tagline}
      </h1>
      <p className="mt-6 max-w-xl text-base text-text-secondary">{messages.home.description}</p>
      <div className="mt-10 flex flex-col gap-3 sm:flex-row">
        <Link
          href="/events"
          className="rounded-lg bg-accent px-6 py-3 text-sm font-semibold text-accent-foreground transition-colors hover:bg-accent-hover active:bg-accent-active"
        >
          {messages.home.browseEvents}
        </Link>
        <Link
          href="/admin"
          className="rounded-lg border border-border-strong bg-surface-elevated px-6 py-3 text-sm font-semibold text-text-primary transition-colors hover:bg-surface-hover"
        >
          {messages.home.goToAdmin}
        </Link>
      </div>
    </main>
  );
}
