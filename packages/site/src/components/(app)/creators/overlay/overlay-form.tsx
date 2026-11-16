"use client";

import { type JSX, useState } from "react";
import { OverlayConnection } from "@/components/(app)/creators/overlay/overlay-connection";
import { OverlayPreview } from "@/components/(app)/creators/overlay/overlay-preview";
import { OverlayUrlConfirmation } from "@/components/(app)/creators/overlay/overlay-url-confirmation";
import { Button } from "@/components/ui/button";
import { Field, Label } from "@/components/ui/fieldset";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { Text } from "@/components/ui/text";
import { useOverlayForm } from "@/hooks/(app)/use-overlay-form";
import type { OverlayConfig } from "@/lib/(app)/stream-overlay";
import type { OverlayCommander, OverlayUrlAction } from "@/lib/(app)/types/overlay";

type OverlayFormProps = {
  governorId: number;
  config: OverlayConfig;
  commanders: Record<string, OverlayCommander>;
};

export function OverlayForm({ governorId, config, commanders }: OverlayFormProps): JSX.Element {
  const { settings, setSettings, enabled, url, dirty, pending, error, message, save, changeUrl } =
    useOverlayForm(governorId, config);
  const [confirmation, setConfirmation] = useState<OverlayUrlAction>("rotate");
  const [confirmationOpen, setConfirmationOpen] = useState(false);

  const busy = pending !== null;
  const saveLabel = enabled ? "Save settings" : "Create overlay URL";
  const submitLabel = pending === "save" ? "Saving…" : saveLabel;

  function openConfirmation(action: OverlayUrlAction): void {
    setConfirmation(action);
    setConfirmationOpen(true);
  }

  async function confirmUrlChange(): Promise<void> {
    await changeUrl(confirmation);
    setConfirmationOpen(false);
  }

  return (
    <div className="grid items-start gap-8 xl:grid-cols-[minmax(0,1fr)_minmax(0,430px)]">
      <div className="min-w-0 space-y-8">
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void save();
          }}
          className="space-y-5"
        >
          <div className="grid gap-5 sm:grid-cols-2 xl:grid-cols-1 2xl:grid-cols-2">
            <Field>
              <Label>Max pairings</Label>
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
            <Field>
              <Label>Branding</Label>
              <Listbox
                name="branding"
                value={settings.branding}
                onChange={(branding) => setSettings({ ...settings, branding })}
                disabled={busy}
              >
                <ListboxOption value="icon_watermark">
                  <ListboxLabel>Icon watermark</ListboxLabel>
                </ListboxOption>
                <ListboxOption value="text_watermark">
                  <ListboxLabel>Text watermark</ListboxLabel>
                </ListboxOption>
              </Listbox>
            </Field>
          </div>
          <div className="flex flex-wrap items-center gap-3">
            <Button type="submit" disabled={busy || (enabled && !dirty)}>
              {submitLabel}
            </Button>
            {dirty ? <Text>Unsaved changes</Text> : null}
          </div>
        </form>
        <OverlayConnection url={url} enabled={enabled} busy={busy} onConfirm={openConfirmation} />
        {error ? <Text role="alert">{error}</Text> : null}
        <Text role="status">{message}</Text>
      </div>
      <OverlayPreview settings={settings} commanders={commanders} />
      <OverlayUrlConfirmation
        action={confirmation}
        open={confirmationOpen}
        busy={busy}
        onClose={() => setConfirmationOpen(false)}
        onConfirm={() => void confirmUrlChange()}
      />
    </div>
  );
}
