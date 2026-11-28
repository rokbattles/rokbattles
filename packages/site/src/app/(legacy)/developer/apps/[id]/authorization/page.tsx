import type { Metadata } from "next";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { AppAuthorization } from "@/components/(app)/developer/apps/app-authorization";
import { getDeveloperApp } from "@/lib/(app)/get-developer-apps";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export const metadata: Metadata = { title: "App Authorization" };

export default async function Page({ params }: PageProps<"/developer/apps/[id]/authorization">) {
  const { id } = await params;
  const app = await getDeveloperApp(id);

  return (
    <SettingsPage>
      <AppAuthorization
        appId={app.id}
        initialRedirectUris={app.redirectUris}
        initialOidc={app.oidc}
      />
    </SettingsPage>
  );
}
