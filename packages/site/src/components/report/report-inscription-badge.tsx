import { useId } from "react";
import { getInscriptionName } from "@/hooks/use-inscription-name";
import { getInscriptionRarity } from "@/lib/report/parsers";

type ReportInscriptionBadgeProps = {
  id: number;
};

const palettes = {
  special: {
    outer: ["rgb(255,255,122)", "rgb(241,81,0)"],
    inner: ["rgb(255,255,123)", "rgb(255,217,44)"],
    text: "text-[rgb(217,98,0)]",
  },
  rare: {
    outer: ["rgb(192,229,253)", "rgb(57,99,255)"],
    inner: ["rgb(207,237,255)", "rgb(160,192,255)"],
    text: "text-[rgb(57,99,255)]",
  },
  common: {
    outer: ["rgb(231,231,231)", "rgb(77,77,77)"],
    inner: ["rgb(229,230,230)", "rgb(231,231,231)"],
    text: "text-[rgb(68,68,68)]",
  },
} as const;

export function ReportInscriptionBadge({ id }: ReportInscriptionBadgeProps) {
  const gradientId = useId();
  const name = getInscriptionName(id);
  const palette = palettes[getInscriptionRarity(id)];

  const label = name ?? id.toString();

  return (
    <div className="relative flex h-5 w-28 select-none items-center justify-center text-xs font-semibold">
      <svg
        aria-hidden="true"
        className="absolute inset-0 size-full"
        preserveAspectRatio="none"
        viewBox="0 0 112 20"
      >
        <defs>
          <linearGradient
            id={`${gradientId}-outer`}
            x1="0"
            x2="0"
            y1="0"
            y2="20"
            gradientUnits="userSpaceOnUse"
          >
            <stop stopColor={palette.outer[0]} />
            <stop offset="1" stopColor={palette.outer[1]} />
          </linearGradient>
          <linearGradient
            id={`${gradientId}-inner`}
            x1="0"
            x2="0"
            y1="0"
            y2="20"
            gradientUnits="userSpaceOnUse"
          >
            <stop stopColor={palette.inner[0]} />
            <stop offset="1" stopColor={palette.inner[1]} />
          </linearGradient>
        </defs>
        <path
          d="M 11.2 0 H 100.8 L 112 10 L 100.8 20 H 11.2 L 0 10 Z"
          fill={`url(#${gradientId}-outer)`}
        />
        <path d="M 12 1 H 100 L 111 10 L 100 19 H 12 L 1 10 Z" fill={`url(#${gradientId}-inner)`} />
      </svg>
      <span
        className={`relative z-10 truncate px-2 py-1 text-center leading-[12px] ${palette.text}`}
        title={label}
      >
        {label}
      </span>
    </div>
  );
}
