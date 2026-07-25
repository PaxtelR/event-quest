"use client";

import QRCode from "qrcode";
import { useEffect, useState } from "react";

/**
 * Spec §15.1's QR component rules: pure black modules on a white
 * background regardless of the app's dark theme (readability across
 * cameras/lighting beats brand consistency here), with an orange accent
 * border on the surrounding dark card.
 */
export function RotatingQrCode({ value }: { value: string }) {
  const [dataUrl, setDataUrl] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    QRCode.toDataURL(value, {
      width: 480,
      margin: 2,
      color: { dark: "#000000", light: "#ffffff" },
    }).then((url) => {
      if (!cancelled) setDataUrl(url);
    });
    return () => {
      cancelled = true;
    };
  }, [value]);

  return (
    <div className="rounded-2xl border-4 border-accent bg-white p-4 shadow-lg">
      {dataUrl ? (
        // A locally generated data: URI, not an optimizable remote asset.
        // eslint-disable-next-line @next/next/no-img-element
        <img src={dataUrl} alt="" className="h-64 w-64 sm:h-80 sm:w-80" />
      ) : (
        <div className="h-64 w-64 animate-pulse bg-black/5 sm:h-80 sm:w-80" />
      )}
    </div>
  );
}
