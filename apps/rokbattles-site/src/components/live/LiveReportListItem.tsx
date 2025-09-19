import { Swords } from "lucide-react";
import type { Locale } from "next-intl";
import { Link } from "@/components/ui/link";

type LiveReportListItemProps = {
  left: string;
  right: string;
  leftSecondary?: string;
  rightSecondary?: string;
  query: Record<string, string>;
  locale: Locale;
};

export function LiveReportListItem({
  left,
  right,
  leftSecondary,
  rightSecondary,
  query,
  locale,
}: LiveReportListItemProps) {
  return (
    <Link
      href={{ pathname: "/live", query }}
      locale={locale}
      className="flex items-center justify-between px-5 py-3 transition hover:bg-zinc-800"
    >
      <div className="w-36">
        <div className="truncate text-sm font-medium text-zinc-100">{left}</div>
        <div className="truncate text-[11px] text-zinc-400">{leftSecondary ?? ""}</div>
      </div>
      <div className="mx-2 flex shrink-0 items-center justify-center">
        <Swords className="size-5 text-zinc-300" aria-hidden="true" />
      </div>
      <div className="w-36 text-right">
        <div className="truncate text-sm font-medium text-zinc-100">{right}</div>
        <div className="truncate text-[11px] text-zinc-400">{rightSecondary ?? ""}</div>
      </div>
    </Link>
  );
}
