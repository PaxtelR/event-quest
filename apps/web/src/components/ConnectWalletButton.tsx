"use client";

import { useSelectedWalletAccount } from "@solana/react";
import { useWallets } from "@wallet-standard/react";
import type { UiWallet, UiWalletAccount } from "@wallet-standard/react";
import { useEffect, useRef, useState } from "react";

import { useSession, useSignInWithWallet, useSignOut } from "@/hooks/useAuth";
import { messages } from "@/i18n/en-US";
import { truncateAddress } from "@/lib/wallet";
import { getBackpackBrowseUrl, getPhantomBrowseUrl, getSolflareBrowseUrl } from "@/lib/walletDeepLinks";

import { WalletOption } from "./WalletOption";

export function ConnectWalletButton() {
  const [selectedAccount, setSelectedAccount, filteredWallets] = useSelectedWalletAccount();
  // `useWallets()` is only consulted for its emptiness — `filteredWallets`
  // (already narrowed to Devnet-capable wallets by `WalletProvider`) is
  // what the picker actually lists.
  const allWallets = useWallets();

  if (selectedAccount) {
    return (
      <AuthenticatedWalletMenu
        account={selectedAccount}
        onForget={() => setSelectedAccount(undefined)}
      />
    );
  }

  return (
    <WalletPicker
      wallets={filteredWallets}
      hasAnyWalletAtAll={allWallets.length > 0}
      onSelect={setSelectedAccount}
    />
  );
}

function WalletPicker({
  wallets,
  hasAnyWalletAtAll,
  onSelect,
}: {
  wallets: readonly UiWallet[];
  hasAnyWalletAtAll: boolean;
  onSelect: (account: UiWalletAccount) => void;
}) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    }
    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [open]);

  return (
    <div ref={containerRef} className="relative">
      <button
        type="button"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
        className="min-h-11 rounded-lg bg-accent px-4 py-2 text-sm font-semibold text-accent-foreground transition-colors hover:bg-accent-hover active:bg-accent-active"
      >
        {messages.common.connectWallet}
      </button>
      {open && (
        <div
          role="menu"
          aria-label={messages.wallet.chooseWallet}
          className="absolute right-0 z-20 mt-2 w-64 rounded-xl border border-border bg-surface-elevated p-1.5 shadow-lg"
        >
          {!hasAnyWalletAtAll ? (
            <div className="flex flex-col gap-2 p-3">
              <p className="text-sm text-text-secondary">{messages.wallet.noWalletsFound}</p>
              {/* Mobile wallet apps only inject a provider inside their own
                  in-app browser, never a phone's regular Safari/Chrome — a
                  QR-scanned check-in link opened there will never see a
                  connectable wallet no matter how long it waits. */}
              <p className="text-xs text-text-disabled">{messages.wallet.openInWalletHint}</p>
              <div className="flex flex-col gap-1.5">
                <a
                  href={getPhantomBrowseUrl(window.location.href, window.location.origin)}
                  className="min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-3 py-2 text-center text-sm font-semibold text-text-primary hover:bg-surface-hover"
                >
                  {messages.wallet.openInPhantom}
                </a>
                <a
                  href={getSolflareBrowseUrl(window.location.href, window.location.origin)}
                  className="min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-3 py-2 text-center text-sm font-semibold text-text-primary hover:bg-surface-hover"
                >
                  {messages.wallet.openInSolflare}
                </a>
                <a
                  href={getBackpackBrowseUrl(window.location.href, window.location.origin)}
                  className="min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-3 py-2 text-center text-sm font-semibold text-text-primary hover:bg-surface-hover"
                >
                  {messages.wallet.openInBackpack}
                </a>
              </div>
            </div>
          ) : wallets.length === 0 ? (
            <p className="p-3 text-sm text-text-secondary">{messages.devnet.walletNotOnDevnet}</p>
          ) : (
            wallets.map((wallet) => (
              <WalletOption
                key={wallet.name}
                wallet={wallet}
                onConnected={(account) => {
                  onSelect(account);
                  setOpen(false);
                }}
              />
            ))
          )}
        </div>
      )}
    </div>
  );
}

function AuthenticatedWalletMenu({
  account,
  onForget,
}: {
  account: UiWalletAccount;
  onForget: () => void;
}) {
  const session = useSession();
  const signIn = useSignInWithWallet(account);
  const signOut = useSignOut();
  const attemptedForAddressRef = useRef<string | null>(null);

  const isSignedIn = session.data?.wallet === account.address;

  // Selecting a wallet account implies intent to use the app, so the
  // sign-in prompt follows immediately — still a direct continuation of
  // the user's own "connect" click, not an unprompted popup. Guarded by a
  // ref (not just `signIn.isIdle`) so a rejected/failed attempt for this
  // exact address is never silently retried in a loop.
  useEffect(() => {
    if (session.isLoading) return;
    if (isSignedIn) return;
    if (attemptedForAddressRef.current === account.address) return;
    attemptedForAddressRef.current = account.address;
    signIn.mutate();
    // Re-running on every `signIn`/`session` object identity change would
    // re-trigger the wallet prompt; the ref above is the actual re-run guard.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [account.address, isSignedIn, session.isLoading]);

  if (!isSignedIn) {
    return (
      <div className="flex min-h-11 items-center gap-2 rounded-lg border border-border-strong bg-surface-elevated px-4 py-2 text-sm text-text-secondary">
        {signIn.isError ? (
          <>
            <span>{messages.wallet.signInFailed}</span>
            <button
              type="button"
              onClick={() => {
                attemptedForAddressRef.current = null;
                signIn.mutate();
              }}
              className="font-semibold text-accent hover:text-accent-hover"
            >
              {messages.common.retry}
            </button>
          </>
        ) : (
          <span>{messages.wallet.signingIn}…</span>
        )}
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2">
      <span
        className="min-h-11 rounded-lg border border-border-strong bg-surface-elevated px-4 py-2 font-mono text-sm text-text-primary content-center"
        title={account.address}
      >
        {truncateAddress(account.address)}
      </span>
      <button
        type="button"
        onClick={() => {
          signOut.mutate();
          onForget();
        }}
        className="min-h-11 rounded-lg border border-border-strong px-3 py-2 text-sm text-text-secondary transition-colors hover:bg-surface-hover"
      >
        {messages.common.disconnectWallet}
      </button>
    </div>
  );
}
