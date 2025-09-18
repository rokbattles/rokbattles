import { getTranslations } from "next-intl/server";
import type { CommanderInfo } from "@/lib/types/reports";

type Props = {
  info?: CommanderInfo;
  names: Record<string, string | undefined>;
};

export async function BattleCommanderSummary({ info, names }: Props) {
  const t = await getTranslations("battle");
  const id = info?.id;
  const nameKey = typeof id === "number" ? String(id) : undefined;
  const name = nameKey ? (names[nameKey] ?? nameKey) : undefined;
  const level = typeof info?.level === "number" ? info.level : undefined;
  const skills = info?.skills && info.skills.length > 0 ? info.skills : undefined;

  if (!name && !level && !skills) {
    return <div className="text-sm text-zinc-400">&mdash;</div>;
  }

  return (
    <div className="flex items-center justify-between">
      <div className="text-sm text-zinc-200">{name ?? "\u2014"}</div>
      <div className="flex items-center gap-1.5">
        {typeof level === "number" && (
          <span className="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold text-zinc-100 ring-1 ring-inset ring-white/20">
            {t("levelTag", { level })}
          </span>
        )}
        {skills && (
          <span className="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold text-zinc-100 ring-1 ring-inset ring-white/20">
            {skills}
          </span>
        )}
      </div>
    </div>
  );
}
