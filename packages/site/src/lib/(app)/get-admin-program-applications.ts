import { cookies } from "next/headers";
import { notFound, redirect } from "next/navigation";
import {
  ADMIN_PROGRAM_APPLICATIONS_PATH,
  type AdminApplicationPage,
} from "@/lib/(app)/admin-program-applications";

export async function getAdminProgramApplications(page: number): Promise<AdminApplicationPage> {
  const sid = (await cookies()).get("sid")?.value;

  if (!sid) {
    redirect("/");
  }

  const origin = process.env.API_URL || "http://localhost:8001";
  const response = await fetch(`${origin}${ADMIN_PROGRAM_APPLICATIONS_PATH}?page=${page}`, {
    headers: { Cookie: `sid=${sid}` },
    cache: "no-store",
  });

  if (response.status === 401) {
    redirect("/");
  }

  if (response.status === 403) {
    notFound();
  }

  if (!response.ok) {
    throw new Error(`Admin program applications request failed (${response.status})`);
  }

  return (await response.json()) as AdminApplicationPage;
}
