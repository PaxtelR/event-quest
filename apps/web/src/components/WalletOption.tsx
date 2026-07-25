"use client";

import { useConnect } from "@wallet-standard/react";
import type { UiWallet, UiWalletAccount } from "@wallet-standard/react";

/**
 * One row in the wallet picker. `useConnect` is called once per wallet
 * here (one hook call per mounted component instance keeps this within
 * the rules of hooks, unlike calling it conditionally in a loop).
 */
export function WalletOption({
  wallet,
  onConnected,
}: {
  wallet: UiWallet;
  onConnected: (account: UiWalletAccount) => void;
}) {
  const [isConnecting, connect] = useConnect(wallet);

  return (
    <button
      type="button"
      role="menuitem"
      disabled={isConnecting}
      onClick={async () => {
        const accounts = await connect();
        const account = accounts[0];
        if (account) {
          onConnected(account);
        }
      }}
      className="flex min-h-11 w-full items-center gap-3 rounded-md px-3 py-2 text-left text-sm text-text-primary transition-colors hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-60"
    >
      {wallet.icon ? (
        // Wallet icons are small data: URIs the wallet itself supplies,
        // not an optimizable remote asset — next/image doesn't apply.
        // eslint-disable-next-line @next/next/no-img-element
        <img src={wallet.icon} alt="" aria-hidden className="h-5 w-5 rounded" />
      ) : (
        <span aria-hidden className="h-5 w-5 rounded bg-surface-elevated" />
      )}
      <span className="flex-1">{wallet.name}</span>
    </button>
  );
}
