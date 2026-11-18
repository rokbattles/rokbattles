"use client";

import { useRouter } from "next/navigation";
import { type FormEvent, useState } from "react";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";
import { Description, Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Code, Text } from "@/components/ui/text";
import { Textarea } from "@/components/ui/textarea";
import { type DeveloperApp, updateDeveloperApp } from "@/lib/(app)/developer-apps";
import { CopyValue } from "./copy-value";

export function AppSettings({ initialApp }: { initialApp: DeveloperApp }) {
  const router = useRouter();
  const [pending, setPending] = useState(false);
  const [result, setResult] = useState<{ message: string; error: boolean } | null>(null);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const data = new FormData(event.currentTarget);
    setPending(true);
    setResult(null);

    try {
      await updateDeveloperApp(initialApp.id, {
        section: "general",
        name: String(data.get("name") ?? ""),
        description: String(data.get("description") ?? "") || null,
        termsOfServiceUrl: String(data.get("termsOfServiceUrl") ?? "") || null,
        privacyPolicyUrl: String(data.get("privacyPolicyUrl") ?? "") || null,
      });
      router.refresh();
      setResult({ message: "App settings saved.", error: false });
    } catch (error) {
      setResult({
        message: error instanceof Error ? error.message : "App settings could not be saved.",
        error: true,
      });
    } finally {
      setPending(false);
    }
  }

  return (
    <>
      <SettingsSection title="General Information">
        <form onSubmit={submit} className="space-y-6">
          <Field disabled={pending}>
            <Label>App name</Label>
            <Input name="name" defaultValue={initialApp.name} required />
          </Field>
          <Field disabled={pending}>
            <Label>Description</Label>
            <Description>Optional. Up to 1,000 characters.</Description>
            <Textarea name="description" defaultValue={initialApp.description ?? ""} rows={3} />
          </Field>
          <Field disabled={pending}>
            <Label>Terms of Service URL</Label>
            <Description>Optional HTTPS link to your app’s terms.</Description>
            <Input
              type="url"
              name="termsOfServiceUrl"
              defaultValue={initialApp.termsOfServiceUrl ?? ""}
            />
          </Field>
          <Field disabled={pending}>
            <Label>Privacy Policy URL</Label>
            <Description>Optional HTTPS link to your app’s privacy policy.</Description>
            <Input
              type="url"
              name="privacyPolicyUrl"
              defaultValue={initialApp.privacyPolicyUrl ?? ""}
            />
          </Field>
          <div className="flex flex-wrap items-center gap-3">
            <Button type="submit" disabled={pending}>
              {pending ? "Saving…" : "Save settings"}
            </Button>
            {result ? (
              <p
                role={result.error ? "alert" : "status"}
                className={result.error ? "text-sm text-red-400" : "text-sm text-zinc-400"}
              >
                {result.message}
              </p>
            ) : null}
          </div>
        </form>
      </SettingsSection>
      <Divider />
      <SettingsSection title="Integration Details">
        <dl className="space-y-5">
          <div className="space-y-2">
            <dt className="text-sm font-medium">App ID</dt>
            <dd>
              <CopyValue label="app ID" value={initialApp.id} />
            </dd>
          </div>
          <div className="space-y-2">
            <dt className="text-sm font-medium">User-Agent</dt>
            <dd className="space-y-2">
              <CopyValue label="User-Agent" value={initialApp.userAgent} />
              <Text>
                Use this header for future API requests made on behalf of users. Inside the
                parentheses, you can append <Code>/version</Code> and{" "}
                <Code>; platform information</Code> to the app name.
              </Text>
            </dd>
          </div>
        </dl>
      </SettingsSection>
    </>
  );
}
