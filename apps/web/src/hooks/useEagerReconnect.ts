"use client";

import { getWalletFeature, useWallets } from "@wallet-standard/react";
import { useEffect, useRef } from "react";

import { supportsEventQuest, walletStateSync } from "@/lib/wallet";

// `SelectedWalletAccountContextProvider` (from @solana/react) only ever
// *matches* a previously-selected account against whatever accounts a
// wallet already discloses — it never itself asks a wallet to reconnect.
// Real wallet extensions (Brave Wallet, Phantom, Backpack) don't disclose
// any accounts to a fresh page load without an explicit `connect()` call,
// even for a site the user already authorized — which is why, without
// this, a refresh or a new tab always required clicking "Connect wallet"
// and re-picking the wallet again, even though the session cookie (and
// the previously-selected wallet's storage key) both survived just fine.
//
// This calls `standard:connect` with `{ silent: true }` on every
// registered wallet once, on mount, but only if something was selected in
// a previous session — the Wallet Standard spec defines `silent` as
// "reconnect without prompting if already authorized, otherwise resolve
// with nothing," so this never shows a wallet popup a user didn't ask
// for. Once a wallet's `standard:events` change notification fires with
// its now-populated accounts, `SelectedWalletAccountContextProvider`'s own
// reactive matching picks the selection back up — this hook only needs to
// trigger that, not perform the match itself.
export function useEagerReconnect(): void {
  const wallets = useWallets();
  const attemptedRef = useRef(false);

  useEffect(() => {
    if (attemptedRef.current) return;
    if (!walletStateSync.getSelectedWallet()) return;
    if (wallets.length === 0) return;
    attemptedRef.current = true;

    for (const wallet of wallets) {
      if (!supportsEventQuest(wallet)) continue;
      if (!wallet.features.includes("standard:connect")) continue;
      try {
        const connectFeature = getWalletFeature(wallet, "standard:connect") as {
          connect: (input: { silent: boolean }) => Promise<unknown>;
        };
        void connectFeature.connect({ silent: true }).catch(() => {
          // Not previously authorized (or the wallet doesn't honor
          // `silent`) — nothing to do; the user still sees the normal
          // "Connect wallet" button.
        });
      } catch {
        // Feature lookup itself failed — same as above, no-op.
      }
    }
  }, [wallets]);
}
