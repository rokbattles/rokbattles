"use client";

import { type JSX, useId, useState } from "react";
import { RevealButton } from "@/components/(app)/account/settings/reveal-button";
import { Button } from "@/components/ui/button";
import { Field, Label } from "@/components/ui/fieldset";
import { Input } from "@/components/ui/input";
import { Text } from "@/components/ui/text";
import { useCopyToClipboard } from "@/hooks/use-copy-to-clipboard";

type SettingsSecretInputProps = {
  label: string;
  value: string;
};

export function SettingsSecretInput({ label, value }: SettingsSecretInputProps): JSX.Element {
  const id = useId();
  const [revealed, setRevealed] = useState(false);
  const [message, setMessage] = useState("");
  const [, copyToClipboard] = useCopyToClipboard();

  async function copy(): Promise<void> {
    const copied = await copyToClipboard(value);

    if (copied) {
      setMessage(`${label} copied.`);
    } else {
      setRevealed(true);
      setMessage("Unable to copy. Select the value and copy it manually.");
    }
  }

  return (
    <Field className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <Label>{label}</Label>
        <RevealButton
          controls={id}
          label={label}
          revealed={revealed}
          onToggle={() => setRevealed(!revealed)}
        />
      </div>
      <div className="flex min-w-0 gap-3">
        <Input
          id={id}
          readOnly
          type={revealed ? "text" : "password"}
          value={value}
          autoComplete="off"
          onFocus={(event) => event.currentTarget.select()}
        />
        <Button outline onClick={() => void copy()} aria-label={`Copy ${label}`}>
          Copy
        </Button>
      </div>
      {message ? <Text role="status">{message}</Text> : null}
    </Field>
  );
}
