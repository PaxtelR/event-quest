// Mirrors apps/api's JSON response shapes exactly (camelCase, per
// eventquest-api's serde `rename_all`). Kept as one file since these DTOs
// are shared across many pages — see apps/api/src/events/models.rs and
// apps/api/src/checkpoints/models.rs for the source of truth.

export type EventStatus = "draft" | "active" | "paused" | "finished" | "cancelled";
export type EventVisibility = "public" | "private";
export type CheckpointStatus = "draft" | "active" | "paused" | "closed";

export type EventDto = {
  id: string;
  organizationId: string;
  name: string;
  slug: string;
  description: string | null;
  bannerUrl: string | null;
  locationName: string | null;
  locationAddress: string | null;
  timezone: string;
  startsAt: string;
  endsAt: string;
  status: EventStatus;
  visibility: EventVisibility;
  solanaNetwork: string;
  onchainEventAddress: string | null;
  onchainCreateSignature: string | null;
  createdByWallet: string;
  createdAt: string;
  updatedAt: string;
};

export type CheckpointDto = {
  id: string;
  eventId: string;
  name: string;
  description: string | null;
  points: number;
  rotationSeconds: number;
  opensAt: string;
  closesAt: string;
  status: CheckpointStatus;
  attestorPubkey: string;
  onchainCheckpointAddress: string | null;
  createdAt: string;
  updatedAt: string;
};
