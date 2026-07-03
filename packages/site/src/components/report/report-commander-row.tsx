"use client";

import { useExtracted } from "next-intl";
import { CommanderIcon } from "@/components/commander-icon";
import { ReportSkillSlot } from "@/components/report/report-skill-slot";
import { Badge } from "@/components/ui/badge";
import { Strong } from "@/components/ui/text";
import { getCommanderName } from "@/hooks/use-commander-name";
import { getFormationName } from "@/hooks/use-formation-name";
import { getCommanderSkillDisplays } from "@/lib/commander";
import type { RawCommanderInfo } from "@/lib/types/raw-report";

type ReportCommanderRowProps = {
  commander?: RawCommanderInfo;
  formation?: number | null;
};

export function ReportCommanderRow({ commander, formation }: ReportCommanderRowProps) {
  const t = useExtracted();
  const commanderId = commander?.id;
  const commanderName = getCommanderName(commanderId ?? null);
  const formationName = getFormationName(formation ?? null);
  const level = typeof commander?.level === "number" ? commander.level : null;
  const commanderLabel = commanderName ?? commanderId ?? t("Unknown");
  const commanderAlt = t("{name} icon", { name: commanderLabel.toString() });
  const skillDisplays = getCommanderSkillDisplays(
    commanderId,
    commander?.skills,
    commander?.awakened
  );
  const formationLabel =
    typeof formation === "number"
      ? (formationName ?? t("Formation {formation}", { formation: formation.toString() }))
      : null;

  return (
    <div className="grid grid-cols-1 items-center gap-2 sm:grid-cols-[minmax(0,1fr)_auto] sm:gap-3">
      <div className="flex min-w-0 items-center gap-2">
        <CommanderIcon
          alt={commanderAlt}
          awakened={commander?.awakened}
          className="size-12 outline-0!"
          id={commanderId}
          sizes="48px"
        />
        <div className="min-w-0">
          <Strong className="block truncate text-sm">{commanderLabel}</Strong>
          <div className="mt-1 flex flex-wrap items-center gap-1.5">
            {formationLabel ? <Badge>{formationLabel}</Badge> : null}
            {level != null ? <Badge>{t("Lvl {level}", { level: level.toString() })}</Badge> : null}
          </div>
        </div>
      </div>
      {skillDisplays.length > 0 ? (
        <div className="flex flex-wrap justify-end gap-1.5 sm:flex-nowrap">
          {skillDisplays.map((skill) => (
            <ReportSkillSlot
              key={`${skill.id}-${skill.expert ? "expert" : "base"}`}
              spriteUrls={skill.spriteUrls}
              level={skill.level}
              alt={t("Skill {id}", { id: skill.id.toString() })}
              title={skill.id.toString()}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}
