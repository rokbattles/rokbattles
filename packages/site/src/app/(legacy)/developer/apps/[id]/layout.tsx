import { AppSettingsNav } from "@/components/(app)/developer/apps/app-settings-nav";
import { Heading } from "@/components/ui/heading";
import { getDeveloperApp } from "@/lib/(app)/get-developer-apps";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Layout({ children, params }: LayoutProps<"/developer/apps/[id]">) {
  const { id } = await params;
  const app = await getDeveloperApp(id);

  return (
    <div className="space-y-4">
      <Heading className="wrap-anywhere">{app.name}</Heading>
      <AppSettingsNav appId={app.id} />
      {children}
    </div>
  );
}
