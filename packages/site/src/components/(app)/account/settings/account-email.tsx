"use client";

import { useState } from "react";
import { RevealButton } from "@/components/(app)/account/settings/reveal-button";

export function AccountEmail({ email }: { email: string }) {
  const [revealed, setRevealed] = useState(false);

  return (
    <div className="flex flex-wrap items-center justify-between gap-4 px-3 py-2 text-sm">
      <span>Email</span>
      <div className="flex min-w-0 items-center gap-3">
        <span id="account-email" className="break-all text-zinc-400">
          {revealed ? email : "********"}
        </span>
        <RevealButton
          controls="account-email"
          label="Email"
          revealed={revealed}
          onToggle={() => setRevealed(!revealed)}
        />
      </div>
    </div>
  );
}
