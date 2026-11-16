export type SessionPlatform = "browser" | "desktop" | "unknown";

export type DeviceSession = {
  id: string;
  platform: SessionPlatform;
  createdAt: number;
  current: boolean;
};

export class SessionRequestError extends Error {
  constructor(public status: number) {
    super("Session request failed");
  }
}

const endpoint = "/proxy/v1/auth/sessions";

async function request(path: string, method: string, signal?: AbortSignal): Promise<Response> {
  const response = await fetch(path, { method, signal, cache: "no-store" });

  if (!response.ok) {
    throw new SessionRequestError(response.status);
  }

  return response;
}

export async function fetchSessions(signal?: AbortSignal): Promise<DeviceSession[]> {
  const response = await request(endpoint, "GET", signal);
  const payload = (await response.json()) as { sessions: DeviceSession[] };

  return payload.sessions;
}

export async function logoutSession(id: string): Promise<void> {
  await request(`${endpoint}/${encodeURIComponent(id)}`, "DELETE");
}

export async function logoutAllSessions(): Promise<void> {
  await request(endpoint, "DELETE");
}

const relativeTime = new Intl.RelativeTimeFormat("en-US", { numeric: "always" });

export function formatSessionAge(createdAt: number, now: number): string {
  const minutes = Math.max(0, Math.floor((now - createdAt) / 60_000));

  if (minutes >= 1440) {
    return relativeTime.format(-Math.floor(minutes / 1440), "day");
  }

  if (minutes >= 60) {
    return relativeTime.format(-Math.floor(minutes / 60), "hour");
  }

  return minutes > 0 ? relativeTime.format(-minutes, "minute") : "just now";
}
