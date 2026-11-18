import type { Metadata } from "next";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { AppSettings } from "@/components/(app)/developer/apps/app-settings";
import { getDeveloperApp } from "@/lib/(app)/get-developer-apps";

export const metadata: Metadata = { title: "App Settings" };

export default async function Page({ params }: PageProps<"/developer/apps/[id]">) {
  const { id } = await params;
  const app = await getDeveloperApp(id);

  return (
    <SettingsPage>
      <AppSettings initialApp={app} />
    </SettingsPage>
  );
}
