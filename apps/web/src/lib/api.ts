// Thin fetch wrapper for apps/api — every request goes through this
// app's own origin (see next.config.ts's rewrites), so the session
// cookie is same-origin and no CORS handling is needed. Every error
// response from the backend follows spec §20's shape; this class
// preserves it instead of losing the code/message when it's caught.
export class ApiError extends Error {
  readonly code: string;
  readonly requestId: string;
  readonly status: number;

  constructor(status: number, body: { code: string; message: string; requestId: string }) {
    super(body.message);
    this.name = "ApiError";
    this.code = body.code;
    this.requestId = body.requestId;
    this.status = status;
  }
}

export type ApiFetchOptions = Omit<RequestInit, "body"> & {
  json?: unknown;
  idempotencyKey?: string;
};

export async function apiFetch<T>(path: string, options: ApiFetchOptions = {}): Promise<T> {
  const { json, idempotencyKey, headers, ...init } = options;

  const requestHeaders = new Headers(headers);
  if (json !== undefined) {
    requestHeaders.set("Content-Type", "application/json");
  }
  if (idempotencyKey) {
    requestHeaders.set("Idempotency-Key", idempotencyKey);
  }

  const response = await fetch(`/api/v1${path}`, {
    ...init,
    credentials: "same-origin",
    headers: requestHeaders,
    body: json !== undefined ? JSON.stringify(json) : undefined,
  });

  if (response.status === 204) {
    return undefined as T;
  }

  const contentType = response.headers.get("content-type") ?? "";
  const body = contentType.includes("application/json") ? await response.json() : undefined;

  if (!response.ok) {
    if (body && typeof body === "object" && "code" in body) {
      throw new ApiError(response.status, body as { code: string; message: string; requestId: string });
    }
    throw new ApiError(response.status, {
      code: "INTERNAL_ERROR",
      message: `Request to ${path} failed with status ${response.status}.`,
      requestId: "unknown",
    });
  }

  return body as T;
}
