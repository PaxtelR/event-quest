"use client";

import Link from "next/link";
import { useParams } from "next/navigation";

import { RequireAuth } from "@/components/RequireAuth";
import { useCheckpoints } from "@/hooks/useCheckpoints";
import { useEventParticipants } from "@/hooks/useParticipants";
import { messages } from "@/i18n/en-US";

function StatCard({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-xl border border-border bg-surface p-5">
      <p className="text-2xl font-semibold text-text-primary">{value}</p>
      <p className="mt-1 text-xs text-text-secondary">{label}</p>
    </div>
  );
}

function AnalyticsContent({ eventId }: { eventId: string }) {
  const participants = useEventParticipants(eventId);
  const checkpoints = useCheckpoints(eventId);

  const isLoading = participants.isLoading || checkpoints.isLoading;
  const isError = participants.isError || checkpoints.isError;

  const totalParticipants = participants.data?.length ?? 0;
  const totalPoints = participants.data?.reduce((sum, p) => sum + p.points, 0) ?? 0;
  const totalCheckins = participants.data?.reduce((sum, p) => sum + p.checkinCount, 0) ?? 0;
  const completedCount = participants.data?.filter((p) => p.completed).length ?? 0;

  return (
    <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-10 sm:px-6">
      <Link href={`/admin/events/${eventId}`} className="text-sm text-text-secondary hover:text-text-primary">
        ← {messages.admin.myEvents}
      </Link>
      <h1 className="mt-4 text-2xl font-semibold text-text-primary">{messages.analytics.pageTitle}</h1>

      {isLoading && (
        <p className="mt-6 text-sm text-text-secondary" role="status">
          {messages.common.loading}…
        </p>
      )}
      {isError && (
        <p className="mt-6 text-sm text-error" role="alert">
          {messages.analytics.loadFailed}
        </p>
      )}

      {!isLoading && !isError && (
        <div className="mt-6 grid grid-cols-2 gap-4 sm:grid-cols-3">
          <StatCard label={messages.analytics.totalParticipants} value={totalParticipants} />
          <StatCard label={messages.analytics.totalCheckpoints} value={checkpoints.data?.length ?? 0} />
          <StatCard label={messages.analytics.totalPoints} value={totalPoints} />
          <StatCard label={messages.analytics.totalCheckins} value={totalCheckins} />
          <StatCard label={messages.analytics.completedCount} value={completedCount} />
        </div>
      )}
    </main>
  );
}

export default function EventAnalyticsPage() {
  const params = useParams<{ eventId: string }>();
  return <RequireAuth>{() => <AnalyticsContent eventId={params.eventId} />}</RequireAuth>;
}
