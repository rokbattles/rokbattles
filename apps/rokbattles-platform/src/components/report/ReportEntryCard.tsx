"use client";

import Image from "next/image";
import { useMemo } from "react";
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
import { useArmamentName } from "@/hooks/useArmamentName";
import { useCommanderName } from "@/hooks/useCommanderName";
import { useFormationName } from "@/hooks/useFormationName";
import { useInscriptionName } from "@/hooks/useInscriptionName";
import { cn } from "@/lib/cn";
import {
  type ArmamentBuff,
  type EquipmentToken,
  getInscriptionRarity,
  parseArmamentBuffs,
  parseEquipment,
  parseSemicolonNumberList,
} from "@/lib/report/parsers";
import type {
  RawBattleResults,
  RawCommanderInfo,
  RawParticipantInfo,
  RawReportPayload,
} from "@/lib/types/rawReport";
import type { ReportEntry } from "@/lib/types/report";

type ReportEntryCardProps = {
  entry: ReportEntry;
};

type ParticipantSide = "self" | "enemy";
const ARTIFACT_IDS = new Set([20401, 20402]);

export function ReportEntryCard({ entry }: ReportEntryCardProps) {
  const payload = useMemo(() => (entry.report ?? {}) as RawReportPayload, [entry.report]);

  const metadata = payload?.metadata;
  const battleResults = payload?.battle_results;
  const selfParticipant = payload?.self;
  const enemyParticipant = payload?.enemy;

  const start = metadata?.start_date ?? entry.startDate;
  const end = metadata?.end_date;

  const periodLabel = formatPeriod(start, end);

  return (
    <section className="space-y-6">
      <header className="flex flex-wrap items-center gap-3">
        <Subheading level={3} className="text-lg">
          {periodLabel}
        </Subheading>
      </header>

      {battleResults ? (
        <section className="space-y-4">
          <Subheading level={3} className="text-base">
            Battle summary
          </Subheading>
          <BattleResultsChart results={battleResults} />
        </section>
      ) : null}
      <section className="grid gap-8 lg:grid-cols-2">
        <ParticipantCard participant={selfParticipant} side="self" />
        <ParticipantCard participant={enemyParticipant} side="enemy" />
      </section>
    </section>
  );
}

type BattleMetricConfig = {
  label: string;
  selfKey: keyof RawBattleResults;
  enemyKey: keyof RawBattleResults;
};

const BATTLE_METRICS: readonly BattleMetricConfig[] = [
  { label: "Units", selfKey: "max", enemyKey: "enemy_max" },
  { label: "Remaining", selfKey: "remaining", enemyKey: "enemy_remaining" },
  { label: "Heal", selfKey: "healing", enemyKey: "enemy_healing" },
  { label: "Dead", selfKey: "death", enemyKey: "enemy_death" },
  {
    label: "Severely wounded",
    selfKey: "severely_wounded",
    enemyKey: "enemy_severely_wounded",
  },
  { label: "Slightly wounded", selfKey: "wounded", enemyKey: "enemy_wounded" },
  { label: "Watchtower Damage", selfKey: "watchtower", enemyKey: "enemy_watchtower" },
  { label: "Kill Points", selfKey: "kill_score", enemyKey: "enemy_kill_score" },
] as const;

type BattleSummaryDatum = {
  key: string;
  label: string;
  self: number;
  enemy: number;
};

function BattleResultsChart({ results }: { results: RawBattleResults }) {
  const numberFormatter = useMemo(
    () =>
      new Intl.NumberFormat("en-US", {
        maximumFractionDigits: 0,
      }),
    []
  );

  const chartData = useMemo(() => {
    const rows: BattleSummaryDatum[] = [];
    for (const metric of BATTLE_METRICS) {
      const selfValue = getMetricValue(results, metric.selfKey);
      const enemyValue = getMetricValue(results, metric.enemyKey);
      if (selfValue == null && enemyValue == null) {
        continue;
      }

      rows.push({
        key: metric.label,
        label: metric.label,
        self: selfValue ?? 0,
        enemy: enemyValue ?? 0,
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
              width={140}
              tick={{ fontSize: 12, fill: "#6b7280" }}
              axisLine={{ stroke: "#d4d4d8" }}
              tickLine={false}
            />
            <RechartsTooltip
              cursor={{ fill: "rgba(39, 39, 42, 0.08)" }}
              // @ts-expect-error
              content={<BattleSummaryTooltip formatter={numberFormatter} />}
            />
            <Bar
              dataKey="self"
              stackId="battle"
              fill="#3b82f6"
              radius={[4, 0, 0, 4]}
              maxBarSize={28}
            />
            <Bar
              dataKey="enemy"
              stackId="battle"
              fill="#f87171"
              radius={[0, 4, 4, 0]}
              maxBarSize={28}
            />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}

function getMetricValue(results: RawBattleResults, key: keyof RawBattleResults) {
  const raw = results?.[key];
  if (typeof raw !== "number" || !Number.isFinite(raw)) {
    return null;
  }
  return raw;
}

function BattleSummaryTooltip({
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
            entry.key === "self"
              ? { label: "Self", color: "#3b82f6" }
              : entry.key === "enemy"
                ? { label: "Enemy", color: "#f87171" }
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
  side,
}: {
  participant?: RawParticipantInfo;
  side: ParticipantSide;
}) {
  const playerName = participant?.player_name?.trim() || "Unknown commander";
  const allianceTag = participant?.alliance_tag?.trim();
  const playerId = participant?.player_id;

  const equipmentTokens = useMemo(
    () => parseEquipment(participant?.equipment ?? null),
    [participant?.equipment]
  );

  const artifactTokens = useMemo(() => {
    const tokens = parseEquipment(participant?.equipment_2 ?? null);
    return tokens.filter((token) => ARTIFACT_IDS.has(token.id));
  }, [participant?.equipment_2]);

  const inscriptionIds = useMemo(
    () => parseSemicolonNumberList(participant?.inscriptions ?? null),
    [participant?.inscriptions]
  );

  const armamentBuffs = useMemo(
    () => parseArmamentBuffs(participant?.armament_buffs ?? null),
    [participant?.armament_buffs]
  );

  const sideBadgeColor = side === "self" ? "blue" : "rose";
  const sideBadgeLabel = side === "self" ? "Self" : "Enemy";

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
            <Badge color={sideBadgeColor}>{sideBadgeLabel}</Badge>
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs text-zinc-500 dark:text-zinc-400">
            {typeof playerId === "number" && playerId > 0 ? <Badge>ID {playerId}</Badge> : null}
            {allianceTag ? <Badge>{allianceTag}</Badge> : null}
            {participant?.is_rally ? <Badge>Rally</Badge> : null}
          </div>
        </div>
      </div>

      <div className="space-y-4">
        <CommanderSection
          primary={participant?.primary_commander}
          secondary={participant?.secondary_commander}
          primaryFormation={participant?.formation}
        />
        <EquipmentSection tokens={equipmentTokens} />
        <ArtifactSection tokens={artifactTokens} />
        <ArmamentSection buffs={armamentBuffs} inscriptions={inscriptionIds} />
      </div>
    </div>
  );
}

function CommanderSection({
  primary,
  secondary,
  primaryFormation,
}: {
  primary?: RawCommanderInfo;
  secondary?: RawCommanderInfo;
  primaryFormation?: number | null;
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
        {showPrimary ? <CommanderRow commander={primary} formation={primaryFormation} /> : null}
        {showSecondary ? <CommanderRow commander={secondary} /> : null}
      </div>
    </div>
  );
}

function hasCommander(commander?: RawCommanderInfo) {
  const id = commander?.id;
  return typeof id === "number" && Number.isFinite(id) && id > 0;
}

function CommanderRow({
  commander,
  formation,
}: {
  commander?: RawCommanderInfo;
  formation?: number | null;
}) {
  const commanderId = commander?.id;
  const commanderName = useCommanderName(commanderId ?? null);
  const formationName = useFormationName(formation ?? null);
  const level = typeof commander?.level === "number" ? commander.level : null;
  const skillSummary = commander?.skills?.trim();
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
      {typeof formation === "number" ? (
        <Badge>{formationName ?? `Formation ${formation}`}</Badge>
      ) : null}
      {level != null ? <Badge>Lvl {level}</Badge> : null}
      {skillSummary ? <Badge>{skillSummary}</Badge> : null}
    </Text>
  );
}

function EquipmentSection({ tokens }: { tokens: EquipmentToken[] }) {
  if (tokens.length === 0) {
    return null;
  }

  const slots = tokens.reduce<Record<number, EquipmentToken | undefined>>((acc, token) => {
    acc[token.slot] = token;
    return acc;
  }, {});

  return (
    <div className="space-y-2">
      <Subheading>Equipment</Subheading>
      <div className="flex justify-center">
        <div className="grid grid-cols-[auto_auto_auto] gap-2 justify-items-center">
          <div />
          <EquipmentGlyph token={slots[2]} />
          <div />
          <EquipmentGlyph token={slots[1]} />
          <EquipmentGlyph token={slots[3]} />
          <EquipmentGlyph token={slots[4]} />
          <EquipmentGlyph token={slots[7]} />
          <EquipmentGlyph token={slots[5]} />
          <EquipmentGlyph token={slots[8]} />
          <div />
          <EquipmentGlyph token={slots[6]} />
          <div />
        </div>
      </div>
    </div>
  );
}

function ArtifactSection({ tokens }: { tokens: EquipmentToken[] }) {
  if (tokens.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>Artifacts</Subheading>
      <div className="grid grid-cols-4 gap-3 sm:grid-cols-5">
        {tokens.map((token) => (
          <EquipmentGlyph key={`${token.slot}-${token.id}`} token={token} />
        ))}
      </div>
    </div>
  );
}

function EquipmentGlyph({ token }: { token?: EquipmentToken }) {
  const { tier, isSpecialTalent } = getTierInfo(token?.attr);
  const tierLabel = tier != null ? toRomanNumeral(tier) : null;

  return (
    <div className="relative h-14 w-14 select-none overflow-hidden rounded-lg bg-zinc-900/60 ring-1 ring-white/10 sm:h-16 sm:w-16">
      {token?.id ? (
        <Image
          src={`/lilith/images/equipment/${token.id}.png`}
          alt={`Equipment ${token.id}`}
          fill
          sizes="(min-width: 640px) 64px, 56px"
          className="object-contain"
        />
      ) : (
        <div className="flex h-full w-full items-center justify-center text-[10px] font-semibold text-zinc-300">
          —
        </div>
      )}
      {tierLabel ? (
        <span className="absolute bottom-1 left-1 rounded bg-black/70 px-1 text-[0.625rem] font-semibold text-white">
          {tierLabel}
        </span>
      ) : null}
      {isSpecialTalent ? (
        <span className="absolute bottom-1 right-1 rounded bg-amber-500 px-1 text-[0.625rem] font-semibold text-white">
          ST
        </span>
      ) : null}
    </div>
  );
}

function InscriptionBadge({ id }: { id: number }) {
  const name = useInscriptionName(id ?? null);
  const rarity = getInscriptionRarity(id);
  const color = rarity === "special" ? "amber" : rarity === "rare" ? "blue" : "gray";

  return (
    <div className="relative flex h-5 w-28 select-none items-center justify-center text-xs font-semibold">
      <div
        className={cn(
          "absolute inset-0 [clip-path:polygon(90%_0%,_100%_50%,_90%_100%,_10%_100%,_0%_50%,_10%_0%)]",
          color === "amber" &&
            "bg-[rgb(217,98,0)] bg-gradient-to-b from-[rgb(255,255,122)] to-[rgb(241,81,0)]",
          color === "blue" &&
            "bg-[rgb(57,99,255)] bg-gradient-to-b from-[rgb(192,229,253)] to-[rgb(57,99,255)]",
          color === "gray" &&
            "bg-[rgb(68,68,68)] bg-gradient-to-b from-[rgb(231,231,231)] to-[rgb(77,77,77)]"
        )}
      />
      <div
        className={cn(
          "absolute inset-px [clip-path:polygon(90%_0%,_100%_50%,_90%_100%,_10%_100%,_0%_50%,_10%_0%)]",
          color === "amber" && "bg-gradient-to-b from-[rgb(255,255,123)] to-[rgb(255,217,44)]",
          color === "blue" && "bg-gradient-to-b from-[rgb(207,237,255)] to-[rgb(160,192,255)]",
          color === "gray" && "bg-gradient-to-b from-[rgb(229,230,230)] to-[rgb(231,231,231)]"
        )}
      />
      <span
        className={cn(
          "relative z-10 truncate px-2 text-center leading-none",
          color === "amber" && "text-[rgb(217,98,0)]",
          color === "blue" && "text-[rgb(57,99,255)]",
          color === "gray" && "text-[rgb(68,68,68)]"
        )}
      >
        {name ?? id}
      </span>
    </div>
  );
}

function ArmamentSection({
  buffs,
  inscriptions,
}: {
  buffs: ArmamentBuff[];
  inscriptions: number[];
}) {
  if (buffs.length === 0 && inscriptions.length === 0) {
    return null;
  }

  return (
    <div className="space-y-2">
      <Subheading>Armament Info</Subheading>
      {inscriptions.length > 0 ? (
        <div className="flex flex-wrap gap-1.5">
          {inscriptions.map((id) => (
            <InscriptionBadge key={id} id={id} />
          ))}
        </div>
      ) : null}
      {buffs.length > 0 ? (
        <div className="space-y-1.5">
          {buffs.map((buff) => (
            <ArmamentRow key={buff.id} buff={buff} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function ArmamentRow({ buff }: { buff: ArmamentBuff }) {
  const name = useArmamentName(buff.id ?? null);
  const percentage = (Number(buff.value ?? 0) * 100).toFixed(2);

  return (
    <Text className="flex items-center justify-between">
      <span>{name ?? `Armament ${buff.id}`}</span>
      <span className="font-mono text-zinc-900 dark:text-white">{percentage}%</span>
    </Text>
  );
}

function getTierInfo(attr?: number) {
  if (typeof attr !== "number" || !Number.isFinite(attr)) {
    return { tier: undefined, isSpecialTalent: false };
  }

  const numeric = Number(attr);
  const isSpecialTalent = numeric >= 10;
  const base = isSpecialTalent ? numeric % 10 : numeric;
  const tier = Math.max(0, Math.min(5, Math.trunc(base)));

  return { tier, isSpecialTalent };
}

function toRomanNumeral(value: number | undefined) {
  if (typeof value !== "number" || value <= 0) {
    return null;
  }

  const numerals = ["", "I", "II", "III", "IV", "V"];
  return numerals[value] ?? null;
}

function formatPeriod(start?: number | null, end?: number | null): string {
  const startMs = toMillis(start);
  if (startMs == null) {
    return "Unknown period";
  }

  const startDate = new Date(startMs);
  const startLabel = formatUtc(startDate);

  const endMs = toMillis(end);
  if (endMs == null) {
    return startLabel;
  }

  const endDate = new Date(endMs);
  const sameDay =
    startDate.getUTCFullYear() === endDate.getUTCFullYear() &&
    startDate.getUTCMonth() === endDate.getUTCMonth() &&
    startDate.getUTCDate() === endDate.getUTCDate();

  const endLabel = sameDay
    ? formatUtc(endDate, { includeDate: false, includePrefix: false })
    : formatUtc(endDate, { includePrefix: false });

  return `${startLabel} - ${endLabel}`;
}

function formatUtc(
  date: Date,
  options: { includeDate?: boolean; includePrefix?: boolean } = { includeDate: true }
) {
  const includeDate = options.includeDate ?? true;
  const includePrefix = options.includePrefix ?? true;
  const month = String(date.getUTCMonth() + 1).padStart(2, "0");
  const day = String(date.getUTCDate()).padStart(2, "0");
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");

  const prefix = includePrefix ? "UTC " : "";
  return includeDate
    ? `${prefix}${month}/${day} ${hours}:${minutes}`
    : `${prefix}${hours}:${minutes}`;
}

function toMillis(value?: number | null): number | null {
  if (value == null || !Number.isFinite(value)) {
    return null;
  }

  const numeric = Number(value);
  const absValue = Math.abs(numeric);
  const millis = absValue < 1_000_000_000_000 ? numeric * 1000 : numeric;
  return millis > 0 ? millis : null;
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
