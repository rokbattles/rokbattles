"use client";

import { ReportSkillSlot } from "@/components/report/report-skill-slot";
import { resolveLocale } from "@/i18n/locale";
import { getCommanderSkillName, getCommanderSkillSpriteUrls } from "@/lib/commander";
import type { RawAuxiliarySkillInfo, RawSupportSkillsInfo } from "@/lib/types/raw-report";

type ReportSupportSkillsProps = {
  supportSkills?: RawSupportSkillsInfo | null;
  auxiliarySkills?: RawAuxiliarySkillInfo[] | null;
};

export function ReportSupportSkills({ supportSkills, auxiliarySkills }: ReportSupportSkillsProps) {
  const locale = resolveLocale();
  const skills = supportSkills?.enable
    ? (supportSkills.skills ?? []).map((skill) => ({
        heroId: skill.hero_id,
        level: skill.skill_level,
        skillId: skill.skill_id,
      }))
    : (auxiliarySkills ?? []).map((skill) => ({
        heroId: skill.hero_id,
        level: skill.level,
        skillId: skill.skill_id,
      }));

  if (skills.length === 0) {
    return null;
  }

  return (
    <div className="flex justify-end">
      <div className="flex flex-wrap justify-end gap-1.5">
        {skills.map((skill) => {
          const spriteUrls = getCommanderSkillSpriteUrls(skill.heroId, skill.skillId);
          const skillName = getCommanderSkillName(skill.heroId, skill.skillId, locale);

          return (
            <ReportSkillSlot
              key={`${skill.heroId}-${skill.skillId}-${skill.level}`}
              spriteUrls={spriteUrls}
              level={skill.level}
              alt={skillName}
              title={skillName}
            />
          );
        })}
      </div>
    </div>
  );
}
