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
