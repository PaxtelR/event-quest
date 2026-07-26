"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { UiWalletAccount } from "@wallet-standard/react";

import { apiFetch } from "@/lib/api";
import type { EventDto } from "@/types/api";

import { useOnChainAction } from "./useOnChainAction";

export function useEvents(organizationId?: string) {
  return useQuery({
    queryKey: ["events", organizationId ?? "all"],
    queryFn: () =>
      apiFetch<EventDto[]>(
        organizationId ? `/events?organizationId=${encodeURIComponent(organizationId)}` : "/events",
      ),
    enabled: true,
  });
}

export function useEvent(eventId: string | undefined) {
  return useQuery({
    queryKey: ["events", "byId", eventId],
    queryFn: () => apiFetch<EventDto>(`/events/${eventId}`),
    enabled: Boolean(eventId),
  });
}

/**
 * There's no `GET /events/by-slug/:slug` endpoint (spec §12.3 never
 * defines one) — the public `/events/[slug]` page reuses the public
 * listing and filters client-side, which is fine at MVP scale. Returns
 * the same query-state shape (`isLoading`/`data`/...) as the other event
 * hooks so pages don't need special-case handling.
 */
export function useEventBySlug(slug: string) {
  const events = useEvents();
  return {
    ...events,
    data: events.data?.find((event) => event.slug === slug),
  };
}

export type CreateEventInput = {
  name: string;
  slug: string;
  description?: string;
  locationName?: string;
  startsAt: string;
  endsAt: string;
  visibility: "public" | "private";
  organizationId?: string;
  timezone?: string;
};

export function useCreateEvent() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: CreateEventInput) => apiFetch<EventDto>("/events", { method: "POST", json: input }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["events"] });
    },
  });
}

export type UpdateEventInput = Partial<Omit<CreateEventInput, "organizationId" | "slug">>;

export function useUpdateEvent(eventId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdateEventInput) =>
      apiFetch<EventDto>(`/events/${eventId}`, { method: "PATCH", json: input }),
    onSuccess: (event) => {
      queryClient.setQueryData(["events", "byId", eventId], event);
      queryClient.invalidateQueries({ queryKey: ["events"] });
    },
  });
}

/**
 * Publish/pause/finish each need a real organizer-signed on-chain
 * transaction — see apps/api/src/onchain.rs and
 * apps/api/src/events/mod.rs's header comment. `useOnChainAction` drives
 * the shared prepare/submitted pattern.
 */
export function useEventLifecycleAction(
  eventId: string,
  action: "publish" | "pause" | "finish",
  account: UiWalletAccount,
) {
  return useOnChainAction<EventDto>({
    account,
    preparePath: `/events/${eventId}/${action}/prepare-transaction`,
    submittedPath: `/events/${eventId}/${action}/submitted`,
    invalidateKeys: [["events", "byId", eventId], ["events"]],
  });
}
