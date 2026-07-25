import { describe, expect, it } from "vitest";

import { createCheckpointSchema, createEventSchema } from "./validation";

describe("createEventSchema", () => {
  const valid = {
    name: "Solana Workshop",
    slug: "solana-workshop",
    startsAt: "2026-08-01T10:00",
    endsAt: "2026-08-01T18:00",
    visibility: "public" as const,
  };

  it("accepts a valid event", () => {
    expect(createEventSchema.safeParse(valid).success).toBe(true);
  });

  it("rejects an empty name", () => {
    const result = createEventSchema.safeParse({ ...valid, name: "  " });
    expect(result.success).toBe(false);
  });

  it("rejects a slug with uppercase or spaces", () => {
    const result = createEventSchema.safeParse({ ...valid, slug: "Solana Workshop" });
    expect(result.success).toBe(false);
  });

  it("rejects when the start date is not before the end date", () => {
    const result = createEventSchema.safeParse({ ...valid, startsAt: valid.endsAt, endsAt: valid.startsAt });
    expect(result.success).toBe(false);
  });
});

describe("createCheckpointSchema", () => {
  const valid = {
    name: "Main stage",
    points: 25,
    opensAt: "2026-08-01T10:00",
    closesAt: "2026-08-01T12:00",
  };

  it("accepts a valid checkpoint", () => {
    expect(createCheckpointSchema.safeParse(valid).success).toBe(true);
  });

  it("rejects zero or negative points", () => {
    expect(createCheckpointSchema.safeParse({ ...valid, points: 0 }).success).toBe(false);
  });

  it("rejects when opensAt is not before closesAt", () => {
    const result = createCheckpointSchema.safeParse({ ...valid, opensAt: valid.closesAt, closesAt: valid.opensAt });
    expect(result.success).toBe(false);
  });
});
