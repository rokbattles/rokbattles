"use client";

import { useExtracted } from "next-intl";
import type { ReactElement } from "react";
import {
  ArkMetricCards,
  ArkSectionHeading,
  useArkNumber,
} from "@/components/account-ark/ark-shared";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import type { ArkMatchDetail } from "@/lib/types/ark";

type ArkMatchDetailIndividualResultsSectionProps = { detail: ArkMatchDetail };

export function ArkMatchDetailIndividualResultsSection({
  detail,
}: ArkMatchDetailIndividualResultsSectionProps): ReactElement {
  const t = useExtracted();
  const number = useArkNumber();

  const { overview, individualResults: result } = detail;
  const scores = [
    { label: t("Kills"), value: result.killScore, color: "bg-red-500" },
    { label: t("Occupation"), value: result.occupationScore, color: "bg-blue-500" },
    { label: t("Ark of Osiris"), value: result.arkOfOsirisScore, color: "bg-amber-500" },
    { label: t("Provisions"), value: result.provisionsScore, color: "bg-emerald-500" },
    { label: t("Healing"), value: result.healingScore, color: "bg-violet-500" },
  ];
  const maxScore = Math.max(1, ...scores.map((score) => score.value ?? 0));

  const highlightLabels: Record<string, string> = {
    flag_score: t("Ark score"),
    building_score: t("Occupation score"),
    gather_score: t("Provisions score"),
    killed_score: t("Kills"),
    be_killed_score: t("Severely wounded"),
    healing_score: t("Units healed"),
  };

  return (
    <div className="space-y-8">
      <section className="space-y-4">
        <ArkSectionHeading
          title={t("Your contribution")}
          description={t("Your individual result, from combat to objectives.")}
        />
        {!detail.hasIndividualResults ? (
          <Text>
            {t(
              "Upload the individual result mail to see your score breakdown and pairing performance."
            )}
          </Text>
        ) : null}
        <ArkMetricCards
          items={[
            {
              label: t("Individual score"),
              value: number(overview.score),
              detail:
                overview.rank && overview.rank > 0
                  ? t("Rank #{rank}", { rank: number(overview.rank) })
                  : undefined,
            },
            {
              label: t("Battle win rate"),
              value: result.winRate == null ? "—" : `${number(result.winRate)}%`,
              detail: t("{wins} won · {losses} lost", {
                wins: number(result.battlesWin),
                losses: number(result.battlesLose),
              }),
            },
            { label: t("Kills"), value: number(result.kills) },
            { label: t("Severely wounded"), value: number(result.severelyWounded) },
          ]}
        />
      </section>
      <div className="grid gap-4 lg:grid-cols-2">
        <section className="rounded-md border p-5 border-zinc-800">
          <Subheading>{t("Individual score breakdown")}</Subheading>
          <dl className="mt-6 space-y-4">
            {scores.map((score) => (
              <div key={score.label}>
                <div className="mb-2 flex items-center justify-between gap-4 text-sm">
                  <dt>{score.label}</dt>
                  <dd className="tabular-nums">{number(score.value)}</dd>
                </div>
                <div aria-hidden="true" className="h-2 overflow-hidden rounded-full bg-zinc-800">
                  <div
                    className={`h-full rounded-full ${score.color}`}
                    style={{ width: `${((score.value ?? 0) / maxScore) * 100}%` }}
                  />
                </div>
              </div>
            ))}
          </dl>
        </section>
        <section className="rounded-md border p-5 border-zinc-800">
          <Subheading>{t("Match activity")}</Subheading>
          <dl className="mt-6 divide-y text-sm divide-zinc-800">
            {[
              { label: t("Units healed"), value: result.unitsHealed },
              { label: t("Speedups used (mins)"), value: result.speedupsMinutes ?? 0 },
              { label: t("Teleports"), value: result.teleports },
            ].map((metric) => (
              <div key={metric.label} className="flex justify-between gap-4 py-3">
                <dt className="text-zinc-400">{metric.label}</dt>
                <dd className="font-medium tabular-nums">{number(metric.value)}</dd>
              </div>
            ))}
          </dl>
        </section>
      </div>
      {detail.highlights.length ? (
        <section className="space-y-4">
          <ArkSectionHeading
            title={t("Alliance highlights")}
            description={t("Your alliance totals and the MVP in each category.")}
          />
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {detail.highlights.map((highlight) => (
              <div key={highlight.category} className="rounded-md border p-4 border-zinc-800">
                <Text>{highlightLabels[highlight.category] ?? highlight.category}</Text>
                <div className="mt-2 text-2xl font-semibold tabular-nums">
                  {number(highlight.allianceValue)}
                </div>
                {highlight.playerName ? (
                  <div className="mt-4 border-t pt-3 border-zinc-800">
                    <div className="truncate text-sm font-medium">{highlight.playerName}</div>
                    <div className="mt-1 text-xs text-zinc-400">
                      {t("MVP · {value}", { value: number(highlight.playerValue) })}
                    </div>
                  </div>
                ) : (
                  <Text className="mt-4">{t("No MVP recorded")}</Text>
                )}
              </div>
            ))}
          </div>
        </section>
      ) : null}
    </div>
  );
}
