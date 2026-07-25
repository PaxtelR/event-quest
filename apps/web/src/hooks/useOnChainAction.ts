"use client";

import { getBase58Decoder } from "@solana/kit";
import { useSignAndSendTransaction } from "@solana/react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import type { UiWalletAccount } from "@wallet-standard/react";

import { apiFetch } from "@/lib/api";
import { DEVNET_CHAIN } from "@/lib/wallet";

/** Mirrors apps/api/src/onchain.rs's `PrepareResponse<T>` JSON shape. */
export type PrepareResponse<T> = {
  requiresSignature: boolean;
  transaction?: string;
  network?: string;
  resource?: T;
};

function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

/**
 * Drives the shared organizer on-chain-action pattern (spec-adjacent:
 * apps/api/src/onchain.rs's `prepare-transaction` / `submitted` pair used
 * by events' publish/pause/finish and checkpoints' activate/pause): call
 * `preparePath`, and if it says a signature is required, have the
 * connected wallet sign and send the returned transaction itself, then
 * report the signature to `submittedPath`. If no signature was required,
 * the action was already applied off-chain and its resource is returned
 * directly.
 */
export function useOnChainAction<T>({
  account,
  preparePath,
  submittedPath,
  invalidateKeys,
}: {
  account: UiWalletAccount;
  preparePath: string;
  submittedPath: string;
  invalidateKeys: readonly (readonly unknown[])[];
}) {
  const signAndSendTransaction = useSignAndSendTransaction(account, DEVNET_CHAIN);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (): Promise<T> => {
      const prepared = await apiFetch<PrepareResponse<T>>(preparePath, { method: "POST" });

      if (!prepared.requiresSignature) {
        return prepared.resource as T;
      }

      const transactionBytes = base64ToBytes(prepared.transaction as string);
      const { signature } = await signAndSendTransaction({ transaction: transactionBytes });
      const signatureBase58 = getBase58Decoder().decode(signature);

      return apiFetch<T>(submittedPath, {
        method: "POST",
        json: { signature: signatureBase58 },
      });
    },
    onSuccess: () => {
      for (const key of invalidateKeys) {
        queryClient.invalidateQueries({ queryKey: key as unknown[] });
      }
    },
  });
}
