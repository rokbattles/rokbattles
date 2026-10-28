"use client";

import { cn } from "cn";

import type { ReactElement } from "react";
import { ArkAllianceEmblem } from "@/components/account-ark/ark-alliance-emblem";
import { useArkLeagueLabels, useArkNumber } from "@/components/account-ark/ark-shared";
import { Badge } from "@/components/ui/badge";
import { getArkOutcome } from "@/lib/ark/analytics";
import { formatArkAllianceLabel } from "@/lib/ark/format";
import type { ArkMatchDetail } from "@/lib/types/ark";

const dateFormatter = new Intl.DateTimeFormat("en-US", { dateStyle: "medium", timeZone: "UTC" });
const timeFormatter = new Intl.DateTimeFormat("en-US", {
  hour: "2-digit",
  minute: "2-digit",
  hourCycle: "h23",
  timeZone: "UTC",
});

type ArkScoreboardProps = { detail: ArkMatchDetail };

export function ArkScoreboard({ detail }: ArkScoreboardProps): ReactElement {
  const number = useArkNumber();
  const leagues = useArkLeagueLabels();

  const outcome = getArkOutcome(detail);
  const labels = {
    win: "Victory",
    loss: "Defeat",
    draw: "Draw",
    unknown: "Result unavailable",
  };
  const colors = { win: "green", loss: "red", draw: "amber", unknown: "zinc" } as const;

  const alliances = detail.alliances.toSorted((a, b) => Number(b.isBlue) - Number(a.isBlue));
  const total = alliances.reduce((sum, alliance) => sum + (alliance.score ?? 0), 0);

  return (
    <section className="overflow-hidden rounded-md border border-zinc-800">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b px-5 py-4 text-sm border-zinc-800">
        <div className="flex flex-wrap items-center gap-3">
          <span>{leagues[detail.league]}</span>
          <Badge color={colors[outcome]}>{labels[outcome]}</Badge>
        </div>
        <span className="text-zinc-400">
          {`${dateFormatter.format(detail.mailTimeMillis)} at ${timeFormatter.format(detail.mailTimeMillis)} UTC`}
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
              <div className="mb-1 flex items-center gap-2 text-xs font-medium text-zinc-400">
                {alliance.isBlue ? "Iset" : "Seth"}
                {alliance.id != null && alliance.id === detail.selfAllianceId ? (
                  <Badge color="zinc">You</Badge>
                ) : null}
              </div>
              <div
                className="truncate font-semibold"
                title={formatArkAllianceLabel(alliance, "Unknown alliance")}
              >
                {formatArkAllianceLabel(alliance, "Unknown alliance")}
              </div>
              <div
                className={cn(
                  "mt-2 text-3xl font-semibold tabular-nums",
                  alliance.isBlue ? "text-blue-400" : "text-red-400"
                )}
              >
                {number(alliance.score)}
              </div>
              <div className="mt-1 text-xs text-zinc-400">
                {`Members: ${number(alliance.members)} / ${number(alliance.membersMax)}`}
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
