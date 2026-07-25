"use client";

import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { apiFetch } from "@/lib/api";

export type CurrentQrWindow = {
  token: string;
  qrUrl: string;
  issuedAt: string;
  expiresAt: string;
  remainingSeconds: number;
};

type DisplayTokenResponse = { displayAccessToken: string; expiresAt: string };

/** Authenticated once (spec §8.4) to get a token the public SSE/current-qr
 * routes accept without a wallet session — the checkpoint display is
 * typically a kiosk/TV with no browser session of its own. */
export function useDisplayToken(checkpointId: string) {
  return useQuery({
    queryKey: ["display-token", checkpointId],
    queryFn: () =>
      apiFetch<DisplayTokenResponse>(`/checkpoints/${checkpointId}/display-token`, { method: "POST" }),
    staleTime: 10 * 60 * 1000,
  });
}

type QrStreamState = {
  window: CurrentQrWindow | null;
  active: boolean;
  connected: boolean;
};

/**
 * Consumes the public `qr-stream` SSE endpoint (spec §15.4) — connection
 * status feeds the display's own connection indicator, and an "inactive"
 * event (checkpoint paused/closed) clears the current QR immediately
 * rather than ever showing a stale one during a disconnect.
 */
export function useQrStream(checkpointId: string, displayAccessToken: string | undefined): QrStreamState {
  const [state, setState] = useState<QrStreamState>({ window: null, active: true, connected: false });

  useEffect(() => {
    if (!displayAccessToken) return;

    const source = new EventSource(
      `/api/v1/public/checkpoints/${checkpointId}/qr-stream?displayAccessToken=${encodeURIComponent(displayAccessToken)}`,
    );

    source.addEventListener("open", () => setState((current) => ({ ...current, connected: true })));
    source.addEventListener("qr", (event) => {
      const window = JSON.parse((event as MessageEvent<string>).data) as CurrentQrWindow;
      setState({ window, active: true, connected: true });
    });
    source.addEventListener("inactive", () => {
      setState((current) => ({ ...current, window: null, active: false }));
    });
    source.onerror = () => setState((current) => ({ ...current, connected: false }));

    return () => source.close();
  }, [checkpointId, displayAccessToken]);

  return state;
}

function secondsUntil(expiresAt: string): number {
  return Math.max(0, Math.round((new Date(expiresAt).getTime() - Date.now()) / 1000));
}

/**
 * Ticks down to zero every 250ms while `expiresAt` is set. `Date.now()` is
 * only ever read from inside the interval's callback (an inherently async
 * context), never synchronously during render or at the top of the effect
 * body — reading the clock during render would make this hook impure, and
 * setting state synchronously in the effect body risks cascading renders.
 * Callers only display this value while `expiresAt` is actually set, so
 * there's nothing to reset when it becomes `undefined`.
 */
export function useCountdown(expiresAt: string | undefined): number {
  const [remaining, setRemaining] = useState(0);

  useEffect(() => {
    if (!expiresAt) return;
    const interval = setInterval(() => setRemaining(secondsUntil(expiresAt)), 250);
    return () => clearInterval(interval);
  }, [expiresAt]);

  return remaining;
}
