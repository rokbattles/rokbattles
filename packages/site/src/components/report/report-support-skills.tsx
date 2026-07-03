"use client";

import { ReportSkillSlot } from "@/components/report/report-skill-slot";
import { resolveLocale } from "@/i18n/locale";
import { getCommanderSkillName, getCommanderSkillSpriteUrls } from "@/lib/commander";
import type { RawSupportSkillsInfo } from "@/lib/types/raw-report";

type ReportSupportSkillsProps = {
  supportSkills?: RawSupportSkillsInfo | null;
};

export function ReportSupportSkills({ supportSkills }: ReportSupportSkillsProps) {
  const locale = resolveLocale();

  if (!supportSkills?.enable) {
    return null;
  }

  const skills = supportSkills.skills ?? [];

  if (skills.length === 0) {
    return null;
  }

  return (
    <div className="flex justify-end">
      <div className="flex flex-wrap justify-end gap-1.5">
        {skills.map((skill) => {
          const spriteUrls = getCommanderSkillSpriteUrls(skill.hero_id, skill.skill_id);
          const skillName = getCommanderSkillName(skill.hero_id, skill.skill_id, locale);

          return (
            <ReportSkillSlot
              key={`${skill.hero_id}-${skill.skill_id}-${skill.skill_level}`}
              spriteUrls={spriteUrls}
              level={skill.skill_level}
              alt={skillName}
              title={skillName}
            />
          );
        })}
      </div>
    </div>
  );
}
