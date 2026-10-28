"use client";

import { ChevronDown } from "lucide-react";
import { useExtracted } from "next-intl";
import { Subheading } from "@/components/ui/heading";
import { getSupremeStrifeTalents } from "@/lib/supreme-strife-talents";

type ReportSupremeStrifeTalentSectionProps = {
  talentIds?: readonly number[];
};

export function ReportSupremeStrifeTalentSection({
  talentIds,
}: ReportSupremeStrifeTalentSectionProps) {
  const t = useExtracted();
  const talents = getSupremeStrifeTalents(talentIds);

  if (talents.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>{t("Supreme Strife Talents")}</Subheading>
      <div className="space-y-3">
        {talents.map((talent) => (
          <details className="group" key={talent.id}>
            <summary className="flex cursor-pointer list-none items-start justify-between gap-3 rounded-sm focus-visible:outline-2 focus-visible:outline-blue-500 focus-visible:outline-offset-2 [&::-webkit-details-marker]:hidden">
              <span className="font-medium text-base/6 sm:text-sm/6 text-white">{talent.name}</span>
              <ChevronDown
                data-slot="icon"
                aria-hidden="true"
                className="mt-1 size-4 shrink-0 transition-transform group-open:rotate-180 text-zinc-500"
              />
            </summary>
            <p className="mt-2 whitespace-pre-line text-base/6 sm:text-sm/6 text-zinc-300">
              {talent.description}
            </p>
          </details>
        ))}
      </div>
    </div>
  );
}
