"use client";

import Link from "next/link";

import { RequireAuth } from "@/components/RequireAuth";
import { useMyEvents } from "@/hooks/useParticipants";
import { messages } from "@/i18n/en-US";

function PassportContent() {
  const myEvents = useMyEvents();

  return (
    <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-10 sm:px-6">
      <h1 className="text-2xl font-semibold text-text-primary">{messages.passport.pageTitle}</h1>

      <div className="mt-8 flex flex-col gap-4">
        {myEvents.isLoading && (
          <p className="text-sm text-text-secondary" role="status">
            {messages.common.loading}…
          </p>
        )}
        {myEvents.isError && (
          <p className="text-sm text-error" role="alert">
            {messages.passport.loadFailed}
          </p>
        )}
        {myEvents.data?.length === 0 && (
          <div className="rounded-xl border border-border bg-surface px-4 py-10 text-center">
            <p className="text-sm text-text-secondary">{messages.passport.empty}</p>
            <Link href="/events" className="mt-4 inline-block text-sm font-semibold text-accent hover:text-accent-hover">
              {messages.passport.browseEvents}
            </Link>
          </div>
        )}
        {myEvents.data?.map((progress) => (
          <Link
            key={progress.eventId}
            href={`/events/${progress.eventSlug}`}
            className="flex items-center justify-between gap-3 rounded-xl border border-border bg-surface p-5 transition-colors hover:bg-surface-hover"
          >
            <div>
              <p className="text-sm font-semibold text-text-primary">{progress.eventName}</p>
              <p className="font-mono text-xs text-text-disabled">
                {progress.points} {messages.passport.points} · {progress.checkinCount} {messages.passport.checkins}
              </p>
            </div>
            {progress.completed && (
              <span className="rounded-full border border-accent/40 bg-accent-soft px-2.5 py-0.5 text-xs font-medium text-accent">
                {messages.passport.completedBadge}
              </span>
            )}
          </Link>
        ))}
      </div>
    </main>
  );
}

export default function PassportPage() {
  return (
    <RequireAuth message={messages.passport.signInRequired}>{() => <PassportContent />}</RequireAuth>
  );
}
