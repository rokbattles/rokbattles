"use client";

import { Plus, X } from "lucide-react";
import { type FormEvent, useRef, useState } from "react";
import { SettingsSection } from "@/components/(app)/account/settings/settings-section";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";
import { Input } from "@/components/ui/input";
import { type AppOidc, updateDeveloperApp } from "@/lib/(app)/developer-apps";
import { ScopePreview } from "./scope-preview";

const MAX_REDIRECT_URIS = 10;

export function AppAuthorization({
  appId,
  initialRedirectUris,
  initialOidc,
}: {
  appId: string;
  initialRedirectUris: string[];
  initialOidc: AppOidc | null;
}) {
  const [savedUris, setSavedUris] = useState(initialRedirectUris);
  const [oidc, setOidc] = useState(initialOidc);
  const [rows, setRows] = useState(() =>
    (initialRedirectUris.length ? initialRedirectUris : [""]).map((uri, index) => ({
      id: `initial-${index}`,
      uri,
    }))
  );
  const nextRowId = useRef(0);
  const [pending, setPending] = useState(false);
  const [result, setResult] = useState<{ message: string; error: boolean } | null>(null);
  const redirectUris = rows.map(({ uri }) => uri.trim()).filter(Boolean);
  const changed =
    redirectUris.length !== savedUris.length ||
    redirectUris.some((uri, index) => uri !== savedUris[index]);

  function addRedirect() {
    const id = `added-${nextRowId.current++}`;
    setRows((previous) => [...previous, { id, uri: "" }]);
    setResult(null);
  }

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setPending(true);
    setResult(null);

    try {
      const updated = await updateDeveloperApp(appId, { section: "authorization", redirectUris });
      setSavedUris(updated.redirectUris);
      setOidc(updated.oidc);
      setResult({ message: "Redirect URIs saved.", error: false });
    } catch (error) {
      setResult({
        message: error instanceof Error ? error.message : "Redirect URIs could not be saved.",
        error: true,
      });
    } finally {
      setPending(false);
    }
  }

  return (
    <>
      <SettingsSection
        title="Redirect URIs"
        description={`Register up to ${MAX_REDIRECT_URIS} callback URLs.`}
      >
        <form onSubmit={submit} className="space-y-4">
          <div className="space-y-2">
            {rows.map(({ id, uri }, index) => (
              <div key={id} className="flex items-center gap-2">
                <Input
                  type="url"
                  aria-label={`Redirect URI ${index + 1}`}
                  placeholder="https://example.com/callback"
                  value={uri}
                  disabled={pending}
                  onChange={(event) => {
                    const value = event.target.value;
                    setRows((previous) =>
                      previous.map((row) => (row.id === id ? { ...row, uri: value } : row))
                    );
                    setResult(null);
                  }}
                />
                <Button
                  type="button"
                  plain
                  aria-label={`Remove redirect URI ${index + 1}`}
                  disabled={pending}
                  onClick={() => {
                    setRows((previous) => previous.filter((row) => row.id !== id));
                    setResult(null);
                  }}
                >
                  <X data-slot="icon" aria-hidden="true" />
                </Button>
              </div>
            ))}
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <Button
              type="button"
              outline
              disabled={pending || rows.length >= MAX_REDIRECT_URIS}
              onClick={addRedirect}
            >
              <Plus data-slot="icon" aria-hidden="true" />
              Add redirect URI
            </Button>
            <Button type="submit" disabled={pending || !changed}>
              {pending ? "Saving…" : "Save redirect URIs"}
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
      <SettingsSection
        title="OpenID Connect"
        description="Use the available scopes to build an authorization request."
      >
        <ScopePreview appId={appId} redirectUris={savedUris} oidc={oidc} />
      </SettingsSection>
    </>
  );
}
