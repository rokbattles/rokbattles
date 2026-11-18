import { cookies } from "next/headers";
import { notFound, redirect } from "next/navigation";
import type { DeveloperApp } from "@/lib/(app)/developer-apps";

async function requestApps(path: string) {
  const sid = (await cookies()).get("sid")?.value;
  if (!sid) {
    redirect("/");
  }

  const origin = process.env.API_URL || "http://localhost:8001";
  const response = await fetch(`${origin}/v1/developer/apps${path}`, {
    headers: { Cookie: `sid=${sid}` },
    cache: "no-store",
  });

  if (response.status === 401) {
    redirect("/");
  }

  if (response.status === 403) {
    redirect("/account/settings");
  }

  if (response.status === 404 && path) {
    notFound();
  }

  if (!response.ok) {
    throw new Error(`Developer apps request failed (${response.status})`);
  }

  return response;
}

export async function getDeveloperApps(): Promise<DeveloperApp[]> {
  const response = await requestApps("");
  return (await response.json()) as DeveloperApp[];
}

export async function getDeveloperApp(id: string): Promise<DeveloperApp> {
  if (!/^[a-f0-9]{24}$/i.test(id)) {
    notFound();
  }

  const response = await requestApps(`/${id}`);
  return (await response.json()) as DeveloperApp;
}
