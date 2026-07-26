"use client";

import { SelectedWalletAccountContextProvider } from "@solana/react";

import { useEagerReconnect } from "@/hooks/useEagerReconnect";
import { supportsEventQuest, walletStateSync } from "@/lib/wallet";

function EagerReconnect() {
  useEagerReconnect();
  return null;
}

export function WalletProvider({ children }: { children: React.ReactNode }) {
  return (
    <SelectedWalletAccountContextProvider filterWallets={supportsEventQuest} stateSync={walletStateSync}>
      <EagerReconnect />
      {children}
    </SelectedWalletAccountContextProvider>
  );
}
