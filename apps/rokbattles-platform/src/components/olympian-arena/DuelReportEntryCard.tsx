"use client";

import { useMemo, useState } from "react";
import type { TooltipProps } from "recharts";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Tooltip as RechartsTooltip,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from "recharts";
import { Avatar } from "@/components/ui/Avatar";
import { Badge } from "@/components/ui/Badge";
import { Subheading } from "@/components/ui/Heading";
import { Strong, Text } from "@/components/ui/Text";
import { getArmamentInfo } from "@/hooks/useArmamentName";
import { useCommanderName } from "@/hooks/useCommanderName";
import type { DuelReportEntry } from "@/hooks/useOlympianArenaDuel";
import { formatUtcDateTime } from "@/lib/datetime";

export type DuelReportPayload = {
  metadata?: {
    email_id?: string;
    email_time?: number;
    email_receiver?: string;
    server_id?: number;
  };
  sender?: DuelParticipantInfo;
  opponent?: DuelParticipantInfo;
  results?: DuelResults;
};

type DuelParticipantInfo = {
  player_id?: number;
  player_name?: string;
  kingdom?: number;
  alliance?: string;
  duel_id?: number;
  avatar_url?: string;
  frame_url?: string;
  commanders?: {
    primary?: DuelCommanderInfo;
    secondary?: DuelCommanderInfo;
  };
  buffs?: DuelBuffEntry[];
};

type DuelCommanderInfo = {
  id?: number;
  level?: number;
  star?: number;
  awaked?: boolean;
  skills?: DuelSkillInfo[];
};

type DuelSkillInfo = {
  id?: number;
  level?: number;
  order?: number;
};

type DuelBuffEntry = {
  id?: number;
  value?: number;
};

type NormalizedBuff = {
  id: number;
  value: number;
};

type TroopBuffDisplay = {
  id: number;
  value: number;
  name: string;
  percent: boolean;
};

type DuelResults = {
  kill_points?: number;
  sev_wounded?: number;
  wounded?: number;
  dead?: number;
  heal?: number;
  units?: number;
  power?: number;
  win?: boolean;
  opponent_kill_points?: number;
  opponent_sev_wounded?: number;
  opponent_wounded?: number;
  opponent_dead?: number;
  opponent_heal?: number;
  opponent_units?: number;
  opponent_power?: number;
  opponent_win?: boolean;
};

type DuelEntryCardProps = {
  entry: DuelReportEntry;
};

export default function DuelReportEntryCard({ entry }: DuelEntryCardProps) {
  const payload = useMemo(() => (entry.report ?? {}) as DuelReportPayload, [entry.report]);
  const metadata = payload.metadata;
  const results = payload.results;
  const sender = payload.sender;
  const opponent = payload.opponent;

  const periodLabel = formatUtcDateTime(metadata?.email_time);
  const outcome = getOutcome(results);

  return (
    <section className="space-y-6">
      <header className="flex flex-wrap items-center gap-3">
        <Subheading level={3} className="text-lg">
          {periodLabel}
        </Subheading>
      </header>

      {results ? (
        <section className="space-y-4">
          <Subheading level={3} className="text-base">
            Duel summary
          </Subheading>
          <DuelResultsChart results={results} />
        </section>
      ) : null}

      <section className="grid gap-8 lg:grid-cols-2">
        <ParticipantCard participant={sender} isWinner={outcome?.winner === "sender"} />
        <ParticipantCard participant={opponent} isWinner={outcome?.winner === "opponent"} />
      </section>
    </section>
  );
}

type DuelMetricConfig = {
  label: string;
  senderKey: keyof DuelResults;
  opponentKey: keyof DuelResults;
};

const DUEL_METRICS: readonly DuelMetricConfig[] = [
  { label: "Units", senderKey: "units", opponentKey: "opponent_units" },
  { label: "Dead", senderKey: "dead", opponentKey: "opponent_dead" },
  {
    label: "Severely wounded",
    senderKey: "sev_wounded",
    opponentKey: "opponent_sev_wounded",
  },
  { label: "Wounded", senderKey: "wounded", opponentKey: "opponent_wounded" },
  { label: "Healed", senderKey: "heal", opponentKey: "opponent_heal" },
  { label: "Kill Points", senderKey: "kill_points", opponentKey: "opponent_kill_points" },
  { label: "Power", senderKey: "power", opponentKey: "opponent_power" },
] as const;

type DuelSummaryDatum = {
  key: string;
  label: string;
  sender: number;
  opponent: number;
};

function DuelResultsChart({ results }: { results: DuelResults }) {
  const numberFormatter = useMemo(
    () =>
      new Intl.NumberFormat("en-US", {
        maximumFractionDigits: 0,
      }),
    []
  );

  const chartData = useMemo(() => {
    const rows: DuelSummaryDatum[] = [];
    for (const metric of DUEL_METRICS) {
      const senderValue = getMetricValue(results, metric.senderKey);
      const opponentValue = getMetricValue(results, metric.opponentKey);
      if (senderValue == null && opponentValue == null) {
        continue;
      }

      rows.push({
        key: metric.label,
        label: metric.label,
        sender: senderValue ?? 0,
        opponent: opponentValue ?? 0,
      });
    }
    return rows;
  }, [results]);

  if (chartData.length === 0) {
    return null;
  }

  return (
    <div className="space-y-4">
      <div className="h-[320px] w-full">
        <ResponsiveContainer>
          <BarChart
            data={chartData}
            layout="vertical"
            margin={{ top: 12, right: 16, bottom: 12, left: 4 }}
          >
            <CartesianGrid strokeDasharray="3 3" horizontal={false} stroke="#d4d4d8" />
            <XAxis
              type="number"
              tickFormatter={(value) => numberFormatter.format(value)}
              tick={{ fontSize: 12, fill: "#6b7280" }}
              axisLine={{ stroke: "#d4d4d8" }}
              tickLine={false}
            />
            <YAxis
              type="category"
              dataKey="label"
              width={150}
              tick={{ fontSize: 12, fill: "#6b7280" }}
              axisLine={{ stroke: "#d4d4d8" }}
              tickLine={false}
            />
            <RechartsTooltip
              cursor={{ fill: "rgba(39, 39, 42, 0.08)" }}
              // @ts-expect-error
              content={<DuelSummaryTooltip formatter={numberFormatter} />}
            />
            <Bar dataKey="sender" stackId="duel" fill="#3b82f6" radius={[4, 0, 0, 4]} />
            <Bar dataKey="opponent" stackId="duel" fill="#f87171" radius={[0, 4, 4, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

function getMetricValue(results: DuelResults, key: keyof DuelResults) {
  const raw = results?.[key];
  if (typeof raw !== "number" || !Number.isFinite(raw)) {
    return null;
  }
  return raw;
}

function DuelSummaryTooltip({
  active,
  // @ts-expect-error
  payload,
  // @ts-expect-error
  label,
  formatter,
}: TooltipProps<number, string> & { formatter: Intl.NumberFormat }) {
  if (!active || !payload || payload.length === 0 || !label) {
    return null;
  }

  const entries = payload
    .filter((entry) => typeof entry.dataKey === "string")
    .map((entry) => ({
      key: entry.dataKey as string,
      value: Number(entry.value ?? 0),
    }));

  return (
    <div className="rounded-lg border border-zinc-950/10 bg-white px-4 py-3 text-xs shadow-lg dark:border-white/10 dark:bg-zinc-900">
      <div className="font-semibold text-zinc-700 dark:text-zinc-100">{label}</div>
      <div className="mt-3 space-y-1.5">
        {entries.map((entry) => {
          const descriptor =
            entry.key === "sender"
              ? { label: "Sender", color: "#3b82f6" }
              : entry.key === "opponent"
                ? { label: "Opponent", color: "#f87171" }
                : null;
          if (!descriptor) {
            return null;
          }
          return (
            <div key={descriptor.label} className="flex items-center gap-3">
              <span
                className="inline-block size-2 rounded-full"
                style={{ backgroundColor: descriptor.color }}
                aria-hidden="true"
              />
              <span className="flex-1 text-zinc-600 dark:text-zinc-300">{descriptor.label}</span>
              <span className="font-mono text-zinc-800 dark:text-white">
                {formatter.format(entry.value)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ParticipantCard({
  participant,
  isWinner,
}: {
  participant?: DuelParticipantInfo;
  isWinner?: boolean;
}) {
  const playerName = participant?.player_name?.trim() || "Unknown commander";
  const allianceTag = participant?.alliance?.trim();
  const playerId = participant?.player_id;

  const buffs = useMemo<NormalizedBuff[]>(
    () => normalizeBuffs(participant?.buffs ?? []),
    [participant?.buffs]
  );

  return (
    <div className="flex flex-col gap-5">
      <div className="flex items-start gap-3">
        <Avatar
          src={participant?.avatar_url ?? undefined}
          frameSrc={normalizeFrameUrl(participant?.frame_url)}
          alt={playerName}
          initials={generateInitials(playerName)}
          className="size-12"
        />
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <div className="text-base font-semibold text-zinc-900 dark:text-white">
              {playerName}
            </div>
            {isWinner ? <Badge color="emerald">Winner</Badge> : null}
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-zinc-500 dark:text-zinc-400">
            {typeof playerId === "number" && playerId > 0 ? <Badge>ID {playerId}</Badge> : null}
            {allianceTag ? <Badge>{allianceTag}</Badge> : null}
          </div>
        </div>
      </div>

      <div className="space-y-4">
        <CommanderSection
          primary={participant?.commanders?.primary}
          secondary={participant?.commanders?.secondary}
        />
        <TroopBuffsSection buffs={buffs} />
      </div>
    </div>
  );
}

function CommanderSection({
  primary,
  secondary,
}: {
  primary?: DuelCommanderInfo;
  secondary?: DuelCommanderInfo;
}) {
  const showPrimary = hasCommander(primary);
  const showSecondary = hasCommander(secondary);

  if (!showPrimary && !showSecondary) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>Commanders</Subheading>
      <div className="space-y-2">
        {showPrimary ? <CommanderRow commander={primary} label="Primary" /> : null}
        {showSecondary ? <CommanderRow commander={secondary} label="Secondary" /> : null}
      </div>
    </div>
  );
}

function hasCommander(commander?: DuelCommanderInfo) {
  const id = commander?.id;
  return typeof id === "number" && Number.isFinite(id) && id > 0;
}

function CommanderRow({
  commander,
  label,
}: {
  commander?: DuelCommanderInfo;
  label: "Primary" | "Secondary";
}) {
  const commanderId = commander?.id;
  const commanderName = useCommanderName(commanderId ?? null);
  const level = typeof commander?.level === "number" ? commander.level : null;
  const commanderLabel = commanderName ?? commanderId ?? "Unknown";
  const commanderIconSrc = `/lilith/images/commander/${commanderId}.png`;

  return (
    <Text className="flex flex-wrap items-center gap-2 text-sm">
      <span className="inline-flex items-center gap-1">
        <Avatar
          src={commanderIconSrc}
          alt={`${commanderLabel} icon`}
          className="size-12 outline-0!"
        />
        <Strong>{commanderLabel}</Strong>
      </span>
      <Badge>{label}</Badge>
      {level != null ? <Badge>Lvl {level}</Badge> : null}
    </Text>
  );
}

function TroopBuffsSection({ buffs }: { buffs: NormalizedBuff[] }) {
  const [expanded, setExpanded] = useState(false);
  const displayBuffs = useMemo<TroopBuffDisplay[]>(
    () =>
      buffs
        .map((buff) => {
          const info = getArmamentInfo(buff.id);
          if (!info?.name) {
            return null;
          }

          return {
            id: buff.id,
            value: buff.value,
            name: info.name,
            percent: info.percent,
          };
        })
        .filter((buff): buff is TroopBuffDisplay => buff != null),
    [buffs]
  );
  const visibleBuffs = expanded ? displayBuffs : displayBuffs.slice(0, 10);
  const hasMore = displayBuffs.length > 10;

  if (displayBuffs.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>Troop Buffs</Subheading>
      <div className="space-y-1.5">
        {visibleBuffs.map((buff) => (
          <TroopBuffRow key={buff.id} buff={buff} />
        ))}
      </div>
      {hasMore ? (
        <button
          type="button"
          onClick={() => setExpanded((prev) => !prev)}
          className="text-sm font-medium text-blue-600 transition hover:text-blue-500 dark:text-blue-400 dark:hover:text-blue-300"
        >
          {expanded ? "Show less" : "Show more"}
        </button>
      ) : null}
    </div>
  );
}

function TroopBuffRow({ buff }: { buff: TroopBuffDisplay }) {
  const valueLabel = formatBuffValue(buff.value, buff.percent);

  return (
    <Text className="flex items-center justify-between">
      <span>{buff.name}</span>
      <span className="font-mono text-zinc-900 dark:text-white">{valueLabel}</span>
    </Text>
  );
}

function formatBuffValue(value: number | undefined, isPercent: boolean) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return isPercent ? "0%" : "+0";
  }

  if (!isPercent) {
    return formatSignedNumber(value);
  }

  const percentValue = Math.abs(value) <= 1 ? value * 100 : value;
  return `${percentValue.toFixed(2)}%`;
}

function formatSignedNumber(value: number) {
  const normalized = Math.round(value * 100) / 100;
  const sign = normalized >= 0 ? "+" : "";
  const formatted = Number.isInteger(normalized)
    ? String(normalized)
    : normalized.toFixed(2).replace(/\.?0+$/, "");
  return `${sign}${formatted}`;
}

function normalizeBuffs(rawBuffs: DuelBuffEntry[]): NormalizedBuff[] {
  const aggregate = new Map<number, number>();

  for (const buff of rawBuffs) {
    const id = typeof buff.id === "number" && Number.isFinite(buff.id) ? buff.id : null;
    if (id == null || id <= 0) {
      continue;
    }

    const value = typeof buff.value === "number" && Number.isFinite(buff.value) ? buff.value : 0;
    aggregate.set(id, (aggregate.get(id) ?? 0) + value);
  }

  return Array.from(aggregate.entries())
    .map(([id, value]) => ({ id, value }))
    .sort((a, b) => a.id - b.id);
}

function getOutcome(results?: DuelResults) {
  if (!results) {
    return null;
  }

  if (results.win === true) {
    return { label: "Sender wins", winner: "sender" as const };
  }

  if (results.opponent_win === true) {
    return { label: "Opponent wins", winner: "opponent" as const };
  }

  return null;
}

function generateInitials(name: string) {
  const tokens = name.split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return undefined;
  if (tokens.length === 1) return tokens[0]?.slice(0, 2).toUpperCase();
  return (tokens[0]?.slice(0, 1) + tokens[tokens.length - 1]?.slice(0, 1)).toUpperCase();
}

function normalizeFrameUrl(frameUrl?: string | null) {
  if (typeof frameUrl !== "string") return undefined;
  const trimmed = frameUrl.trim();
  if (trimmed.length === 0 || trimmed.toLowerCase() === "null") return undefined;
  return trimmed.replace("http://", "https://");
}
