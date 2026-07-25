import Link from "next/link";

import { ConnectWalletButton } from "@/components/ConnectWalletButton";
import { messages } from "@/i18n/en-US";

export function SiteHeader() {
  return (
    <header className="flex items-center justify-between gap-4 border-b border-border px-4 py-3 sm:px-6">
      <Link href="/" className="text-sm font-semibold tracking-wide text-text-primary">
        {messages.common.appName}
      </Link>
      <nav
        aria-label={messages.nav.mainNavigation}
        className="hidden items-center gap-6 text-sm text-text-secondary sm:flex"
      >
        <Link href="/events" className="transition-colors hover:text-text-primary">
          {messages.nav.events}
        </Link>
        <Link href="/admin" className="transition-colors hover:text-text-primary">
          {messages.nav.admin}
        </Link>
        <Link href="/participant/passport" className="transition-colors hover:text-text-primary">
          {messages.nav.myPassport}
        </Link>
      </nav>
      <ConnectWalletButton />
    </header>
  );
}
