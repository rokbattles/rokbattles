import type { Program } from "@/lib/(app)/types/program-application";

export const ADMIN_PROGRAM_APPLICATIONS_PATH = "/v1/admin/program-applications";

export type AdminApplicationSummary = {
  id: string;
  discordId: string;
  program: Program;
  status: string;
  createdAt: number;
};

export type AdminApplicationPage = {
  applications: AdminApplicationListItem[];
  hasNextPage: boolean;
};

export type AdminApplicationListItem = AdminApplicationSummary & {
  defaultGovernor: { governorId: number; governorName: string | null } | null;
};

export type AdminApplicationDetail = AdminApplicationListItem & {
  form: { question: string; answer: string }[];
  email: string | null;
};

export type ApplicationDecision = "approve" | "deny";

function applicationEndpoint(id: string) {
  return `/proxy${ADMIN_PROGRAM_APPLICATIONS_PATH}/${id}`;
}

export async function fetchAdminApplication(id: string): Promise<AdminApplicationDetail> {
  const response = await fetch(applicationEndpoint(id), { cache: "no-store" });

  if (!response.ok) {
    throw new Error("Application details unavailable");
  }

  return (await response.json()) as AdminApplicationDetail;
}

export async function reviewAdminApplication(
  id: string,
  decision: ApplicationDecision
): Promise<AdminApplicationSummary | null> {
  const response = await fetch(applicationEndpoint(id), {
    method: "PATCH",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ decision }),
  });

  if (response.status === 409) {
    return null;
  }

  if (!response.ok) {
    throw new Error("Unable to save the decision");
  }

  return (await response.json()) as AdminApplicationSummary;
}
