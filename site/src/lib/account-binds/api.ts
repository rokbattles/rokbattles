const BIND_ENDPOINT = "/proxy/v1/governor/bind";
const DEFAULT_BIND_ENDPOINT = "/proxy/v1/governor/bind/default";

type ClaimBindResponse = {
  claim: {
    governorId: number;
    governorName: string | null;
    governorAvatar: string | null;
    default: boolean;
  };
};

export class BindApiError extends Error {}

export async function claimBind(governorId: number): Promise<ClaimBindResponse["claim"]> {
  const response = await fetch(BIND_ENDPOINT, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ governorId }),
  });

  if (!response.ok) {
    throw new BindApiError(
      await extractApiError(response, "Unable to claim bind. Please try again.")
    );
  }

  const payload = (await response.json()) as ClaimBindResponse;
  return payload.claim;
}

export async function setDefaultBind(governorId: number): Promise<void> {
  const response = await fetch(
    `${DEFAULT_BIND_ENDPOINT}?governorId=${encodeURIComponent(governorId.toString())}`,
    {
      method: "PATCH",
    }
  );

  if (!response.ok) {
    throw new BindApiError(
      await extractApiError(response, "Unable to set default bind. Please try again.")
    );
  }
}

export async function unlinkBind(governorId: number): Promise<void> {
  const response = await fetch(
    `${BIND_ENDPOINT}?governorId=${encodeURIComponent(governorId.toString())}`,
    {
      method: "DELETE",
    }
  );

  if (!response.ok) {
    throw new BindApiError(
      await extractApiError(response, "Unable to unlink bind. Please try again.")
    );
  }
}

async function extractApiError(response: Response, fallback: string): Promise<string> {
  try {
    const payload = (await response.json()) as { error?: unknown };
    if (typeof payload?.error === "string" && payload.error.trim() !== "") {
      return payload.error;
    }
  } catch {
    // ignored
  }

  return fallback;
}
