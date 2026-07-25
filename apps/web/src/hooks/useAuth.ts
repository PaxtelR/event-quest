"use client";

import { useSignMessage } from "@solana/react";
import { getBase58Decoder } from "@solana/kit";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { UiWalletAccount } from "@wallet-standard/react";

import { apiFetch, ApiError } from "@/lib/api";

const AUTH_ME_QUERY_KEY = ["auth", "me"] as const;

export type Session = { wallet: string };

type NonceResponse = { nonce: string; message: string; expiresAt: string };

/**
 * The current session, or `null` when signed out. A 401 from `/auth/me` is
 * the expected "not signed in" response, not an error — it's translated to
 * `null` here so callers don't have to special-case it.
 */
export function useSession() {
  return useQuery<Session | null>({
    queryKey: AUTH_ME_QUERY_KEY,
    queryFn: async () => {
      try {
        return await apiFetch<Session>("/auth/me");
      } catch (error) {
        if (error instanceof ApiError && error.status === 401) {
          return null;
        }
        throw error;
      }
    },
    staleTime: 60_000,
  });
}

/**
 * Signs in as `account`: fetches a nonce, has the wallet sign the exact
 * message text returned (a plain Wallet Standard `signMessage` — see
 * apps/api/src/auth/message.rs's own doc comment: this is a closed-loop
 * message format the backend renders and re-parses itself, not full SIWS
 * ABNF, so `useSignIn`'s wallet-reformatted output would not verify here),
 * and posts the result to `/auth/verify`.
 */
export function useSignInWithWallet(account: UiWalletAccount) {
  const signMessage = useSignMessage(account);
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async () => {
      const { message } = await apiFetch<NonceResponse>("/auth/nonce", { method: "POST" });
      const { signature } = await signMessage({ message: new TextEncoder().encode(message) });
      const signatureBase58 = getBase58Decoder().decode(signature);
      return apiFetch<Session>("/auth/verify", {
        method: "POST",
        json: { wallet: account.address, message, signature: signatureBase58 },
      });
    },
    onSuccess: (session) => {
      queryClient.setQueryData(AUTH_ME_QUERY_KEY, session);
    },
  });
}

export function useSignOut() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => apiFetch<void>("/auth/logout", { method: "POST" }),
    onSuccess: () => {
      queryClient.setQueryData(AUTH_ME_QUERY_KEY, null);
    },
  });
}
