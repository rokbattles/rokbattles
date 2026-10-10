"use client";

import { useExtracted, useLocale } from "next-intl";
import { ConfidenceScore } from "@/components/combat-lab/confidence-score";
import { DrastcCredits } from "@/components/combat-lab/drastc-credits";
import { DrastcRadarChart } from "@/components/combat-lab/drastc-radar-chart";
import { Heading, Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import type { CombatLabPreviewDrastc as DrastcData } from "@/lib/combat-lab/preview-types";

export function CombatLabDrastc({ score }: { score: DrastcData }) {
  const t = useExtracted();
  const locale = useLocale();
  const numberFormatter = new Intl.NumberFormat(locale);
  const scoreFormatter = new Intl.NumberFormat(locale, {
    maximumFractionDigits: 2,
    minimumFractionDigits: 2,
  });
  const categories = [
    { key: "damage", axis: "D", label: t("Damage") },
    { key: "rage", axis: "R", label: t("Rage") },
    { key: "assist", axis: "A", label: t("Assist") },
    { key: "sustainability", axis: "S", label: t("Sustainability") },
    { key: "trade", axis: "T", label: t("Trade") },
    { key: "consistency", axis: "C", label: t("Consistency") },
  ] as const;
  const radar = categories.map((category) => ({
    axis: category.axis,
    fullName: category.label,
    score: score.breakdown[category.key].score,
  }));

  return (
    <section aria-labelledby="drastc-title" className="scroll-mt-28">
      <Heading id="drastc-title" level={2} className="mb-4">
        {t("DRASTC")}
      </Heading>

      <div className="overflow-hidden rounded-md border shadow-sm border-white/10 bg-zinc-900">
        <div className="grid lg:grid-cols-[minmax(0,1fr)_minmax(22rem,.85fr)]">
          <div className="p-5 sm:p-7">
            <div className="grid gap-6 sm:grid-cols-[10rem_minmax(0,1fr)] sm:items-center">
              <div>
                <Text className="!text-sm font-medium">{t("Overall score")}</Text>
                <div className="mt-1 flex items-end gap-1.5">
                  <span className="font-semibold text-5xl tracking-tight tabular-nums text-white">
                    {scoreFormatter.format(score.overall)}
                  </span>
                  <Text className="pb-1 !text-zinc-400">/ 10</Text>
                </div>
              </div>
              <div className="mx-auto w-full max-w-80">
                <DrastcRadarChart data={radar} />
              </div>
            </div>
          </div>

          <div className="border-t p-5 sm:p-7 lg:border-t-0 lg:border-l border-white/10 bg-white/[.025]">
            <ConfidenceScore score={score.confidence.score} />
            <Text className="mt-2 !text-sm/6">
              {t("Based on {battles} battle reports and {governors} governors.", {
                battles: numberFormatter.format(score.samples),
                governors: numberFormatter.format(score.confidence.unique_governors),
              })}
            </Text>

            <Subheading level={3} className="mt-7">
              {t("Breakdown")}
            </Subheading>
            <div className="mt-3 space-y-3">
              {categories.map((category) => {
                const value = score.breakdown[category.key].score;
                return (
                  <div
                    key={category.key}
                    className="grid grid-cols-[7.25rem_1fr_3rem] items-center gap-3 text-sm"
                  >
                    <Text className="!text-zinc-300">{category.label}</Text>
                    <div className="h-1.5 overflow-hidden rounded-full bg-zinc-700">
                      <div
                        className="h-full rounded-full bg-blue-600"
                        style={{ width: `${Math.min(100, value * 10)}%` }}
                      />
                    </div>
                    <Text className="text-right font-medium tabular-nums !text-white">
                      {scoreFormatter.format(value)}
                    </Text>
                  </div>
                );
              })}
            </div>
          </div>
        </div>

        <div className="border-t px-5 py-5 sm:px-7 border-white/10">
          <DrastcCredits />
        </div>
      </div>
    </section>
  );
}
