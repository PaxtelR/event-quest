"use client";

import type { UiWalletAccount } from "@wallet-standard/react";
import Link from "next/link";
import { useParams } from "next/navigation";

import { RequireAuth } from "@/components/RequireAuth";
import { StatusBadge } from "@/components/StatusBadge";
import { useEvent, useEventLifecycleAction } from "@/hooks/useEvents";
import { messages } from "@/i18n/en-US";
import { explorerAddressUrl } from "@/lib/explorer";
import type { EventDto } from "@/types/api";

function LifecycleButton({
  label,
  pendingLabel,
  action,
}: {
  label: string;
  pendingLabel: string;
  action: ReturnType<typeof useEventLifecycleAction>;
}) {
  return (
    <div className="flex flex-col gap-1">
      <button
        type="button"
        onClick={() => action.mutate()}
        disabled={action.isPending}
        className="min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-4 py-2 text-sm font-semibold text-text-primary transition-colors hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-60"
      >
        {action.isPending ? `${pendingLabel}…` : label}
      </button>
      {action.isError && (
        <p className="text-xs text-error" role="alert">
          {messages.admin.lifecycle.actionFailed}
        </p>
      )}
    </div>
  );
}

function EventLifecycleActions({ event, account }: { event: EventDto; account: UiWalletAccount }) {
  const publish = useEventLifecycleAction(event.id, "publish", account);
  const pause = useEventLifecycleAction(event.id, "pause", account);
  const finish = useEventLifecycleAction(event.id, "finish", account);

  const canPublish = event.status === "draft" || event.status === "paused";
  const canPause = event.status === "active";
  const canFinish = event.status === "active" || event.status === "paused";

  if (!canPublish && !canPause && !canFinish) {
    return null;
  }

  return (
    <div className="mt-6 flex flex-wrap gap-3">
      {canPublish && (
        <LifecycleButton
          label={messages.admin.lifecycle.publish}
          pendingLabel={messages.admin.lifecycle.publishing}
          action={publish}
        />
      )}
      {canPause && (
        <LifecycleButton
          label={messages.admin.lifecycle.pause}
          pendingLabel={messages.admin.lifecycle.pausing}
          action={pause}
        />
      )}
      {canFinish && (
        <LifecycleButton
          label={messages.admin.lifecycle.finish}
          pendingLabel={messages.admin.lifecycle.finishing}
          action={finish}
        />
      )}
    </div>
  );
}

function EventDetail({ eventId, account }: { eventId: string; account: UiWalletAccount }) {
  const event = useEvent(eventId);

  if (event.isLoading) {
    return (
      <p className="p-10 text-center text-sm text-text-secondary" role="status">
        {messages.common.loading}…
      </p>
    );
  }

  if (!event.data) {
    return <p className="p-10 text-center text-sm text-error">{messages.events.notFound}</p>;
  }

  const data = event.data;

  return (
    <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-10 sm:px-6">
      <Link href="/admin/events" className="text-sm text-text-secondary hover:text-text-primary">
        ← {messages.admin.myEvents}
      </Link>

      <div className="mt-4 flex items-center justify-between gap-3">
        <h1 className="text-2xl font-semibold text-text-primary">{data.name}</h1>
        <StatusBadge status={data.status} />
      </div>

      {data.description && <p className="mt-3 text-sm text-text-secondary">{data.description}</p>}

      <div className="mt-6 flex flex-wrap gap-3 text-sm">
        <Link href={`/events/${data.slug}`} className="text-accent hover:text-accent-hover">
          {messages.admin.viewPublicPage}
        </Link>
        <Link
          href={`/admin/events/${data.id}/checkpoints`}
          className="text-accent hover:text-accent-hover"
        >
          {messages.admin.manageCheckpoints}
        </Link>
        <Link
          href={`/admin/events/${data.id}/participants`}
          className="text-accent hover:text-accent-hover"
        >
          {messages.participants.pageTitle}
        </Link>
        <Link
          href={`/admin/events/${data.id}/analytics`}
          className="text-accent hover:text-accent-hover"
        >
          {messages.analytics.pageTitle}
        </Link>
      </div>

      <EventLifecycleActions event={data} account={account} />

      <dl className="mt-8 rounded-xl border border-border bg-surface p-5">
        <dt className="text-xs text-text-disabled">{messages.admin.lifecycle.onchainAddress}</dt>
        <dd className="mt-1 font-mono text-sm">
          {data.onchainEventAddress ? (
            <a
              href={explorerAddressUrl(data.onchainEventAddress)}
              target="_blank"
              rel="noreferrer"
              className="text-accent hover:text-accent-hover"
            >
              {data.onchainEventAddress}
            </a>
          ) : (
            <span className="text-text-disabled">{messages.admin.lifecycle.notYetProvisioned}</span>
          )}
        </dd>
      </dl>
    </main>
  );
}

export default function AdminEventDetailPage() {
  const params = useParams<{ eventId: string }>();
  return <RequireAuth>{(account) => <EventDetail eventId={params.eventId} account={account} />}</RequireAuth>;
}
