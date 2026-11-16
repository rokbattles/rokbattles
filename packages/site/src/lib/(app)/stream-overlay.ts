import type { OverlayPreferences } from "@/lib/types/overlay";

export type OverlayConfig = {
  enabled: boolean;
  settings: OverlayPreferences;
};

export async function requestOverlay<T = unknown>(
  path: string,
  method = "GET",
  body?: unknown
): Promise<T> {
  const response = await fetch(path, {
    method,
    cache: "no-store",
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });

  if (!response.ok) {
    throw new Error("Unable to update overlay. Please try again.");
  }

  return response.json();
}
