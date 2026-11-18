import type { Metadata } from "next";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { AppAuthorization } from "@/components/(app)/developer/apps/app-authorization";
import { getDeveloperApp } from "@/lib/(app)/get-developer-apps";

export const metadata: Metadata = { title: "App Authorization" };

export default async function Page({ params }: PageProps<"/developer/apps/[id]/authorization">) {
  const { id } = await params;
  const app = await getDeveloperApp(id);

  return (
    <SettingsPage>
      <AppAuthorization appId={app.id} initialRedirectUris={app.redirectUris} />
    </SettingsPage>
  );
}
