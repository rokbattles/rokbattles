"use client";

import { Description, Field, FieldGroup, Label } from "@/components/ui/Fieldset";
import { Input } from "@/components/ui/Input";
import { useCurrentUser } from "@/hooks/useCurrentUser";

export default function Page() {
  const { user, loading } = useCurrentUser();

  if (loading) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        Loading your account&hellip;
      </p>
    );
  }

  if (!user) {
    return (
      <p className="mt-8 text-sm text-zinc-500 dark:text-zinc-400" role="status" aria-live="polite">
        You must be logged in to view this page.
      </p>
    );
  }

  return (
    <div className="space-y-8 mt-8">
      <FieldGroup>
        <Field>
          <Label htmlFor="account-email">Email address</Label>
          <Input id="account-email" type="email" value={user.email} disabled readOnly />
          <Description>Your email address is synced from Discord.</Description>
        </Field>
      </FieldGroup>
    </div>
  );
}
