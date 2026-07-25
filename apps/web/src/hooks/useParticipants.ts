"use client";

import { useQuery } from "@tanstack/react-query";

import { apiFetch } from "@/lib/api";

export type MyEventProgress = {
  eventId: string;
  eventName: string;
  eventSlug: string;
  points: number;
  checkinCount: number;
  completed: boolean;
  joinedAt: string;
};

/** The authenticated wallet's own cross-event progress — spec §12.3's
 * `GET /me/events` (participant passport page). */
export function useMyEvents() {
  return useQuery({
    queryKey: ["me", "events"],
    queryFn: () => apiFetch<MyEventProgress[]>("/me/events"),
  });
}

export type EventParticipant = {
  participantId: string;
  walletAddress: string;
  points: number;
  checkinCount: number;
  completed: boolean;
  joinedAt: string;
};

/** Organizer-facing participants list for one event — see
 * apps/api/src/participants.rs's header comment for why this endpoint
 * exists (spec never lists one, but the admin participants/analytics
 * pages have no other real data source). */
export function useEventParticipants(eventId: string | undefined) {
  return useQuery({
    queryKey: ["events", eventId, "participants"],
    queryFn: () => apiFetch<EventParticipant[]>(`/events/${eventId}/participants`),
    enabled: Boolean(eventId),
  });
}
