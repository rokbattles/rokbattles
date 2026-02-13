import { getTranslations } from "next-intl/server";
import { Description, Field, FieldGroup, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();
  const t = await getTranslations("account");
  return (
    <div className="space-y-8 mt-8">
      <FieldGroup>
        <Field>
          <Label htmlFor="account-email">{t("settings.emailLabel")}</Label>
          <Input id="account-email" type="email" value={user.email} disabled readOnly />
          <Description>{t("settings.emailDescription")}</Description>
        </Field>
      </FieldGroup>
    </div>
  );
}
