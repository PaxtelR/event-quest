"use client";

import type { UiWalletAccount } from "@wallet-standard/react";
import Link from "next/link";
import { useParams } from "next/navigation";
import { useState } from "react";

import { RequireAuth } from "@/components/RequireAuth";
import { StatusBadge } from "@/components/StatusBadge";
import {
  useCheckpointLifecycleAction,
  useCheckpoints,
  useCreateCheckpoint,
} from "@/hooks/useCheckpoints";
import { messages } from "@/i18n/en-US";
import { ApiError } from "@/lib/api";
import { createCheckpointSchema } from "@/lib/validation";
import type { CheckpointDto } from "@/types/api";

function CreateCheckpointForm({ eventId }: { eventId: string }) {
  const createCheckpoint = useCreateCheckpoint(eventId);
  const [name, setName] = useState("");
  const [points, setPoints] = useState(10);
  const [opensAt, setOpensAt] = useState("");
  const [closesAt, setClosesAt] = useState("");
  const [validationError, setValidationError] = useState<string | null>(null);

  return (
    <form
      className="flex flex-col gap-4 rounded-xl border border-border bg-surface p-5"
      onSubmit={(event) => {
        event.preventDefault();
        setValidationError(null);

        const result = createCheckpointSchema.safeParse({ name, points, opensAt, closesAt });
        if (!result.success) {
          setValidationError(result.error.issues[0]?.message ?? messages.checkpoints.form.createFailed);
          return;
        }

        createCheckpoint.mutate(
          {
            ...result.data,
            opensAt: new Date(result.data.opensAt).toISOString(),
            closesAt: new Date(result.data.closesAt).toISOString(),
          },
          {
            onSuccess: () => {
              setName("");
              setOpensAt("");
              setClosesAt("");
            },
          },
        );
      }}
    >
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-text-primary">{messages.checkpoints.form.name}</span>
          <input
            required
            value={name}
            onChange={(event) => setName(event.target.value)}
            className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-text-primary">{messages.checkpoints.form.points}</span>
          <input
            required
            type="number"
            min={1}
            value={points}
            onChange={(event) => setPoints(Number(event.target.value))}
            className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-text-primary">{messages.checkpoints.form.opensAt}</span>
          <input
            required
            type="datetime-local"
            value={opensAt}
            onChange={(event) => setOpensAt(event.target.value)}
            className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
          />
        </label>
        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-text-primary">{messages.checkpoints.form.closesAt}</span>
          <input
            required
            type="datetime-local"
            value={closesAt}
            onChange={(event) => setClosesAt(event.target.value)}
            className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
          />
        </label>
      </div>

      {(validationError || createCheckpoint.isError) && (
        <p className="text-sm text-error" role="alert">
          {validationError ??
            (createCheckpoint.error instanceof ApiError
              ? createCheckpoint.error.message
              : messages.checkpoints.form.createFailed)}
        </p>
      )}

      <button
        type="submit"
        disabled={createCheckpoint.isPending}
        className="min-h-11 self-start rounded-lg bg-accent px-5 py-2 text-sm font-semibold text-accent-foreground transition-colors hover:bg-accent-hover active:bg-accent-active disabled:cursor-not-allowed disabled:opacity-60"
      >
        {createCheckpoint.isPending ? `${messages.checkpoints.form.creating}…` : messages.checkpoints.addCheckpoint}
      </button>
    </form>
  );
}

function CheckpointRow({ checkpoint, account }: { checkpoint: CheckpointDto; account: UiWalletAccount }) {
  const activate = useCheckpointLifecycleAction(checkpoint.id, "activate", account);
  const pause = useCheckpointLifecycleAction(checkpoint.id, "pause", account);

  return (
    <div className="flex flex-col gap-3 rounded-xl border border-border bg-surface p-4 sm:flex-row sm:items-center sm:justify-between">
      <div>
        <p className="text-sm font-semibold text-text-primary">{checkpoint.name}</p>
        <p className="text-xs text-text-disabled">
          {checkpoint.points} {messages.events.points}
        </p>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <StatusBadge status={checkpoint.status} />
        {checkpoint.status !== "active" && (
          <button
            type="button"
            onClick={() => activate.mutate()}
            disabled={activate.isPending}
            className="min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-3 py-1.5 text-xs font-semibold text-text-primary hover:bg-surface-hover disabled:opacity-60"
          >
            {activate.isPending ? `${messages.checkpoints.lifecycle.activating}…` : messages.checkpoints.lifecycle.activate}
          </button>
        )}
        {checkpoint.status === "active" && (
          <button
            type="button"
            onClick={() => pause.mutate()}
            disabled={pause.isPending}
            className="min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-3 py-1.5 text-xs font-semibold text-text-primary hover:bg-surface-hover disabled:opacity-60"
          >
            {pause.isPending ? `${messages.checkpoints.lifecycle.pausing}…` : messages.checkpoints.lifecycle.pause}
          </button>
        )}
        <Link
          href={`/admin/checkpoints/${checkpoint.id}/display`}
          className="min-h-11 content-center rounded-lg border border-accent/50 px-3 py-1.5 text-xs font-semibold text-accent hover:bg-accent-soft"
        >
          {messages.checkpoints.openDisplay}
        </Link>
      </div>
    </div>
  );
}

function CheckpointsList({ eventId, account }: { eventId: string; account: UiWalletAccount }) {
  const checkpoints = useCheckpoints(eventId);

  return (
    <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-10 sm:px-6">
      <Link href={`/admin/events/${eventId}`} className="text-sm text-text-secondary hover:text-text-primary">
        ← {messages.admin.myEvents}
      </Link>
      <h1 className="mt-4 text-2xl font-semibold text-text-primary">{messages.checkpoints.pageTitle}</h1>

      <div className="mt-6 flex flex-col gap-4">
        {checkpoints.isLoading && (
          <p className="text-sm text-text-secondary" role="status">
            {messages.common.loading}…
          </p>
        )}
        {checkpoints.isError && (
          <p className="text-sm text-error" role="alert">
            {messages.checkpoints.loadFailed}
          </p>
        )}
        {checkpoints.data?.length === 0 && (
          <p className="rounded-xl border border-border bg-surface px-4 py-6 text-center text-sm text-text-secondary">
            {messages.checkpoints.noCheckpointsYet}
          </p>
        )}
        {checkpoints.data?.map((checkpoint) => (
          <CheckpointRow key={checkpoint.id} checkpoint={checkpoint} account={account} />
        ))}
      </div>

      <h2 className="mt-8 text-sm font-semibold text-text-primary">{messages.checkpoints.addCheckpoint}</h2>
      <div className="mt-3">
        <CreateCheckpointForm eventId={eventId} />
      </div>
    </main>
  );
}

export default function EventCheckpointsPage() {
  const params = useParams<{ eventId: string }>();
  return (
    <RequireAuth>{(account) => <CheckpointsList eventId={params.eventId} account={account} />}</RequireAuth>
  );
}
