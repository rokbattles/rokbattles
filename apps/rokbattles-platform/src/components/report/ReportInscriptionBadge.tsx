import { getInscriptionName } from "@/hooks/useInscriptionName";
import { cn } from "@/lib/cn";
import { getInscriptionRarity } from "@/lib/report/parsers";

type ReportInscriptionBadgeProps = {
  id: number;
};

export function ReportInscriptionBadge({ id }: ReportInscriptionBadgeProps) {
  const name = getInscriptionName(id ?? null);
  const rarity = getInscriptionRarity(id);
  const color = rarity === "special" ? "amber" : rarity === "rare" ? "blue" : "gray";

  return (
    <div className="relative flex h-5 w-28 select-none items-center justify-center text-xs font-semibold">
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
          "relative z-10 truncate px-2 text-center leading-none",
          color === "amber" && "text-[rgb(217,98,0)]",
          color === "blue" && "text-[rgb(57,99,255)]",
          color === "gray" && "text-[rgb(68,68,68)]"
        )}
      >
        {name ?? id}
      </span>
    </div>
  );
}
