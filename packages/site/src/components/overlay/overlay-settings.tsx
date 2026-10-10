"use client";

import { useExtracted } from "next-intl";
import { type FormEvent, type JSX, use, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { Description, Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { Text } from "@/components/ui/text";
import type { OverlayPreferences } from "@/lib/types/overlay";
import { GovernorContext } from "@/providers/governor-context";

type OverlayConfig = {
  enabled: boolean;
  settings: OverlayPreferences;
};

type GovernorOverlaySettingsProps = { governorId: number };
type OverlayFormProps = { endpoint: string; config: OverlayConfig };

async function request<T = unknown>(path: string, method = "GET", body?: unknown): Promise<T> {
  const response = await fetch(path, {
    method,
    cache: "no-store",
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
  });

  if (!response.ok) {
    throw new Error("Unable to update overlay. Please try again.");
  }

  return response.json();
}

export function OverlaySettings(): JSX.Element {
  const governor = use(GovernorContext)?.activeGovernor;
  const t = useExtracted();

  if (!governor) {
    return <Text>{t("Select a governor to configure your stream overlay.")}</Text>;
  }

  return <GovernorOverlaySettings key={governor.governorId} governorId={governor.governorId} />;
}

function GovernorOverlaySettings({ governorId }: GovernorOverlaySettingsProps): JSX.Element {
  const t = useExtracted();
  const endpoint = `/proxy/v1/governor/${governorId}/overlay`;
  const [config, setConfig] = useState<OverlayConfig | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    let cancelled = false;

    request<OverlayConfig>(endpoint)
      .then((value) => {
        if (!cancelled) {
          setConfig(value);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setError(t("Unable to load overlay settings."));
        }
      });

    return () => {
      cancelled = true;
    };
  }, [endpoint, t]);

  if (error) {
    return <Text role="alert">{error}</Text>;
  }

  if (!config) {
    return <Text>{t("Loading overlay settings…")}</Text>;
  }

  return <OverlayForm endpoint={endpoint} config={config} />;
}

function OverlayForm({ endpoint, config }: OverlayFormProps): JSX.Element {
  const t = useExtracted();
  const [settings, setSettings] = useState(config.settings);
  const [enabled, setEnabled] = useState(config.enabled);
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  async function rotate(): Promise<void> {
    const { path } = await request<{ path: string }>(`${endpoint}/rotate`, "POST");

    setUrl(`https://rokbattles.com${path}`);
    setEnabled(true);
  }

  async function submit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    setBusy(true);
    setError("");

    try {
      await request(endpoint, "PUT", settings);

      if (!enabled) {
        await rotate();
      }
    } catch {
      setError(t("Unable to save overlay settings. Please try again."));
    } finally {
      setBusy(false);
    }
  }

  async function changeToken(action: "rotate" | "revoke"): Promise<void> {
    setBusy(true);
    setError("");

    try {
      if (action === "rotate") {
        await rotate();
      } else {
        await request(endpoint, "DELETE");

        setEnabled(false);
        setUrl("");
      }
    } catch {
      setError(t("Unable to update the overlay URL. Please try again."));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="max-w-2xl space-y-6">
      <form onSubmit={submit} className="space-y-5">
        <Field className="w-1/2">
          <Label>{t("Max reports")}</Label>
          <Listbox
            name="limit"
            value={settings.limit}
            onChange={(limit) => setSettings({ ...settings, limit })}
            disabled={busy}
          >
            {[1, 2, 3, 4, 5, 6, 7].map((count) => (
              <ListboxOption key={count} value={count}>
                <ListboxLabel>{count}</ListboxLabel>
              </ListboxOption>
            ))}
          </Listbox>
        </Field>

        <Field className="w-1/2">
          <Label>{t("Branding")}</Label>
          <Listbox
            name="branding"
            value={settings.branding ?? "text_watermark"}
            onChange={(branding) => setSettings({ ...settings, branding })}
            disabled={busy}
          >
            <ListboxOption value="icon_watermark">
              <ListboxLabel>{t("Icon watermark")}</ListboxLabel>
            </ListboxOption>
            <ListboxOption value="text_watermark">
              <ListboxLabel>{t("Text watermark")}</ListboxLabel>
            </ListboxOption>
          </Listbox>
        </Field>

        <Button type="submit" disabled={busy}>
          {enabled ? t("Save settings") : t("Create overlay URL")}
        </Button>
      </form>

      {url ? (
        <Field>
          <Label>{t("OBS browser source URL")}</Label>
          <Input readOnly value={url} onFocus={(event) => event.currentTarget.select()} />
          <Description>
            {t("Copy this URL into a Browser Source. Keep this URL private.")}
          </Description>
        </Field>
      ) : null}

      {enabled ? (
        <div className="space-y-3">
          <Text>{t("For your privacy, an existing URL is only shown when created.")}</Text>

          <div className="flex flex-wrap gap-3">
            <Button outline disabled={busy} onClick={() => void changeToken("rotate")}>
              {t("Replace URL")}
            </Button>
            <Button plain disabled={busy} onClick={() => void changeToken("revoke")}>
              {t("Revoke overlay")}
            </Button>
          </div>
        </div>
      ) : null}

      {error ? <Text role="alert">{error}</Text> : null}
    </div>
  );
}
