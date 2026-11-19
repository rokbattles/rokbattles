import type { ClaimedGovernor } from "@/lib/types/current-user";

export type BindSuggestion = Pick<
  ClaimedGovernor,
  "governorId" | "governorName" | "governorAvatar"
>;

const endpoint = "/proxy/v1/me/governor-binds";

export async function claimBind(governorId: number): Promise<ClaimedGovernor> {
  const response = await bindRequest(`/${governorId}`, "POST", "Unable to bind this governor.");
  const payload = (await response.json()) as { claim: ClaimedGovernor };
  return payload.claim;
}

export async function setDefaultBind(governorId: number): Promise<void> {
  await bindRequest(`/${governorId}/default`, "PATCH", "Unable to set the default governor.");
}

export async function unlinkBinds(governorIds: number[]): Promise<void> {
  await bindRequest("/bulk-unbind", "POST", "Unable to unbind the selected governors.", {
    governorIds,
  });
}

export async function shareBinds(governorIds: number[], discordId: string): Promise<void> {
  await bindRequest("/share", "POST", "Unable to share the selected governors.", {
    governorIds,
    discordId,
  });
}

export async function revokeShare(governorId: number, discordId: string): Promise<void> {
  await bindRequest(
    `/${governorId}/shares/${encodeURIComponent(discordId)}`,
    "DELETE",
    "Unable to revoke this share."
  );
}

export async function findMoreGovernors(governorId: number): Promise<BindSuggestion[]> {
  const response = await bindRequest(
    `/${governorId}/suggestions`,
    "GET",
    "Unable to find other governors."
  );

  return (await response.json()) as BindSuggestion[];
}

export async function bindSuggestions(governorId: number, governorIds: number[]): Promise<void> {
  await bindRequest(
    `/${governorId}/suggestions/bind`,
    "POST",
    "Unable to bind the selected governors.",
    { governorIds }
  );
}

async function bindRequest(
  path: string,
  method: string,
  fallback: string,
  body?: object
): Promise<Response> {
  const response = await fetch(`${endpoint}${path}`, {
    method,
    ...(body
      ? { headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }
      : {}),
  });

  if (!response.ok) {
    const payload = (await response.json().catch(() => null)) as { error?: unknown } | null;
    const message = payload?.error;
    throw new Error(
      typeof message === "string" && message.trim() && message !== "internal-server-error"
        ? message
        : fallback
    );
  }

  return response;
}
