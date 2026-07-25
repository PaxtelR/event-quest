"use client";

import Link from "next/link";
import { useParams } from "next/navigation";

import { RequireAuth } from "@/components/RequireAuth";
import { useEventParticipants } from "@/hooks/useParticipants";
import { messages } from "@/i18n/en-US";
import { truncateAddress } from "@/lib/wallet";

function ParticipantsList({ eventId }: { eventId: string }) {
  const participants = useEventParticipants(eventId);

  return (
    <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-10 sm:px-6">
      <Link href={`/admin/events/${eventId}`} className="text-sm text-text-secondary hover:text-text-primary">
        ← {messages.admin.myEvents}
      </Link>
      <h1 className="mt-4 text-2xl font-semibold text-text-primary">{messages.participants.pageTitle}</h1>

      <div className="mt-6 flex flex-col gap-3">
        {participants.isLoading && (
          <p className="text-sm text-text-secondary" role="status">
            {messages.common.loading}…
          </p>
        )}
        {participants.isError && (
          <p className="text-sm text-error" role="alert">
            {messages.participants.loadFailed}
          </p>
        )}
        {participants.data?.length === 0 && (
          <p className="rounded-xl border border-border bg-surface px-4 py-10 text-center text-sm text-text-secondary">
            {messages.participants.empty}
          </p>
        )}
        {participants.data?.map((participant) => (
          <div
            key={participant.participantId}
            className="flex items-center justify-between gap-3 rounded-xl border border-border bg-surface p-4"
          >
            <div>
              <p className="font-mono text-sm text-text-primary" title={participant.walletAddress}>
                {truncateAddress(participant.walletAddress)}
              </p>
              <p className="text-xs text-text-disabled">
                {participant.points} {messages.participants.points} · {participant.checkinCount}{" "}
                {messages.participants.checkins}
              </p>
            </div>
            {participant.completed && (
              <span className="rounded-full border border-accent/40 bg-accent-soft px-2.5 py-0.5 text-xs font-medium text-accent">
                {messages.participants.completed}
              </span>
            )}
          </div>
        ))}
      </div>
    </main>
  );
}

export default function EventParticipantsPage() {
  const params = useParams<{ eventId: string }>();
  return <RequireAuth>{() => <ParticipantsList eventId={params.eventId} />}</RequireAuth>;
}
