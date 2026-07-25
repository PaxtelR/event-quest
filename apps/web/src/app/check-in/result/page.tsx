"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { Suspense } from "react";

import { useCheckinStatus } from "@/hooks/useCheckIn";
import { messages } from "@/i18n/en-US";
import { explorerTxUrl } from "@/lib/explorer";

const ERROR_STATUSES = new Set(["failed", "expired", "rejected"]);

function CheckInResultContent() {
  const searchParams = useSearchParams();
  const grantId = searchParams.get("grantId") ?? undefined;
  const status = useCheckinStatus(grantId);

  return (
    <main className="mx-auto flex w-full max-w-md flex-1 flex-col items-center justify-center gap-4 px-4 py-10 text-center sm:px-6">
      {status.isLoading && (
        <p className="text-sm text-text-secondary" role="status">
          {messages.common.loading}…
        </p>
      )}

      {status.data?.status === "confirmed" && (
        <>
          <span
            aria-hidden
            className="flex h-14 w-14 items-center justify-center rounded-full bg-success/10 text-2xl text-success"
          >
            ✓
          </span>
          <p className="text-lg font-semibold text-text-primary">{messages.checkIn.transactionConfirmed}</p>
        </>
      )}

      {status.data && !ERROR_STATUSES.has(status.data.status) && status.data.status !== "confirmed" && (
        <>
          <span
            aria-hidden
            className="h-8 w-8 animate-spin rounded-full border-2 border-border-strong border-t-accent motion-reduce:animate-none"
          />
          <p className="text-sm text-text-secondary" role="status">
            {messages.checkIn.confirming}…
          </p>
        </>
      )}

      {status.data && ERROR_STATUSES.has(status.data.status) && (
        <p className="text-sm text-error" role="alert">
          {status.data.failureMessage ?? messages.common.somethingWentWrong}
        </p>
      )}

      {status.data?.transactionSignature && (
        <a
          href={explorerTxUrl(status.data.transactionSignature)}
          target="_blank"
          rel="noreferrer"
          className="font-mono text-xs text-accent hover:text-accent-hover"
        >
          {messages.checkIn.viewTransaction} ↗
        </a>
      )}

      <Link
        href="/check-in"
        className="mt-4 min-h-11 content-center rounded-lg border border-border-strong bg-surface-elevated px-5 py-2 text-sm font-semibold text-text-primary hover:bg-surface-hover"
      >
        {messages.checkIn.scanAnother}
      </Link>
    </main>
  );
}

export default function CheckInResultPage() {
  return (
    <Suspense
      fallback={
        <p className="p-10 text-center text-sm text-text-secondary" role="status">
          {messages.common.loading}…
        </p>
      }
    >
      <CheckInResultContent />
    </Suspense>
  );
}
