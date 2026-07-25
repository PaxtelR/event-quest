import type { UiWallet } from "@wallet-standard/react";

// Spec §15.1: "Impedir envio se a configuração não for Devnet." Wallet
// Standard's `chains` array is what a wallet declares it CAN sign for, not
// what cluster it's currently pointed at — this is the strongest guard
// available at the application level; every signing hook in this app is
// still called with this exact chain id, so a wallet that lies about
// support fails at the signing call, not silently.
export const DEVNET_CHAIN = "solana:devnet" as const;

const REQUIRED_FEATURES = [
  "solana:signMessage",
  "solana:signTransaction",
  "solana:signAndSendTransaction",
] as const;

export function supportsEventQuest(wallet: UiWallet): boolean {
  return (
    wallet.chains.includes(DEVNET_CHAIN) &&
    REQUIRED_FEATURES.every((feature) => wallet.features.includes(feature))
  );
}

export function truncateAddress(address: string): string {
  if (address.length <= 10) return address;
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

const SELECTED_WALLET_STORAGE_KEY = "eventquest:selected-wallet-account";

// Persists the selected wallet account's storage key across reloads — the
// exact contract `SelectedWalletAccountContextProvider` (from
// `@solana/react`) expects for its `stateSync` prop.
export const walletStateSync = {
  getSelectedWallet(): string | null {
    if (typeof window === "undefined") return null;
    return window.localStorage.getItem(SELECTED_WALLET_STORAGE_KEY);
  },
  storeSelectedWallet(accountKey: string): void {
    window.localStorage.setItem(SELECTED_WALLET_STORAGE_KEY, accountKey);
  },
  deleteSelectedWallet(): void {
    window.localStorage.removeItem(SELECTED_WALLET_STORAGE_KEY);
  },
};
