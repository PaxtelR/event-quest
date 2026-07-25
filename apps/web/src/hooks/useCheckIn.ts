"use client";

import { useMutation, useQuery } from "@tanstack/react-query";

import { apiFetch } from "@/lib/api";

export type CheckinGrant = {
  grantId: string;
  eventId: string;
  checkpointId: string;
  wallet: string;
  challengeHash: string;
  expiresAt: string;
};

export type PreparedCheckIn = {
  transaction: string;
  network: string;
  expiresAt: string;
  grantId: string;
};

export type CheckinStatus = {
  grantId: string;
  status: string;
  transactionSignature: string | null;
  failureCode: string | null;
  failureMessage: string | null;
  expiresAt: string;
};

const TERMINAL_STATUSES = new Set(["confirmed", "failed", "expired", "rejected"]);

export function useValidateQr() {
  return useMutation({
    mutationFn: (input: { qrToken: string; wallet: string }) =>
      apiFetch<CheckinGrant>("/check-ins/validate", { method: "POST", json: input }),
  });
}

export function usePrepareCheckIn() {
  return useMutation({
    mutationFn: (grantId: string) =>
      apiFetch<PreparedCheckIn>(`/check-ins/${grantId}/prepare-transaction`, { method: "POST" }),
  });
}

export function useSubmitCheckIn() {
  return useMutation({
    mutationFn: ({ grantId, signature }: { grantId: string; signature: string }) =>
      apiFetch<CheckinStatus>(`/check-ins/${grantId}/submitted`, {
        method: "POST",
        json: { signature },
      }),
  });
}

/** Polls until apps/indexer confirms the attendance (spec §16) or the
 * attempt reaches a terminal failure state. */
export function useCheckinStatus(grantId: string | undefined) {
  return useQuery({
    queryKey: ["check-ins", grantId],
    queryFn: () => apiFetch<CheckinStatus>(`/check-ins/${grantId}`),
    enabled: Boolean(grantId),
    refetchInterval: (query) => (TERMINAL_STATUSES.has(query.state.data?.status ?? "") ? false : 2000),
  });
}
