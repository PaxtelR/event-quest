import { z } from "zod";

// Boundary validation for the two create-forms (spec §15.1 lists Zod as
// the required validation library) — user input is the one place in this
// app that genuinely needs runtime validation, everything else already
// comes from a typed API response.

export const createEventSchema = z
  .object({
    name: z.string().trim().min(1, "Event name is required."),
    slug: z
      .string()
      .trim()
      .min(1, "URL slug is required.")
      .regex(/^[a-z0-9-]+$/, "Use only lowercase letters, numbers, and hyphens."),
    description: z.string().trim().optional(),
    locationName: z.string().trim().optional(),
    startsAt: z.string().min(1, "Start date is required."),
    endsAt: z.string().min(1, "End date is required."),
    visibility: z.enum(["public", "private"]),
  })
  .refine((value) => new Date(value.startsAt) < new Date(value.endsAt), {
    message: "Start date must be before the end date.",
    path: ["endsAt"],
  });

export type CreateEventFormValues = z.infer<typeof createEventSchema>;

export const createCheckpointSchema = z
  .object({
    name: z.string().trim().min(1, "Checkpoint name is required."),
    points: z.number().int().min(1, "Points must be at least 1."),
    opensAt: z.string().min(1, "Opens-at date is required."),
    closesAt: z.string().min(1, "Closes-at date is required."),
  })
  .refine((value) => new Date(value.opensAt) < new Date(value.closesAt), {
    message: "Opens-at must be before closes-at.",
    path: ["closesAt"],
  });

export type CreateCheckpointFormValues = z.infer<typeof createCheckpointSchema>;
