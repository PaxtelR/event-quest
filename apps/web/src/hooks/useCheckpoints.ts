"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { UiWalletAccount } from "@wallet-standard/react";

import { apiFetch } from "@/lib/api";
import type { CheckpointDto } from "@/types/api";

import { useOnChainAction } from "./useOnChainAction";

export function useCheckpoints(eventId: string | undefined) {
  return useQuery({
    queryKey: ["checkpoints", eventId],
    queryFn: () => apiFetch<CheckpointDto[]>(`/events/${eventId}/checkpoints`),
    enabled: Boolean(eventId),
  });
}

export function useCheckpoint(checkpointId: string | undefined) {
  return useQuery({
    queryKey: ["checkpoints", "byId", checkpointId],
    queryFn: () => apiFetch<CheckpointDto>(`/checkpoints/${checkpointId}`),
    enabled: Boolean(checkpointId),
  });
}

export type CreateCheckpointInput = {
  name: string;
  description?: string;
  points: number;
  rotationSeconds?: number;
  opensAt: string;
  closesAt: string;
};

export function useCreateCheckpoint(eventId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateCheckpointInput) =>
      apiFetch<CheckpointDto>(`/events/${eventId}/checkpoints`, { method: "POST", json: input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["checkpoints", eventId] });
    },
  });
}

/**
 * Activate/pause need a real organizer-signed on-chain transaction — see
 * apps/api/src/onchain.rs and apps/api/src/checkpoints/mod.rs's header
 * comment. Same `useOnChainAction` pattern events' lifecycle actions use.
 */
export function useCheckpointLifecycleAction(
  checkpointId: string,
  action: "activate" | "pause",
  account: UiWalletAccount,
) {
  return useOnChainAction<CheckpointDto>({
    account,
    preparePath: `/checkpoints/${checkpointId}/${action}/prepare-transaction`,
    submittedPath: `/checkpoints/${checkpointId}/${action}/submitted`,
    invalidateKeys: [["checkpoints", "byId", checkpointId], ["checkpoints"]],
  });
}
