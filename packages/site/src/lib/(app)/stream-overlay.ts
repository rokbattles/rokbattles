import type { OverlayPreferences } from "@/lib/(app)/types/overlay";

export type OverlayConfig = {
  enabled: boolean;
  path: string | null;
  settings: OverlayPreferences;
};

function overlayEndpoint(governorId: number): string {
  return `/proxy/v1/governor/${governorId}/overlay`;
}

async function requestOverlay<T>(
  path: string,
  method = "GET",
  body?: OverlayPreferences,
  signal?: AbortSignal
): Promise<T> {
  const response = await fetch(path, {
    method,
    signal,
    cache: "no-store",
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });

  if (!response.ok) {
    throw new Error("Unable to update overlay. Please try again.");
  }

  return response.json();
}

export function fetchOverlayConfig(
  governorId: number,
  signal: AbortSignal
): Promise<OverlayConfig> {
  return requestOverlay(overlayEndpoint(governorId), "GET", undefined, signal);
}

export function saveOverlaySettings(
  governorId: number,
  settings: OverlayPreferences
): Promise<OverlayPreferences> {
  return requestOverlay(overlayEndpoint(governorId), "PUT", settings);
}

export function rotateOverlayUrl(governorId: number): Promise<{ path: string }> {
  return requestOverlay(`${overlayEndpoint(governorId)}/rotate`, "POST");
}

export async function disableOverlay(governorId: number): Promise<void> {
  await requestOverlay(overlayEndpoint(governorId), "DELETE");
}
