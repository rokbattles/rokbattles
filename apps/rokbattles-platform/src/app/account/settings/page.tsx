import { getTranslations } from "next-intl/server";
import {
  Description,
  Field,
  FieldGroup,
  Label,
} from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();
  const t = await getTranslations("account");
  return (
    <div className="mt-8 space-y-8">
      <FieldGroup>
        <Field>
          <Label htmlFor="account-email">{t("settings.emailLabel")}</Label>
          <Input
            disabled
            id="account-email"
            readOnly
            type="email"
            value={user.email}
          />
          <Description>{t("settings.emailDescription")}</Description>
        </Field>
      </FieldGroup>
    </div>
  );
}
