// Mobile wallet apps only inject a Wallet Standard provider inside their
// own in-app browser, never into the phone's regular Safari/Chrome — a
// participant who opens a QR-scanned check-in link in a normal mobile
// browser will never see a connectable wallet there, no matter how long
// they wait. These "browse" universal links are how Phantom/Solflare's
// own docs say to hand a URL to their in-app browser instead, confirmed
// against their current documentation (not guessed):
// https://docs.phantom.com/phantom-deeplinks/other-methods/browse
// https://docs.solflare.com/solflare/technical/deeplinks/other-methods/browse
// https://docs.backpack.app/deeplinks/other-methods/browse
//
// There's no wallet-agnostic version of this — each wallet defines its own
// scheme, and most smaller wallets don't publish one at all. The actual
// wallet-agnostic fix is Solana Mobile's Mobile Wallet Adapter, but that's
// Android-only (no iOS equivalent) and a separate SDK integration, not a
// link — these three cover the large majority of real-world Solana wallet
// usage without that larger lift.

function browseDeepLink(base: string, targetUrl: string, referrerUrl: string): string {
  return `${base}/${encodeURIComponent(targetUrl)}?ref=${encodeURIComponent(referrerUrl)}`;
}

export function getPhantomBrowseUrl(targetUrl: string, referrerUrl: string): string {
  return browseDeepLink("https://phantom.app/ul/browse", targetUrl, referrerUrl);
}

export function getSolflareBrowseUrl(targetUrl: string, referrerUrl: string): string {
  return browseDeepLink("https://solflare.com/ul/v1/browse", targetUrl, referrerUrl);
}

export function getBackpackBrowseUrl(targetUrl: string, referrerUrl: string): string {
  return browseDeepLink("https://backpack.app/ul/v1/browse", targetUrl, referrerUrl);
}
