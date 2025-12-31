"use client";

import { useTranslations } from "next-intl";
import { Description, Field, FieldGroup, Label } from "@/components/ui/Fieldset";
import { Input } from "@/components/ui/Input";
import { useCurrentUser } from "@/hooks/useCurrentUser";

export default function Page() {
  const t = useTranslations("account");
  const { user, loading } = useCurrentUser();

  if (loading) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        {t("states.loading")}
      </p>
    );
  }

  if (!user) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        {t("states.loginRequired")}
      </p>
    );
  }

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
