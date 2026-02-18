import { getExtracted } from "next-intl/server";
import { Description, Field, FieldGroup, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();
  const t = await getExtracted();
  return (
    <div className="space-y-8 mt-8">
      <FieldGroup>
        <Field>
          <Label htmlFor="account-email">{t("Email address")}</Label>
          <Input id="account-email" type="email" value={user.email} disabled readOnly />
          <Description>{t("Your email address is synced from Discord.")}</Description>
        </Field>
      </FieldGroup>
    </div>
  );
}
