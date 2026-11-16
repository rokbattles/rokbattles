"use client";

import { useState } from "react";

export function AccountEmail({ email }: { email: string }) {
  const [revealed, setRevealed] = useState(false);

  return (
    <div className="flex flex-wrap items-center justify-between gap-4 px-3 py-2 text-sm">
      <span>Email</span>
      <div className="flex min-w-0 items-center gap-3">
        <span id="account-email" className="break-all text-zinc-400">
          {revealed ? email : "********"}
        </span>
        <button
          className="shrink-0 cursor-pointer text-white underline decoration-white/50 underline-offset-4 hover:decoration-white focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-white"
          type="button"
          aria-controls="account-email"
          aria-expanded={revealed}
          onClick={() => setRevealed(!revealed)}
        >
          {revealed ? "Hide" : "Reveal"}
        </button>
      </div>
    </div>
  );
}
