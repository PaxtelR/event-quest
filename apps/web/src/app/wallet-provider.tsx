"use client";

import { SelectedWalletAccountContextProvider } from "@solana/react";

import { supportsEventQuest, walletStateSync } from "@/lib/wallet";

export function WalletProvider({ children }: { children: React.ReactNode }) {
  return (
    <SelectedWalletAccountContextProvider filterWallets={supportsEventQuest} stateSync={walletStateSync}>
      {children}
    </SelectedWalletAccountContextProvider>
  );
}
