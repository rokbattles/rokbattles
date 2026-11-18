import { ChevronLeft } from "lucide-react";
import { AppSettingsNav } from "@/components/(app)/developer/apps/app-settings-nav";
import { Heading } from "@/components/ui/heading";
import { Link } from "@/components/ui/link";
import { getDeveloperApp } from "@/lib/(app)/get-developer-apps";

export default async function Layout({ children, params }: LayoutProps<"/developer/apps/[id]">) {
  const { id } = await params;
  const app = await getDeveloperApp(id);

  return (
    <div className="space-y-4">
      <div className="space-y-3">
        <Link
          href="/developer/apps"
          className="inline-flex items-center gap-1 text-sm text-zinc-400 hover:text-white"
        >
          <ChevronLeft aria-hidden="true" className="size-4" />
          All apps
        </Link>
        <Heading className="wrap-anywhere">{app.name}</Heading>
      </div>
      <AppSettingsNav appId={app.id} />
      {children}
    </div>
  );
}
