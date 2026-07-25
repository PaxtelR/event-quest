"use client";

import { getBase58Decoder } from "@solana/kit";
import { useSelectedWalletAccount, useSignAndSendTransaction } from "@solana/react";
import type { UiWalletAccount } from "@wallet-standard/react";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useCallback, useEffect, useRef, useState } from "react";

import { usePrepareCheckIn, useSubmitCheckIn, useValidateQr } from "@/hooks/useCheckIn";
import { useQrScanner } from "@/hooks/useQrScanner";
import { messages } from "@/i18n/en-US";
import { ApiError } from "@/lib/api";
import { DEVNET_CHAIN } from "@/lib/wallet";
import { extractQrToken } from "@/lib/qrToken";

// spec §15.5's exact state machine.
type Stage =
  | "IDLE"
  | "REQUESTING_CAMERA"
  | "SCANNING"
  | "VALIDATING_QR"
  | "PREPARING_TRANSACTION"
  | "AWAITING_WALLET_SIGNATURE"
  | "SUBMITTING"
  | "ERROR";

const STAGE_LABEL: Partial<Record<Stage, string>> = {
  VALIDATING_QR: messages.checkIn.validatingQr,
  PREPARING_TRANSACTION: messages.checkIn.preparingTransaction,
  AWAITING_WALLET_SIGNATURE: messages.checkIn.awaitingSignature,
  SUBMITTING: messages.checkIn.submitting,
};

function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function CheckInFlow({ account }: { account: UiWalletAccount }) {
  const router = useRouter();
  const searchParams = useSearchParams();

  const [stage, setStage] = useState<Stage>("IDLE");
  const [error, setError] = useState<string | null>(null);
  const [pastedLink, setPastedLink] = useState("");
  const runIdRef = useRef(0);

  const validateQr = useValidateQr();
  const prepareCheckIn = usePrepareCheckIn();
  const submitCheckIn = useSubmitCheckIn();
  const signAndSendTransaction = useSignAndSendTransaction(account, DEVNET_CHAIN);

  const runCheckIn = useCallback(
    async (rawToken: string) => {
      const runId = ++runIdRef.current;
      const token = extractQrToken(rawToken);
      if (!token) return;

      try {
        setError(null);
        setStage("VALIDATING_QR");
        const grant = await validateQr.mutateAsync({ qrToken: token, wallet: account.address });
        if (runIdRef.current !== runId) return;

        setStage("PREPARING_TRANSACTION");
        const prepared = await prepareCheckIn.mutateAsync(grant.grantId);
        if (runIdRef.current !== runId) return;

        setStage("AWAITING_WALLET_SIGNATURE");
        const transactionBytes = base64ToBytes(prepared.transaction);
        const { signature } = await signAndSendTransaction({ transaction: transactionBytes });
        if (runIdRef.current !== runId) return;
        const signatureBase58 = getBase58Decoder().decode(signature);

        setStage("SUBMITTING");
        await submitCheckIn.mutateAsync({ grantId: grant.grantId, signature: signatureBase58 });
        if (runIdRef.current !== runId) return;

        router.push(`/check-in/result?grantId=${grant.grantId}`);
      } catch (caught) {
        if (runIdRef.current !== runId) return;
        setStage("ERROR");
        setError(caught instanceof ApiError ? caught.message : messages.common.somethingWentWrong);
      }
    },
    [account.address, validateQr, prepareCheckIn, submitCheckIn, signAndSendTransaction, router],
  );

  const scanning = stage === "REQUESTING_CAMERA" || stage === "SCANNING";
  const { videoRef, status: cameraStatus } = useQrScanner(scanning, (data) => {
    void runCheckIn(data);
  });

  // A native camera app opening `/check-in?token=...` skips scanning
  // entirely — spec's QR encodes this page's own URL for exactly this.
  // A ref (not state) tracks whether it's been handled: it's a guard for
  // the effect below, not something the UI needs to re-render on.
  const tokenFromUrl = searchParams.get("token");
  const handledUrlTokenRef = useRef(false);
  useEffect(() => {
    if (tokenFromUrl && !handledUrlTokenRef.current && stage === "IDLE") {
      handledUrlTokenRef.current = true;
      void runCheckIn(tokenFromUrl);
    }
  }, [tokenFromUrl, stage, runCheckIn]);

  const isBusy = stage !== "IDLE" && stage !== "ERROR";

  return (
    <main className="mx-auto flex w-full max-w-md flex-1 flex-col items-center px-4 py-10 text-center sm:px-6">
      <h1 className="text-2xl font-semibold text-text-primary">{messages.checkIn.pageTitle}</h1>

      {stage === "IDLE" && (
        <div className="mt-8 flex w-full flex-col items-center gap-6">
          <button
            type="button"
            onClick={() => setStage("REQUESTING_CAMERA")}
            className="min-h-11 w-full rounded-lg bg-accent px-6 py-3 text-sm font-semibold text-accent-foreground transition-colors hover:bg-accent-hover active:bg-accent-active"
          >
            {messages.checkIn.scanQrCode}
          </button>

          <form
            className="flex w-full flex-col gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              void runCheckIn(pastedLink);
            }}
          >
            <label className="flex flex-col gap-1.5 text-left">
              <span className="text-xs text-text-secondary">{messages.checkIn.pasteLinkLabel}</span>
              <input
                value={pastedLink}
                onChange={(event) => setPastedLink(event.target.value)}
                placeholder={messages.checkIn.pasteLinkPlaceholder}
                className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
              />
            </label>
            <button
              type="submit"
              disabled={!pastedLink}
              className="min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-4 py-2 text-sm font-semibold text-text-primary hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-60"
            >
              {messages.checkIn.pasteLinkSubmit}
            </button>
          </form>
        </div>
      )}

      {scanning && (
        <div className="mt-8 flex w-full flex-col items-center gap-4">
          <div className="relative w-full overflow-hidden rounded-2xl border-2 border-accent bg-black">
            {/* A live camera preview, not pre-recorded media — no captions apply. */}
            <video ref={videoRef} muted playsInline className="aspect-square w-full object-cover" />
          </div>
          {cameraStatus === "camera-unavailable" ? (
            <p className="text-sm text-error" role="alert">
              {messages.checkIn.cameraUnavailable}
            </p>
          ) : (
            <p className="text-sm text-text-secondary" role="status">
              {cameraStatus === "requesting-camera"
                ? `${messages.checkIn.requestingCamera}…`
                : messages.checkIn.scanInstruction}
            </p>
          )}
        </div>
      )}

      {isBusy && !scanning && (
        <div className="mt-12 flex flex-col items-center gap-3" role="status">
          <span
            aria-hidden
            className="h-8 w-8 animate-spin rounded-full border-2 border-border-strong border-t-accent motion-reduce:animate-none"
          />
          <p className="text-sm text-text-secondary">{STAGE_LABEL[stage]}…</p>
        </div>
      )}

      {stage === "ERROR" && (
        <div className="mt-10 flex flex-col items-center gap-4">
          <p className="text-sm text-error" role="alert">
            {error}
          </p>
          <button
            type="button"
            onClick={() => {
              setStage("IDLE");
              handledUrlTokenRef.current = false;
              setError(null);
            }}
            className="min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-5 py-2 text-sm font-semibold text-text-primary hover:bg-surface-hover"
          >
            {messages.checkIn.tryAgain}
          </button>
        </div>
      )}
    </main>
  );
}

function CheckInGate() {
  const [account] = useSelectedWalletAccount();

  if (!account) {
    return (
      <main className="mx-auto flex w-full max-w-md flex-1 flex-col items-center justify-center px-4 py-20 text-center">
        <p className="text-sm text-text-secondary">{messages.checkIn.connectWalletFirst}</p>
      </main>
    );
  }

  return <CheckInFlow account={account} />;
}

export default function CheckInPage() {
  return (
    <Suspense
      fallback={
        <p className="p-10 text-center text-sm text-text-secondary" role="status">
          {messages.common.loading}…
        </p>
      }
    >
      <CheckInGate />
    </Suspense>
  );
}
