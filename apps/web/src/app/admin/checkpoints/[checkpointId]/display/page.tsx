"use client";

import { useParams } from "next/navigation";
import { useState } from "react";

import { RequireAuth } from "@/components/RequireAuth";
import { RotatingQrCode } from "@/components/RotatingQrCode";
import { useCheckpoint } from "@/hooks/useCheckpoints";
import { useEvent } from "@/hooks/useEvents";
import { useCountdown, useDisplayToken, useQrStream } from "@/hooks/useQrDisplay";
import { messages } from "@/i18n/en-US";

function CheckpointDisplay({ checkpointId }: { checkpointId: string }) {
  const checkpoint = useCheckpoint(checkpointId);
  const event = useEvent(checkpoint.data?.eventId);
  const displayToken = useDisplayToken(checkpointId);
  const stream = useQrStream(checkpointId, displayToken.data?.displayAccessToken);
  const remaining = useCountdown(stream.window?.expiresAt);
  const [isFullscreen, setIsFullscreen] = useState(false);

  return (
    <main className="flex min-h-screen flex-1 flex-col items-center justify-center gap-6 bg-background px-6 py-12 text-center">
      <button
        type="button"
        onClick={() => {
          if (isFullscreen) {
            document.exitFullscreen?.();
          } else {
            document.documentElement.requestFullscreen?.();
          }
          setIsFullscreen((value) => !value);
        }}
        className="absolute top-4 right-4 min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-4 py-2 text-xs font-semibold text-text-primary hover:bg-surface-hover"
      >
        {isFullscreen ? messages.display.exitFullscreen : messages.display.fullscreen}
      </button>

      <div
        className="absolute top-4 left-4 flex items-center gap-2 text-xs text-text-secondary"
        role="status"
      >
        <span
          aria-hidden
          className={`h-2 w-2 rounded-full ${stream.connected ? "bg-success" : "bg-error"}`}
        />
        {stream.connected ? messages.display.connected : messages.display.reconnecting}
      </div>

      {event.data && <p className="text-lg font-medium text-text-secondary">{event.data.name}</p>}
      <h1 className="text-3xl font-semibold text-text-primary">{checkpoint.data?.name}</h1>

      {stream.active && stream.window ? (
        <>
          <RotatingQrCode value={stream.window.qrUrl} />
          <p className="text-base text-text-secondary">{messages.display.scanInstruction}</p>
          <p className="font-mono text-sm text-text-disabled" aria-live="polite">
            {messages.display.newCodeIn} {remaining} {messages.display.seconds}
          </p>
        </>
      ) : (
        <p className="rounded-xl border border-border bg-surface px-8 py-12 text-xl font-medium text-text-secondary">
          {messages.display.paused}
        </p>
      )}
    </main>
  );
}

export default function CheckpointDisplayPage() {
  const params = useParams<{ checkpointId: string }>();
  return <RequireAuth>{() => <CheckpointDisplay checkpointId={params.checkpointId} />}</RequireAuth>;
}
