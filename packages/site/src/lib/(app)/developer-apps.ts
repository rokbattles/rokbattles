type GeneralSettings = {
  name: string;
  description: string | null;
  termsOfServiceUrl: string | null;
  privacyPolicyUrl: string | null;
};

type AppUpdate =
  | ({ section: "general" } & GeneralSettings)
  | { section: "authorization"; redirectUris: string[] };

type AppRegistration = { name: string; acceptTerms: boolean };

export type DeveloperApp = GeneralSettings & {
  redirectUris: string[];
  id: string;
  termsAcceptedAt: number;
  userAgent: string;
};

export function createDeveloperApp(input: AppRegistration): Promise<DeveloperApp> {
  return requestApp("/v1/developer/apps", "POST", input);
}

export function updateDeveloperApp(id: string, input: AppUpdate): Promise<DeveloperApp> {
  return requestApp(`/v1/developer/apps/${id}`, "PATCH", input);
}

async function requestApp(
  path: string,
  method: "POST" | "PATCH",
  input: AppRegistration | AppUpdate
) {
  const response = await fetch(`/proxy${path}`, {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(input),
  });

  if (response.ok) {
    return (await response.json()) as DeveloperApp;
  }

  if (response.status === 401) {
    throw new Error("Your session expired. Sign in again to continue.");
  }

  if (response.status === 403) {
    throw new Error("Developer Program membership is required.");
  }

  if (response.status === 429) {
    throw new Error("Too many requests. Wait a moment and try again.");
  }

  if (response.status === 400 || response.status === 404) {
    const payload = (await response.json()) as { error: string };
    throw new Error(payload.error);
  }

  throw new Error("The app could not be saved. Try again.");
}
