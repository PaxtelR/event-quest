"use client";

import Link from "next/link";

import { StatusBadge } from "@/components/StatusBadge";
import { useEvents } from "@/hooks/useEvents";
import { messages } from "@/i18n/en-US";

function formatDateRange(startsAt: string, endsAt: string, timezone: string): string {
  const formatter = new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: timezone,
  });
  return `${formatter.format(new Date(startsAt))} – ${formatter.format(new Date(endsAt))}`;
}

export default function EventsPage() {
  const events = useEvents();

  return (
    <main className="mx-auto w-full max-w-3xl flex-1 px-4 py-10 sm:px-6">
      <h1 className="text-2xl font-semibold text-text-primary">{messages.events.pageTitle}</h1>
      <p className="mt-2 text-sm text-text-secondary">{messages.events.pageDescription}</p>

      <div className="mt-8 flex flex-col gap-4">
        {events.isLoading && (
          <p className="text-sm text-text-secondary" role="status">
            {messages.common.loading}…
          </p>
        )}

        {events.isError && (
          <p className="text-sm text-error" role="alert">
            {messages.events.loadFailed}
          </p>
        )}

        {events.data && events.data.length === 0 && (
          <p className="rounded-xl border border-border bg-surface px-4 py-8 text-center text-sm text-text-secondary">
            {messages.events.empty}
          </p>
        )}

        {events.data?.map((event) => (
          <Link
            key={event.id}
            href={`/events/${event.slug}`}
            className="flex flex-col gap-2 rounded-xl border border-border bg-surface p-5 transition-colors hover:bg-surface-hover"
          >
            <div className="flex items-center justify-between gap-3">
              <h2 className="text-base font-semibold text-text-primary">{event.name}</h2>
              <StatusBadge status={event.status} />
            </div>
            {event.locationName && <p className="text-sm text-text-secondary">{event.locationName}</p>}
            <p className="font-mono text-xs text-text-disabled">
              {formatDateRange(event.startsAt, event.endsAt, event.timezone)}
            </p>
          </Link>
        ))}
      </div>
    </main>
  );
}
