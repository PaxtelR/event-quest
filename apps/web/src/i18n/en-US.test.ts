import { describe, expect, it } from "vitest";

import { messages } from "./en-US";

// Spec §15.1: every user-visible string must be English, non-empty, and
// centralized here — this is the automated guard for that requirement as
// the copy grows across later tasks.
function collectStrings(value: unknown, path: string, out: Map<string, string>): void {
  if (typeof value === "string") {
    out.set(path, value);
    return;
  }
  if (value && typeof value === "object") {
    for (const [key, child] of Object.entries(value)) {
      collectStrings(child, path ? `${path}.${key}` : key, out);
    }
  }
}

const PORTUGUESE_MARKERS = /[ãõáéíóúâêôçÃÕÁÉÍÓÚÂÊÔÇ]/;

describe("en-US messages", () => {
  const strings = new Map<string, string>();
  collectStrings(messages, "", strings);

  it("has at least one entry", () => {
    expect(strings.size).toBeGreaterThan(0);
  });

  it.each([...strings.entries()])("%s is a non-empty string", (_path, value) => {
    expect(value.trim().length).toBeGreaterThan(0);
  });

  it.each([...strings.entries()])("%s contains no Portuguese diacritics", (_path, value) => {
    expect(PORTUGUESE_MARKERS.test(value)).toBe(false);
  });
});
