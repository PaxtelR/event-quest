"use client";

import Link from "next/link";

import { RequireAuth } from "@/components/RequireAuth";
import { messages } from "@/i18n/en-US";

export default function AdminDashboardPage() {
  return (
    <RequireAuth>
      {() => (
        <main className="mx-auto w-full max-w-2xl flex-1 px-4 py-16 text-center sm:px-6">
          <h1 className="text-2xl font-semibold text-text-primary">{messages.admin.dashboardTitle}</h1>
          <p className="mt-2 text-sm text-text-secondary">{messages.admin.dashboardDescription}</p>
          <Link
            href="/admin/events"
            className="mt-8 inline-block rounded-lg bg-accent px-6 py-3 text-sm font-semibold text-accent-foreground transition-colors hover:bg-accent-hover active:bg-accent-active"
          >
            {messages.admin.manageEvents}
          </Link>
        </main>
      )}
    </RequireAuth>
  );
}
