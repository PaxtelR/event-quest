import { describe, expect, it } from "vitest";

import { extractQrToken } from "./qrToken";

describe("extractQrToken", () => {
  it("extracts the token from a full check-in URL", () => {
    expect(extractQrToken("https://app.example.com/check-in?token=abc.def.ghi")).toBe("abc.def.ghi");
  });

  it("accepts a bare token pasted directly", () => {
    expect(extractQrToken("abc.def.ghi")).toBe("abc.def.ghi");
  });

  it("returns null for an empty string", () => {
    expect(extractQrToken("   ")).toBeNull();
  });

  it("returns null for a URL with no token param", () => {
    expect(extractQrToken("https://app.example.com/check-in")).toBeNull();
  });
});
