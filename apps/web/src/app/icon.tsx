import { ImageResponse } from "next/og";

export const size = { width: 32, height: 32 };
export const contentType = "image/png";

// Minimal wordmark-derived favicon — spec §15.1 "Identidade visual
// mínima": an orange "E" mark on the app's dark surface, no external
// asset pipeline needed for the MVP.
export default function Icon() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: "#141416",
          borderRadius: 6,
          color: "#f97316",
          fontSize: 22,
          fontWeight: 700,
          fontFamily: "system-ui, sans-serif",
        }}
      >
        E
      </div>
    ),
    size,
  );
}
