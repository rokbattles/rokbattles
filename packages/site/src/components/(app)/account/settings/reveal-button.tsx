"use client";

import type { JSX } from "react";

type RevealButtonProps = {
  controls: string;
  label: string;
  revealed: boolean;
  onToggle: () => void;
};

export function RevealButton({
  controls,
  label,
  revealed,
  onToggle,
}: RevealButtonProps): JSX.Element {
  const action = revealed ? "Hide" : "Reveal";

  return (
    <button
      className="shrink-0 cursor-pointer text-sm text-white underline decoration-white/50 underline-offset-4 hover:decoration-white focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-white"
      type="button"
      aria-controls={controls}
      aria-expanded={revealed}
      aria-label={`${action} ${label}`}
      onClick={onToggle}
    >
      {action}
    </button>
  );
}
