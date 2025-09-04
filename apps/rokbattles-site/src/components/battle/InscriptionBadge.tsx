import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

type Props = {
  color?: "gray" | "blue" | "amber";
  children: ReactNode;
};

export async function InscriptionBadge({ color, children }: Props) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-md font-semibold ring-1 ring-inset px-2 py-1 text-xs",
        color === "blue"
          ? "ring-blue-400/50"
          : color === "amber"
            ? "ring-amber-400/50"
            : "ring-white/20",
        "text-zinc-100"
      )}
    >
      {children}
    </span>
  );
}
