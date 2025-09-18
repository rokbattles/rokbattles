import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

type Props = {
  color?: "gray" | "blue" | "amber";
  children: ReactNode;
};

export async function InscriptionBadge({ color, children }: Props) {
  return (
    <div className="relative flex items-center justify-center font-semibold text-xs w-28 h-5">
      <div
        className={cn(
          "absolute inset-0 [clip-path:polygon(90%_0%,_100%_50%,_90%_100%,_10%_100%,_0%_50%,_10%_0%)]",
          color === "amber" &&
            "bg-[rgb(217,98,0)] bg-gradient-to-b from-[rgb(255,255,122)] to-[rgb(241,81,0)]",
          color === "blue" &&
            "bg-[rgb(57,99,255)] bg-gradient-to-b from-[rgb(192,229,253)] to-[rgb(57,99,255)]",
          color === "gray" &&
            "bg-[rgb(68,68,68)] bg-gradient-to-b from-[rgb(231,231,231)] to-[rgb(77,77,77)]"
        )}
      />
      <div
        className={cn(
          "absolute inset-px [clip-path:polygon(90%_0%,_100%_50%,_90%_100%,_10%_100%,_0%_50%,_10%_0%)]",
          color === "amber" && "bg-gradient-to-b from-[rgb(255,255,123)] to-[rgb(255,217,44)]",
          color === "blue" && "bg-gradient-to-b from-[rgb(207,237,255)] to-[rgb(160,192,255)]",
          color === "gray" && "bg-gradient-to-b from-[rgb(229,230,230)] to-[rgb(231,231,231)]"
        )}
      />
      <span
        className={cn(
          "relative z-10 truncate leading-none text-center px-2",
          color === "amber" && "text-[rgb(217,98,0)]",
          color === "blue" && "text-[rgb(57,99,255)]",
          color === "gray" && "text-[rgb(68,68,68)]"
        )}
      >
        {children}
      </span>
    </div>
  );
}
