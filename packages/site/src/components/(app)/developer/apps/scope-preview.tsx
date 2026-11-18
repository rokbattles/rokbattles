"use client";

import { useState } from "react";
import { Checkbox, CheckboxField } from "@/components/ui/checkbox";
import { Field, Fieldset, Label, Legend } from "@/components/ui/fieldset";
import { Subheading } from "@/components/ui/heading";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { Code, Text } from "@/components/ui/text";
import type { AppOidc } from "@/lib/(app)/developer-apps";
import { CopyValue } from "./copy-value";

export function ScopePreview({
  appId,
  redirectUris,
  oidc,
}: {
  appId: string;
  redirectUris: string[];
  oidc: AppOidc | null;
}) {
  const [selection, setSelection] = useState("");
  const [enabled, setEnabled] = useState(() => new Set<string>());
  if (!oidc) {
    return <Text>OpenID Connect is unavailable.</Text>;
  }

  const redirectUri = redirectUris.includes(selection) ? selection : redirectUris[0];
  const scopes = oidc.scopes.filter(({ id }) => id === "openid" || enabled.has(id));

  const parameters = new URLSearchParams({
    response_type: "code",
    client_id: appId,
    redirect_uri: redirectUri ?? "",
    scope: scopes.map(({ id }) => id).join(" "),
    state: "<state>",
    nonce: "<nonce>",
    code_challenge: "<code_challenge>",
    code_challenge_method: "S256",
  });

  return (
    <div className="space-y-6">
      <Fieldset>
        <Legend>Available scopes</Legend>
        <div className="mt-3 grid items-start gap-4 sm:grid-cols-3">
          {oidc.scopes.map(({ id }) => (
            <CheckboxField key={id}>
              <Checkbox
                checked={id === "openid" || enabled.has(id)}
                disabled={id === "openid"}
                onChange={(checked) =>
                  setEnabled((previous) => {
                    const next = new Set(previous);
                    if (checked) {
                      next.add(id);
                    } else {
                      next.delete(id);
                    }
                    return next;
                  })
                }
              />
              <Label>{id}</Label>
            </CheckboxField>
          ))}
        </div>
      </Fieldset>
      {redirectUri ? (
        <>
          <Field>
            <Label>Redirect URI</Label>
            <Listbox<string> value={redirectUri} onChange={setSelection}>
              {redirectUris.map((uri) => (
                <ListboxOption key={uri} value={uri}>
                  <ListboxLabel>{uri}</ListboxLabel>
                </ListboxOption>
              ))}
            </Listbox>
          </Field>
          <div className="space-y-2">
            <Subheading level={3}>Authorization URL</Subheading>
            <Text>
              Generate fresh <Code>state</Code>, <Code>nonce</Code>, and an S256 PKCE{" "}
              <Code>code_challenge</Code> for each authorization attempt and substitute them for the
              encoded placeholders.
            </Text>
            <CopyValue
              as="input"
              label="Authorization URL"
              value={`${oidc.authorizeUrl}?${parameters}`}
            />
          </div>
        </>
      ) : (
        <Text>Save a redirect URI to generate an authorization URL.</Text>
      )}
    </div>
  );
}
