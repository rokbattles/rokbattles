"use client";

import { useExtracted } from "next-intl";
import { CommanderIcon } from "@/components/commander-icon";
import { ReportSkillSlot } from "@/components/report/report-skill-slot";
import { Badge } from "@/components/ui/badge";
import { Strong } from "@/components/ui/text";
import { getCommanderName } from "@/hooks/use-commander-name";
import { type CommanderSkillLevel, getCommanderSkillDisplays } from "@/lib/commander";

type CommanderLoadoutRowProps = {
  id?: number | null;
  awakened?: boolean | null;
  level?: number | null;
  skills?: readonly CommanderSkillLevel[] | null;
  formationLabel?: string | null;
};

export function CommanderLoadoutRow({
  id,
  awakened,
  level,
  skills,
  formationLabel,
}: CommanderLoadoutRowProps) {
  const t = useExtracted();
  const commanderName = getCommanderName(id ?? null);
  const commanderLabel = commanderName ?? id ?? t("Unknown");
  const commanderAlt = t("{name} icon", { name: commanderLabel.toString() });
  const skillDisplays = getCommanderSkillDisplays(id, skills, awakened);
  const levelLabel = typeof level === "number" && Number.isFinite(level) ? level : null;

  return (
    <div className="grid grid-cols-1 items-center gap-2 sm:grid-cols-[minmax(0,1fr)_auto] sm:gap-3">
      <div className="flex min-w-0 items-center gap-2">
        <CommanderIcon
          alt={commanderAlt}
          awakened={awakened}
          className="size-12 outline-0!"
          id={id}
          sizes="48px"
        />
        <div className="min-w-0">
          <Strong className="block truncate text-sm">{commanderLabel}</Strong>
          <div className="mt-1 flex flex-wrap items-center gap-1.5">
            {formationLabel ? <Badge>{formationLabel}</Badge> : null}
            {levelLabel != null ? (
              <Badge>{t("Lvl {level}", { level: levelLabel.toString() })}</Badge>
            ) : null}
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
