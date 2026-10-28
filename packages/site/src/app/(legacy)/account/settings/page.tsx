import { Description, Field, FieldGroup, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { requireCurrentUser } from "@/lib/require-user";

export default async function Page() {
  const user = await requireCurrentUser();

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
