const CLUSTER_QUERY = `cluster=${process.env.NEXT_PUBLIC_SOLANA_NETWORK ?? "devnet"}`;

export function explorerAddressUrl(address: string): string {
  return `https://explorer.solana.com/address/${address}?${CLUSTER_QUERY}`;
}

export function explorerTxUrl(signature: string): string {
  return `https://explorer.solana.com/tx/${signature}?${CLUSTER_QUERY}`;
}
