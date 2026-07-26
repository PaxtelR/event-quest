"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";

import { RequireAuth } from "@/components/RequireAuth";
import { useCreateEvent } from "@/hooks/useEvents";
import { messages } from "@/i18n/en-US";
import { ApiError } from "@/lib/api";
import { createEventSchema } from "@/lib/validation";

function slugify(value: string): string {
  return value
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function CreateEventForm() {
  const router = useRouter();
  const createEvent = useCreateEvent();

  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [slugTouched, setSlugTouched] = useState(false);
  const [description, setDescription] = useState("");
  const [locationName, setLocationName] = useState("");
  const [startsAt, setStartsAt] = useState("");
  const [endsAt, setEndsAt] = useState("");
  const [visibility, setVisibility] = useState<"public" | "private">("private");
  const [validationError, setValidationError] = useState<string | null>(null);

  return (
    <main className="mx-auto w-full max-w-xl flex-1 px-4 py-10 sm:px-6">
      <h1 className="text-2xl font-semibold text-text-primary">{messages.admin.createEvent}</h1>

      <form
        className="mt-8 flex flex-col gap-5"
        onSubmit={(event) => {
          event.preventDefault();
          setValidationError(null);

          const result = createEventSchema.safeParse({
            name,
            slug: slug || slugify(name),
            description: description || undefined,
            locationName: locationName || undefined,
            startsAt,
            endsAt,
            visibility,
          });
          if (!result.success) {
            setValidationError(result.error.issues[0]?.message ?? messages.admin.form.startBeforeEnd);
            return;
          }

          createEvent.mutate(
            {
              ...result.data,
              startsAt: new Date(result.data.startsAt).toISOString(),
              endsAt: new Date(result.data.endsAt).toISOString(),
              // Without this, every event defaults to the backend's
              // `timezone = 'UTC'` column default, and every display page
              // (which formats dates using the event's stored timezone,
              // not the viewer's own) would render UTC wall-clock times
              // mislabeled as if they were whatever the organizer typed —
              // e.g. a 13:00 local start showing as "16:00" to everyone.
              timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
            },
            {
              onSuccess: (event) => router.push(`/admin/events/${event.id}`),
            },
          );
        }}
      >
        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-text-primary">{messages.admin.form.name}</span>
          <input
            required
            value={name}
            onChange={(event) => {
              setName(event.target.value);
              if (!slugTouched) setSlug(slugify(event.target.value));
            }}
            className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
          />
        </label>

        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-text-primary">{messages.admin.form.slug}</span>
          <input
            required
            value={slug}
            onChange={(event) => {
              setSlugTouched(true);
              setSlug(slugify(event.target.value));
            }}
            className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 font-mono text-sm text-text-primary focus:border-accent focus:outline-none"
          />
          <span className="text-xs text-text-disabled">{messages.admin.form.slugHint}</span>
        </label>

        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-text-primary">{messages.admin.form.description}</span>
          <textarea
            value={description}
            onChange={(event) => setDescription(event.target.value)}
            rows={3}
            className="rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
          />
        </label>

        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-text-primary">{messages.admin.form.locationName}</span>
          <input
            value={locationName}
            onChange={(event) => setLocationName(event.target.value)}
            className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
          />
        </label>

        <div className="grid grid-cols-1 gap-5 sm:grid-cols-2">
          <label className="flex flex-col gap-1.5">
            <span className="text-sm font-medium text-text-primary">{messages.admin.form.startsAt}</span>
            <input
              required
              type="datetime-local"
              value={startsAt}
              onChange={(event) => setStartsAt(event.target.value)}
              className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
            />
          </label>
          <label className="flex flex-col gap-1.5">
            <span className="text-sm font-medium text-text-primary">{messages.admin.form.endsAt}</span>
            <input
              required
              type="datetime-local"
              value={endsAt}
              onChange={(event) => setEndsAt(event.target.value)}
              className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
            />
          </label>
        </div>

        <label className="flex flex-col gap-1.5">
          <span className="text-sm font-medium text-text-primary">{messages.admin.form.visibility}</span>
          <select
            value={visibility}
            onChange={(event) => setVisibility(event.target.value as "public" | "private")}
            className="min-h-11 rounded-lg border border-border bg-surface-elevated px-3 py-2 text-sm text-text-primary focus:border-accent focus:outline-none"
          >
            <option value="private">{messages.admin.form.visibilityPrivate}</option>
            <option value="public">{messages.admin.form.visibilityPublic}</option>
          </select>
        </label>

        {(validationError || createEvent.isError) && (
          <p className="text-sm text-error" role="alert">
            {validationError ??
              (createEvent.error instanceof ApiError
                ? createEvent.error.message
                : messages.admin.form.createFailed)}
          </p>
        )}

        <button
          type="submit"
          disabled={createEvent.isPending}
          className="min-h-11 rounded-lg bg-accent px-6 py-2.5 text-sm font-semibold text-accent-foreground transition-colors hover:bg-accent-hover active:bg-accent-active disabled:cursor-not-allowed disabled:opacity-60"
        >
          {createEvent.isPending ? `${messages.admin.form.creating}…` : messages.admin.createEvent}
        </button>
      </form>
    </main>
  );
}

export default function NewEventPage() {
  return <RequireAuth>{() => <CreateEventForm />}</RequireAuth>;
}
