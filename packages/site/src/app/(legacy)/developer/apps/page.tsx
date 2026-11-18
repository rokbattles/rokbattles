import type { Metadata } from "next";
import { CreateAppDialog } from "@/components/(app)/developer/apps/create-app-dialog";
import { Heading } from "@/components/ui/heading";
import { Link } from "@/components/ui/link";
import { Text } from "@/components/ui/text";
import { getDeveloperApps } from "@/lib/(app)/get-developer-apps";

export const metadata: Metadata = { title: "Developer Apps" };

export default async function Page() {
  const apps = await getDeveloperApps();

  return (
    <div className="space-y-8">
      <div className="flex flex-wrap items-center justify-between gap-4">
        <div>
          <Heading>Apps</Heading>
          <Text>Register and manage your ROK Battles integrations.</Text>
        </div>
        <CreateAppDialog />
      </div>

      {apps.length === 0 ? (
        <div className="rounded-md border border-dashed border-white/15 px-6 py-10 text-center">
          <Text>You haven’t registered any apps yet.</Text>
        </div>
      ) : (
        <ul
          aria-label="Your apps"
          className="divide-y divide-white/10 overflow-hidden rounded-md border border-white/10"
        >
          {apps.map((app) => (
            <li key={app.id}>
              <Link
                href={`/developer/apps/${app.id}`}
                className="group flex items-center justify-between gap-4 px-4 py-4 hover:bg-white/5 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-blue-500 sm:px-6"
              >
                <div className="min-w-0 space-y-1">
                  <p className="break-words font-medium">{app.name}</p>
                  {app.description ? (
                    <Text className="line-clamp-2 break-words">{app.description}</Text>
                  ) : null}
                </div>
                <span aria-hidden="true" className="text-sm text-zinc-400 group-hover:text-white">
                  Manage
                </span>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
