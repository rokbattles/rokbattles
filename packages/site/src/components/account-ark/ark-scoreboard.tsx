"use client";

import { cn } from "cn";
import { useExtracted, useFormatter } from "next-intl";
import type { ReactElement } from "react";
import { ArkAllianceEmblem } from "@/components/account-ark/ark-alliance-emblem";
import { useArkLeagueLabels, useArkNumber } from "@/components/account-ark/ark-shared";
import { Badge } from "@/components/ui/badge";
import { getArkOutcome } from "@/lib/ark/analytics";
import { formatArkAllianceLabel } from "@/lib/ark/format";
import type { ArkMatchDetail } from "@/lib/types/ark";

type ArkScoreboardProps = { detail: ArkMatchDetail };

export function ArkScoreboard({ detail }: ArkScoreboardProps): ReactElement {
  const t = useExtracted();
  const intl = useFormatter();
  const number = useArkNumber();
  const leagues = useArkLeagueLabels();

  const outcome = getArkOutcome(detail);
  const labels = {
    win: t("Victory"),
    loss: t("Defeat"),
    draw: t("Draw"),
    unknown: t("Result unavailable"),
  };
  const colors = { win: "green", loss: "red", draw: "amber", unknown: "zinc" } as const;

  const alliances = detail.alliances.toSorted((a, b) => Number(b.isBlue) - Number(a.isBlue));
  const total = alliances.reduce((sum, alliance) => sum + (alliance.score ?? 0), 0);

  return (
    <section className="overflow-hidden rounded-md border border-zinc-200 dark:border-zinc-800">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-zinc-200 px-5 py-4 text-sm dark:border-zinc-800">
        <div className="flex flex-wrap items-center gap-3">
          <span>{leagues[detail.league]}</span>
          <Badge color={colors[outcome]}>{labels[outcome]}</Badge>
        </div>
        <span className="text-zinc-500 dark:text-zinc-400">
          {t("{date} at {time} UTC", {
            date: intl.dateTime(detail.mailTimeMillis, { dateStyle: "medium", timeZone: "UTC" }),
            time: intl.dateTime(detail.mailTimeMillis, {
              hour: "2-digit",
              minute: "2-digit",
              hourCycle: "h23",
              timeZone: "UTC",
            }),
          })}
        </span>
      </div>
      <div className="grid gap-6 p-5 sm:grid-cols-2 sm:p-6">
        {alliances.map((alliance) => (
          <div
            key={`${alliance.id}:${alliance.isBlue}:${alliance.name}`}
            className="flex items-center gap-4"
          >
            <ArkAllianceEmblem logo={alliance.logo} isBlue={alliance.isBlue} large />
            <div className="min-w-0 flex-1">
              <div className="mb-1 flex items-center gap-2 text-xs font-medium text-zinc-500 dark:text-zinc-400">
                {alliance.isBlue ? t("Iset") : t("Seth")}
                {alliance.id != null && alliance.id === detail.selfAllianceId ? (
                  <Badge color="zinc">{t("You")}</Badge>
                ) : null}
              </div>
              <div
                className="truncate font-semibold"
                title={formatArkAllianceLabel(alliance, t("Unknown alliance"))}
              >
                {formatArkAllianceLabel(alliance, t("Unknown alliance"))}
              </div>
              <div
                className={cn(
                  "mt-2 text-3xl font-semibold tabular-nums",
                  alliance.isBlue
                    ? "text-blue-600 dark:text-blue-400"
                    : "text-red-600 dark:text-red-400"
                )}
              >
                {number(alliance.score)}
              </div>
              <div className="mt-1 text-xs text-zinc-500 dark:text-zinc-400">
                {t("Members: {members} / {max}", {
                  members: number(alliance.members),
                  max: number(alliance.membersMax),
                })}
              </div>
            </div>
          </div>
        ))}
      </div>
      {total > 0 ? (
        <div aria-hidden="true" className="flex h-1.5">
          {alliances.map((alliance) => (
            <div
              key={`${alliance.id}:${alliance.isBlue}:${alliance.name}`}
              className={alliance.isBlue ? "bg-blue-500" : "bg-red-500"}
              style={{ width: `${((alliance.score ?? 0) / total) * 100}%` }}
            />
          ))}
        </div>
      ) : null}
    </section>
  );
}
