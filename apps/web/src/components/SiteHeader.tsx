"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";

import { ConnectWalletButton } from "@/components/ConnectWalletButton";
import { messages } from "@/i18n/en-US";

const NAV_LINKS = [
  { href: "/events", label: messages.nav.events },
  { href: "/check-in", label: messages.nav.checkIn },
  { href: "/admin", label: messages.nav.admin },
  { href: "/participant/passport", label: messages.nav.myPassport },
] as const;

export function SiteHeader() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // The nav links are hidden below the `sm` breakpoint (no room for them
  // inline next to the logo and wallet button) — without this menu there
  // was no way at all to reach them, `/participant/passport` included, on
  // a phone-sized viewport.
  useEffect(() => {
    if (!mobileMenuOpen) return;
    function handleClickOutside(event: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setMobileMenuOpen(false);
      }
    }
    function handleEscape(event: KeyboardEvent) {
      if (event.key === "Escape") setMobileMenuOpen(false);
    }
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [mobileMenuOpen]);

  return (
    <div ref={containerRef} className="relative border-b border-border">
      <header className="flex items-center justify-between gap-4 px-4 py-3 sm:px-6">
        <Link href="/" className="text-sm font-semibold tracking-wide text-text-primary">
          {messages.common.appName}
        </Link>
        <nav
          aria-label={messages.nav.mainNavigation}
          className="hidden items-center gap-6 text-sm text-text-secondary sm:flex"
        >
          {NAV_LINKS.map((link) => (
            <Link key={link.href} href={link.href} className="transition-colors hover:text-text-primary">
              {link.label}
            </Link>
          ))}
        </nav>
        <div className="flex items-center gap-2">
          <ConnectWalletButton />
          <button
            type="button"
            aria-haspopup="menu"
            aria-expanded={mobileMenuOpen}
            aria-label={mobileMenuOpen ? messages.nav.closeMenu : messages.nav.openMenu}
            onClick={() => setMobileMenuOpen((value) => !value)}
            className="flex min-h-11 min-w-11 items-center justify-center rounded-lg border border-border-strong bg-surface-elevated text-text-primary hover:bg-surface-hover sm:hidden"
          >
            <span aria-hidden className="text-lg leading-none">
              {mobileMenuOpen ? "✕" : "☰"}
            </span>
          </button>
        </div>
      </header>
      {mobileMenuOpen && (
        <nav
          aria-label={messages.nav.mainNavigation}
          className="flex flex-col gap-1 border-t border-border px-4 py-2 sm:hidden"
        >
          {NAV_LINKS.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              onClick={() => setMobileMenuOpen(false)}
              className="min-h-11 content-center rounded-lg px-2 text-sm text-text-secondary transition-colors hover:bg-surface-hover hover:text-text-primary"
            >
              {link.label}
            </Link>
          ))}
        </nav>
      )}
    </div>
  );
}
