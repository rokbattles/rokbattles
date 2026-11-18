import { DeauthorizeApp } from "@/components/(app)/account/settings/deauthorize-app";
import { SettingsEmptyState } from "@/components/(app)/account/settings/settings-empty-state";
import { Subheading } from "@/components/ui/heading";
import { Text, TextLink } from "@/components/ui/text";

import type { AuthorizedApp } from "@/lib/(app)/authorized-apps";

const date = new Intl.DateTimeFormat("en", { dateStyle: "medium", timeZone: "UTC" });

export function AuthorizedApps({ apps }: { apps: AuthorizedApp[] | null }) {
  if (!apps) {
    return <SettingsEmptyState>Authorized apps could not be loaded.</SettingsEmptyState>;
  }
  if (!apps.length) {
    return <SettingsEmptyState>No authorized apps.</SettingsEmptyState>;
  }

  return (
    <div className="space-y-4">
      {apps.map((app) => (
        <article key={app.id} className="space-y-4 rounded-lg border border-white/10 p-5">
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <Subheading level={3}>{app.name}</Subheading>
            <Text>Authorized on {date.format(app.authorizedOn)}</Text>
          </div>
          {app.termsUrl || app.privacyUrl ? (
            <div className="flex flex-wrap gap-4 text-sm">
              {app.termsUrl ? (
                <TextLink href={app.termsUrl} target="_blank" rel="noopener noreferrer">
                  Terms of Service
                </TextLink>
              ) : null}
              {app.privacyUrl ? (
                <TextLink href={app.privacyUrl} target="_blank" rel="noopener noreferrer">
                  Privacy Policy
                </TextLink>
              ) : null}
            </div>
          ) : null}
          <Text>
            Permissions · {app.permissions.length}{" "}
            {app.permissions.length === 1 ? "permission" : "permissions"}
          </Text>
          <ul className="list-disc space-y-1 pl-5 text-sm text-zinc-400">
            {app.permissions.map((permission) => (
              <li key={permission.id}>{permission.description}</li>
            ))}
          </ul>
          <DeauthorizeApp id={app.id} name={app.name} />
        </article>
      ))}
    </div>
  );
}
