import { AppSettingsNav } from "@/components/(app)/developer/apps/app-settings-nav";
import { Heading } from "@/components/ui/heading";
import { getDeveloperApp } from "@/lib/(app)/get-developer-apps";

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
