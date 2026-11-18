"use client";

import { useState } from "react";
import { Checkbox, CheckboxField } from "@/components/ui/checkbox";
import { Description, Field, Fieldset, Label, Legend } from "@/components/ui/fieldset";
import { Subheading } from "@/components/ui/heading";
import { Select } from "@/components/ui/select";
import { Code, Text } from "@/components/ui/text";
import { CopyValue } from "./copy-value";

type OptionalScope = "profile" | "email";

const authorizeEndpoint = "https://rokbattles.com/proxy/v1/auth/authorize";

const optionalScopes: { value: OptionalScope; description: string }[] = [
  { value: "profile", description: "Requests standard profile claims." },
  { value: "email", description: "Requests email and email verification claims." },
];

export function ScopePreview({ appId, redirectUris }: { appId: string; redirectUris: string[] }) {
  const [selection, setSelection] = useState("");
  const redirectUri = redirectUris.includes(selection) ? selection : redirectUris[0];
  const [enabled, setEnabled] = useState({ profile: false, email: false });
  const scopes = [
    "openid",
    ...optionalScopes.filter(({ value }) => enabled[value]).map(({ value }) => value),
  ];

  const parameters = new URLSearchParams({
    response_type: "code",
    client_id: appId,
    redirect_uri: redirectUri ?? "",
    scope: scopes.join(" "),
    state: "<state>",
    nonce: "<nonce>",
    code_challenge: "<code_challenge>",
    code_challenge_method: "S256",
  });

  return (
    <div className="space-y-6">
      <Fieldset>
        <Legend>Identity scopes</Legend>
        <div className="mt-3 grid items-start gap-4 sm:grid-cols-3">
          <CheckboxField>
            <Checkbox checked disabled />
            <Label>openid</Label>
            <Description>Required for OpenID Connect sign-in.</Description>
          </CheckboxField>
          {optionalScopes.map(({ value, description }) => (
            <CheckboxField key={value}>
              <Checkbox
                checked={enabled[value]}
                onChange={(checked) =>
                  setEnabled((previous) => ({ ...previous, [value]: checked }))
                }
              />
              <Label>{value}</Label>
              <Description>{description}</Description>
            </CheckboxField>
          ))}
        </div>
      </Fieldset>
      {redirectUri ? (
        <>
          <Field>
            <Label>Redirect URI</Label>
            <Select value={redirectUri} onChange={(event) => setSelection(event.target.value)}>
              {redirectUris.map((uri) => (
                <option key={uri} value={uri}>
                  {uri}
                </option>
              ))}
            </Select>
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
              value={`${authorizeEndpoint}?${parameters}`}
            />
          </div>
        </>
      ) : (
        <Text>Save a redirect URI to generate an authorization URL.</Text>
      )}
    </div>
  );
}
