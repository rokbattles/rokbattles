import { ProgramApplicationPrompt } from "@/components/(app)/account/settings/program-application-prompt";
import { SettingsPage } from "@/components/(app)/account/settings/settings-page";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { hasRole } from "@/lib/(app)/roles";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();
  const member = hasRole(user, "developer_program");

  return (
    <SettingsPage>
      <SettingsSection title="Developer Program">
        {!member ? <ProgramApplicationPrompt program="Developer" /> : null}
      </SettingsSection>
    </SettingsPage>
  );
}
