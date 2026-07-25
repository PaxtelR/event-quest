/**
 * The QR encodes a full URL (`${PUBLIC_APP_URL}/check-in?token=...` — see
 * apps/api/src/qr/rotation.rs), since scanning with a phone's native
 * camera app should open this page directly. The in-app scanner and the
 * "paste link" fallback both need to accept either the full URL or a bare
 * token, so this extracts the token from whatever the user provides.
 */
export function extractQrToken(scanned: string): string | null {
  const trimmed = scanned.trim();
  if (!trimmed) return null;

  try {
    const url = new URL(trimmed);
    // A parseable URL with no `token` param isn't a usable check-in link
    // (it's clearly not what the QR/paste-link flow expects), so it's
    // null rather than passed through as a bogus "token".
    return url.searchParams.get("token");
  } catch {
    // Not a URL — treat it as a bare token pasted directly.
    return trimmed;
  }
}
