"use client";

import { useSearchParams } from "next/navigation";
import { useExtracted } from "next-intl";
import { useCallback, useContext, useMemo } from "react";
import {
  AccountLootLayout,
  type AccountLootSection,
} from "@/components/account-loot/account-loot-layout";
import { LootBreakdownTable } from "@/components/account-loot/loot-breakdown-table";
import { LootErrorState } from "@/components/account-loot/loot-error-state";
import { PersonalLootFilters } from "@/components/account-loot/personal-loot-filters";
import { LootExplorerSummary } from "@/components/loot-explorer/loot-explorer-summary";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import {
  defaultLootDateRange,
  type PersonalLootEndpoint,
  usePersonalLoot,
} from "@/hooks/use-personal-loot";
import { formatLocalDateInput } from "@/lib/datetime";
import { buildLootRewardRows } from "@/lib/loot/reward-rows";
import type { LootExplorerOption } from "@/lib/loot-explorer/catalog";
import type { PersonalLootGroup } from "@/lib/types/loot";
import { GovernorContext } from "@/providers/governor-context";

type PersonalLootContentProps = {
  active: AccountLootSection;
  endpoint: PersonalLootEndpoint;
  datasetLocale?: string;
};

type SectionConfig = {
  defaultType: string;
  typeOptions: LootExplorerOption[];
  levelOptionsByType?: Record<string, LootExplorerOption[]>;
  allowMultipleLevels?: boolean;
  showLevelFilter?: boolean;
};

function parseLevels(searchParams: URLSearchParams): number[] {
  const values = searchParams.getAll("level");
  const parsed = new Set<number>();
  for (const value of values) {
    for (const part of value.split(",")) {
      const level = Number.parseInt(part.trim(), 10);
      if (Number.isFinite(level)) {
        parsed.add(level);
      }
    }
  }

  return Array.from(parsed).sort((left, right) => left - right);
}

function levelOptions(levels: number[], label: (level: number) => string): LootExplorerOption[] {
  return levels.map((level) => ({ value: level.toString(), label: label(level) }));
}

function emptyGroup(): PersonalLootGroup {
  return {
    level: null,
    reports: 0,
    lootTotal: 0,
    apUsed: 0,
    honorGained: 0,
    xpGained: 0,
    rewards: [],
  };
}

export function PersonalLootContent({ active, endpoint, datasetLocale }: PersonalLootContentProps) {
  const t = useExtracted();
  const searchParams = useSearchParams();
  const governorContext = useContext(GovernorContext);
  if (!governorContext) {
    throw new Error("My Loot page must be used within a GovernorProvider");
  }

  const formatLevel = useCallback(
    (level: number) => t("Level {level}", { level: level.toString() }),
    [t]
  );
  const config: SectionConfig = useMemo(() => {
    if (active === "barbarian-forts") {
      return {
        defaultType: "barbarian-forts",
        typeOptions: [
          { value: "barbarian-forts", label: t("Barbarian Forts") },
          { value: "marauder-encampments", label: t("Marauder Encampments") },
        ],
        levelOptionsByType: {
          "barbarian-forts": levelOptions(
            Array.from({ length: 15 }, (_, index) => index + 1),
            formatLevel
          ),
          "marauder-encampments": levelOptions([1, 11], formatLevel),
        },
        allowMultipleLevels: false,
      };
    }

    if (active === "baulurs") {
      return {
        defaultType: "ironhand-baulur",
        typeOptions: [
          { value: "ironhand-baulur", label: t("Ironhand Baulur") },
          { value: "miser-khaolak", label: t("Miser Khaolak") },
        ],
        showLevelFilter: false,
      };
    }

    return {
      defaultType: "barbarians",
      typeOptions: [
        { value: "barbarians", label: t("Barbarians") },
        { value: "marauders", label: t("Marauders") },
      ],
      levelOptionsByType: {
        barbarians: levelOptions(
          Array.from({ length: 55 }, (_, index) => index + 1),
          formatLevel
        ),
        marauders: levelOptions([1, 41], formatLevel),
      },
      allowMultipleLevels: true,
    };
  }, [active, formatLevel, t]);

  const selectedType = config.typeOptions.some(
    (option) => option.value === searchParams.get("type")
  )
    ? (searchParams.get("type") as string)
    : config.defaultType;
  const rawSelectedLevels = useMemo(() => parseLevels(searchParams), [searchParams]);
  const selectedLevels = useMemo(() => {
    const options = config.levelOptionsByType?.[selectedType] ?? [];
    const validLevels = new Set(options.map((option) => Number(option.value)));
    const filtered = rawSelectedLevels.filter((level) => validLevels.has(level));
    return config.allowMultipleLevels === false ? filtered.slice(0, 1) : filtered;
  }, [config.allowMultipleLevels, config.levelOptionsByType, rawSelectedLevels, selectedType]);
  const defaults = defaultLootDateRange();
  const startDate = searchParams.get("start") || defaults.start;
  const endDate = searchParams.get("end") || defaults.end;
  const governorId = governorContext.activeGovernor?.governorId;
  const { data, error } = usePersonalLoot({
    governorId,
    endpoint,
    type: selectedType,
    levels: selectedLevels,
    startParam: startDate,
    endParam: endDate,
  });

  const minDate = "2025-01-01";
  const maxDate = formatLocalDateInput(new Date());

  if (error) {
    return (
      <AccountLootLayout active={active}>
        <LootErrorState />
      </AccountLootLayout>
    );
  }

  const summaryItems = (() => {
    if (active === "baulurs") {
      return [{ label: t("Results"), value: data?.totals.results ?? 0 }];
    }

    const items = [
      { label: t("Results"), value: data?.totals.results ?? 0 },
      { label: t("AP used"), value: data?.totals.apUsed ?? 0 },
      { label: t("Honor gained"), value: data?.totals.honorGained ?? 0 },
    ];

    return active === "barbarians"
      ? [...items, { label: t("XP gained"), value: data?.totals.xpGained ?? 0 }]
      : items;
  })();

  return (
    <AccountLootLayout active={active}>
      <PersonalLootFilters
        typeOptions={config.typeOptions}
        selectedType={selectedType}
        levelOptionsByType={config.levelOptionsByType}
        selectedLevels={selectedLevels}
        allowMultipleLevels={config.allowMultipleLevels}
        showLevelFilter={config.showLevelFilter}
        startDate={startDate}
        endDate={endDate}
        minDate={minDate}
        maxDate={maxDate}
      />
      {data ? <LootExplorerSummary items={summaryItems} /> : <Text>{t("Loading loot...")}</Text>}
      {data ? (
        <div className="space-y-8">
          {(data.groups.length ? data.groups : [emptyGroup()]).map((group) => {
            const rows = buildLootRewardRows(
              group,
              (type, subType) =>
                t("Unknown reward {type}/{subType}", {
                  type: type.toString(),
                  subType: subType.toString(),
                }),
              datasetLocale
            );
            return (
              <section key={group.level ?? "all"} className="space-y-4">
                <Subheading>
                  {group.level == null
                    ? t("Loot breakdown")
                    : t("Level {level}", { level: group.level.toString() })}
                </Subheading>
                {rows.length === 0 ? (
                  <Text>{t("No loot in this date range.")}</Text>
                ) : (
                  <LootBreakdownTable rows={rows} />
                )}
              </section>
            );
          })}
        </div>
      ) : null}
    </AccountLootLayout>
  );
}
