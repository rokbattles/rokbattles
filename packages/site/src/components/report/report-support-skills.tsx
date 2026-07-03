"use client";

import { useExtracted } from "next-intl";
import { ReportSkillSlot } from "@/components/report/report-skill-slot";
import { getCommanderSkillSpriteUrls } from "@/lib/commander";
import type { RawSupportSkillsInfo } from "@/lib/types/raw-report";

type ReportSupportSkillsProps = {
  supportSkills?: RawSupportSkillsInfo | null;
};

export function ReportSupportSkills({ supportSkills }: ReportSupportSkillsProps) {
  const t = useExtracted();

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
          const skillId = skill.skill_id.toString();

          return (
            <ReportSkillSlot
              key={`${skill.hero_id}-${skill.skill_id}-${skill.skill_level}`}
              spriteUrls={spriteUrls}
              level={skill.skill_level}
              alt={t("Support skill {id}", { id: skillId })}
              title={skillId}
            />
          );
        })}
      </div>
    </div>
  );
}
