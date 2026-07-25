"use client";

import Link from "next/link";
import { useParams } from "next/navigation";

import { StatusBadge } from "@/components/StatusBadge";
import { useCheckpoints } from "@/hooks/useCheckpoints";
import { useEventBySlug } from "@/hooks/useEvents";
import { messages } from "@/i18n/en-US";

function formatDateTime(value: string, timezone: string): string {
  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: timezone,
  }).format(new Date(value));
}

export default function EventDetailPage() {
  const params = useParams<{ slug: string }>();
  const event = useEventBySlug(params.slug);
  const checkpoints = useCheckpoints(event.data?.id);

  if (event.isLoading) {
    return (
      <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-10 sm:px-6">
        <p className="text-sm text-text-secondary" role="status">
          {messages.common.loading}…
        </p>
      </main>
    );
  }

  if (!event.data) {
    return (
      <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-10 text-center sm:px-6">
        <h1 className="text-xl font-semibold text-text-primary">{messages.events.notFound}</h1>
        <p className="mt-2 text-sm text-text-secondary">{messages.events.notFoundDescription}</p>
        <Link href="/events" className="mt-6 inline-block text-sm font-semibold text-accent hover:text-accent-hover">
          {messages.events.backToEvents}
        </Link>
      </main>
    );
  }

  return (
    <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-10 sm:px-6">
      <Link href="/events" className="text-sm text-text-secondary hover:text-text-primary">
        ← {messages.events.backToEvents}
      </Link>

      <div className="mt-4 flex items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold text-text-primary">{event.data.name}</h1>
        <StatusBadge status={event.data.status} />
      </div>

      {event.data.description && (
        <p className="mt-3 text-sm text-text-secondary">{event.data.description}</p>
      )}

      <dl className="mt-6 grid grid-cols-1 gap-4 rounded-xl border border-border bg-surface p-5 sm:grid-cols-2">
        <div>
          <dt className="text-xs text-text-disabled">Starts</dt>
          <dd className="font-mono text-sm text-text-primary">
            {formatDateTime(event.data.startsAt, event.data.timezone)}
          </dd>
        </div>
        <div>
          <dt className="text-xs text-text-disabled">Ends</dt>
          <dd className="font-mono text-sm text-text-primary">
            {formatDateTime(event.data.endsAt, event.data.timezone)}
          </dd>
        </div>
        {event.data.locationName && (
          <div className="sm:col-span-2">
            <dt className="text-xs text-text-disabled">Location</dt>
            <dd className="text-sm text-text-primary">{event.data.locationName}</dd>
          </div>
        )}
      </dl>

      <h2 className="mt-8 text-lg font-semibold text-text-primary">{messages.events.checkpoints}</h2>
      <div className="mt-3 flex flex-col gap-3">
        {checkpoints.data && checkpoints.data.length === 0 && (
          <p className="rounded-xl border border-border bg-surface px-4 py-6 text-center text-sm text-text-secondary">
            {messages.events.noCheckpoints}
          </p>
        )}
        {checkpoints.data?.map((checkpoint) => (
          <div
            key={checkpoint.id}
            className="flex items-center justify-between gap-3 rounded-xl border border-border bg-surface p-4"
          >
            <div>
              <p className="text-sm font-medium text-text-primary">{checkpoint.name}</p>
              <p className="text-xs text-text-disabled">
                {checkpoint.points} {messages.events.points}
              </p>
            </div>
            <StatusBadge status={checkpoint.status} />
          </div>
        ))}
      </div>
    </main>
  );
}
