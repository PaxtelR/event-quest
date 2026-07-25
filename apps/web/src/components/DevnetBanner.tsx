import { messages } from "@/i18n/en-US";

// Spec §15.1: "Exibir permanentemente no ambiente de demonstração: Rede:
// Solana Devnet." Always rendered — this project never targets Mainnet.
export function DevnetBanner() {
  return (
    <div className="flex items-center justify-center gap-2 border-b border-border bg-surface px-4 py-1.5 text-xs font-medium text-text-secondary">
      <span aria-hidden className="h-1.5 w-1.5 rounded-full bg-accent" />
      {messages.devnet.banner}
    </div>
  );
}
