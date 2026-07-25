"use client";

import jsQR from "jsqr";
import { useEffect, useRef, useState } from "react";

export type ScannerStatus = "idle" | "requesting-camera" | "scanning" | "camera-unavailable";

/**
 * Requests the rear camera only after the caller flips `enabled` to
 * true (spec §15.5: "Solicitar permissão de câmera apenas após ação do
 * usuário"), then polls frames through `jsQR` (pure-JS decoding — works
 * on Safari/iOS too, unlike the native `BarcodeDetector` API) until a QR
 * code resolves, calling `onDetected` at most once per scan session.
 */
export function useQrScanner(enabled: boolean, onDetected: (data: string) => void) {
  const videoRef = useRef<HTMLVideoElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const detectedRef = useRef(false);
  const [status, setStatus] = useState<ScannerStatus>("idle");

  useEffect(() => {
    if (!enabled) return;

    let stream: MediaStream | null = null;
    let frameHandle = 0;
    let cancelled = false;
    detectedRef.current = false;

    async function start() {
      setStatus("requesting-camera");
      try {
        stream = await navigator.mediaDevices.getUserMedia({
          video: { facingMode: { ideal: "environment" } },
          audio: false,
        });
      } catch {
        if (!cancelled) setStatus("camera-unavailable");
        return;
      }
      if (cancelled || !videoRef.current) {
        stream.getTracks().forEach((track) => track.stop());
        return;
      }

      videoRef.current.srcObject = stream;
      await videoRef.current.play();
      setStatus("scanning");

      const canvas = canvasRef.current ?? document.createElement("canvas");
      canvasRef.current = canvas;
      const context = canvas.getContext("2d", { willReadFrequently: true });

      const scanFrame = () => {
        if (cancelled || detectedRef.current) return;
        const video = videoRef.current;
        if (video && context && video.videoWidth > 0) {
          canvas.width = video.videoWidth;
          canvas.height = video.videoHeight;
          context.drawImage(video, 0, 0, canvas.width, canvas.height);
          const frame = context.getImageData(0, 0, canvas.width, canvas.height);
          const result = jsQR(frame.data, frame.width, frame.height, { inversionAttempts: "dontInvert" });
          if (result?.data) {
            detectedRef.current = true;
            navigator.vibrate?.(200);
            onDetected(result.data);
            return;
          }
        }
        frameHandle = requestAnimationFrame(scanFrame);
      };
      frameHandle = requestAnimationFrame(scanFrame);
    }

    start();

    return () => {
      cancelled = true;
      cancelAnimationFrame(frameHandle);
      stream?.getTracks().forEach((track) => track.stop());
      setStatus("idle");
    };
    // `onDetected` is expected to be a stable-enough callback from the
    // caller; adding it here would restart the camera stream on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled]);

  return { videoRef, status };
}
