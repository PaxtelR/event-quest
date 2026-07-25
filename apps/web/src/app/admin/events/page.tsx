"use client";

import Link from "next/link";

import { RequireAuth } from "@/components/RequireAuth";
import { StatusBadge } from "@/components/StatusBadge";
import { useEvents } from "@/hooks/useEvents";
import { useMyOrganizations } from "@/hooks/useOrganizations";
import { messages } from "@/i18n/en-US";

function MyEvents() {
  const organizations = useMyOrganizations();
  const organizationId = organizations.data?.[0]?.id;
  const events = useEvents(organizationId);
  const isLoading = organizations.isLoading || (Boolean(organizationId) && events.isLoading);

  return (
    <main className="mx-auto w-full max-w-3xl flex-1 px-4 py-10 sm:px-6">
      <div className="flex items-center justify-between gap-4">
        <h1 className="text-2xl font-semibold text-text-primary">{messages.admin.myEvents}</h1>
        <Link
          href="/admin/events/new"
          className="min-h-11 rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-accent-foreground transition-colors hover:bg-accent-hover active:bg-accent-active content-center"
        >
          {messages.admin.createEvent}
        </Link>
      </div>

      <div className="mt-8 flex flex-col gap-4">
        {isLoading && (
          <p className="text-sm text-text-secondary" role="status">
            {messages.common.loading}…
          </p>
        )}

        {(organizations.isError || events.isError) && (
          <p className="text-sm text-error" role="alert">
            {messages.admin.loadFailed}
          </p>
        )}

        {!isLoading && (!organizationId || events.data?.length === 0) && (
          <div className="rounded-xl border border-border bg-surface px-4 py-10 text-center">
            <p className="text-sm text-text-secondary">{messages.admin.noEventsYet}</p>
            <Link
              href="/admin/events/new"
              className="mt-4 inline-block text-sm font-semibold text-accent hover:text-accent-hover"
            >
              {messages.admin.createFirstEvent}
            </Link>
          </div>
        )}

        {events.data?.map((event) => (
          <Link
            key={event.id}
            href={`/admin/events/${event.id}`}
            className="flex items-center justify-between gap-3 rounded-xl border border-border bg-surface p-5 transition-colors hover:bg-surface-hover"
          >
            <div>
              <p className="text-sm font-semibold text-text-primary">{event.name}</p>
              <p className="font-mono text-xs text-text-disabled">/{event.slug}</p>
            </div>
            <StatusBadge status={event.status} />
          </Link>
        ))}
      </div>
    </main>
  );
}

export default function AdminEventsPage() {
  return <RequireAuth>{() => <MyEvents />}</RequireAuth>;
}
