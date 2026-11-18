"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useCopyToClipboard } from "@/hooks/use-copy-to-clipboard";

export function CopyValue({
  label,
  value,
  as = "code",
}: {
  label: string;
  value: string;
  as?: "code" | "input";
}) {
  const [, copy] = useCopyToClipboard();
  const [status, setStatus] = useState<{ value: string; message: string } | null>(null);

  async function copyValue() {
    const copied = await copy(value);
    setStatus({
      value,
      message: copied ? "Copied." : "Couldn’t copy. Select and copy the text manually.",
    });
  }

  return (
    <div>
      <div className="flex flex-wrap items-center gap-3">
        {as === "input" ? (
          <Input
            className="min-w-0 flex-1"
            aria-label={label}
            value={value}
            readOnly
            onFocus={(event) => event.currentTarget.select()}
          />
        ) : (
          <code className="min-w-0 flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm wrap-anywhere">
            {value}
          </code>
        )}
        <Button type="button" outline onClick={copyValue} aria-label={`Copy ${label}`}>
          Copy
        </Button>
      </div>
      <p role="status" className="text-sm text-zinc-400">
        {status?.value === value ? status.message : ""}
      </p>
    </div>
  );
}
