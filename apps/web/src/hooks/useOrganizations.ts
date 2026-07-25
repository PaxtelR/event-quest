"use client";

import { useQuery } from "@tanstack/react-query";

import { apiFetch } from "@/lib/api";

export type OrganizationMembership = {
  id: string;
  name: string;
  slug: string;
  role: "owner" | "admin" | "operator";
};

/**
 * The wallets's own organization memberships — see
 * apps/api/src/organizations.rs for why this endpoint exists (the admin
 * UI has no other way to discover which `organizationId` to pass to
 * `GET /events` for the wallet's own draft/private events).
 */
export function useMyOrganizations() {
  return useQuery({
    queryKey: ["organizations", "me"],
    queryFn: () => apiFetch<OrganizationMembership[]>("/organizations/me"),
  });
}
