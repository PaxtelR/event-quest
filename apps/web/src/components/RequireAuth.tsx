"use client";

import { useSelectedWalletAccount } from "@solana/react";
import type { UiWalletAccount } from "@wallet-standard/react";

import { useSession } from "@/hooks/useAuth";
import { messages } from "@/i18n/en-US";

/**
 * Gates admin pages on an authenticated session. The connect/sign-in flow
 * itself lives in the always-visible header (`ConnectWalletButton`) — this
 * just blocks the page's own content and tells the organizer why.
 */
export function RequireAuth({
  message = messages.admin.signInRequired,
  children,
}: {
  message?: string;
  children: (account: UiWalletAccount) => React.ReactNode;
}) {
  const [selectedAccount] = useSelectedWalletAccount();
  const session = useSession();

  if (session.isLoading) {
    return (
      <p className="p-10 text-center text-sm text-text-secondary" role="status">
        {messages.common.loading}…
      </p>
    );
  }

  if (!selectedAccount || !session.data) {
    return (
      <div className="mx-auto flex w-full max-w-md flex-1 flex-col items-center justify-center gap-2 px-4 py-20 text-center">
        <p className="text-sm text-text-secondary">{message}</p>
      </div>
    );
  }

  return <>{children(selectedAccount)}</>;
}
