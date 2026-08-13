"use client";

import dynamic from "next/dynamic";
import { useExtracted, useLocale } from "next-intl";
import { Heading, Subheading } from "@/components/ui/heading";
import { getEquipmentSlotName } from "@/hooks/use-equipment-name";
import type {
  CombatLabPreviewFormationUsagePoint,
  CombatLabPreviewRangeKey,
  CombatLabPreviewLoadouts as LoadoutData,
} from "@/lib/combat-lab/preview-types";

const CombatLabSkills = dynamic(
  () =>
    import("@/components/combat-lab/combat-lab-skills").then((module) => module.CombatLabSkills),
  {
    loading: () => <SkillsSkeleton />,
    ssr: false,
  }
);

const CombatLabFormationChart = dynamic(
  () =>
    import("@/components/combat-lab/combat-lab-charts").then(
      (module) => module.CombatLabFormationChart
    ),
  {
    loading: () => <FormationChartSkeleton />,
    ssr: false,
  }
);

const CombatLabArmamentChart = dynamic(
  () =>
    import("@/components/combat-lab/combat-lab-charts").then(
      (module) => module.CombatLabArmamentChart
    ),
  {
    loading: () => (
      <div className="h-[34rem] animate-pulse rounded-md bg-zinc-950/[.035] dark:bg-white/5" />
    ),
    ssr: false,
  }
);

const CombatLabEquipmentChart = dynamic(
  () =>
    import("@/components/combat-lab/combat-lab-charts").then(
      (module) => module.CombatLabEquipmentChart
    ),
  {
    loading: () => (
      <div className="h-[31rem] animate-pulse rounded-md bg-zinc-950/[.035] dark:bg-white/5" />
    ),
    ssr: false,
  }
);

const armamentSlotOrder = [1, 3, 2, 4] as const;
const equipmentSlotOrder = [1, 2, 3, 4, 5, 6, 7] as const;

function FormationChartSkeleton() {
  return (
    <article className="min-w-0 animate-pulse overflow-hidden rounded-md border border-zinc-950/10 bg-white xl:col-span-2 dark:border-white/10 dark:bg-zinc-900">
      <div className="min-h-[5.25rem] border-zinc-950/10 border-b px-5 py-4 dark:border-white/10">
        <div className="h-5 w-32 rounded bg-zinc-200 dark:bg-zinc-800" />
        <div className="mt-4 flex gap-4">
          {["one", "two", "three", "four"].map((item) => (
            <div className="h-3 w-16 rounded bg-zinc-200 dark:bg-zinc-800" key={item} />
          ))}
        </div>
      </div>
      <div className="h-80 p-5 sm:h-96">
        <div className="h-full rounded bg-zinc-950/[.035] dark:bg-white/5" />
      </div>
    </article>
  );
}

function SkillsSkeleton() {
  return (
    <>
      {["primary", "secondary"].map((role) => (
        <article
          className="min-h-[42rem] animate-pulse rounded-md border border-zinc-950/10 bg-white p-5 dark:border-white/10 dark:bg-zinc-900 sm:p-6"
          key={role}
        >
          <div className="h-6 w-48 rounded bg-zinc-200 dark:bg-zinc-800" />
          <div className="mt-5 aspect-square rounded bg-zinc-950/[.035] dark:bg-white/5" />
        </article>
      ))}
    </>
  );
}

export function CombatLabLoadouts({
  formationUsage,
  loadouts,
  primaryCommanderName,
  rangeKey,
  secondaryCommanderName,
}: {
  formationUsage: CombatLabPreviewFormationUsagePoint[];
  loadouts?: LoadoutData | null;
  primaryCommanderName: string;
  rangeKey: CombatLabPreviewRangeKey;
  secondaryCommanderName: string;
}) {
  const t = useExtracted();
  const locale = useLocale();
  const equipmentSlotNames: Record<number, string> = {
    1: getEquipmentSlotName(1, locale),
    2: getEquipmentSlotName(2, locale),
    3: getEquipmentSlotName(3, locale),
    4: getEquipmentSlotName(4, locale),
    5: getEquipmentSlotName(5, locale),
    6: getEquipmentSlotName(6, locale),
    7: getEquipmentSlotName(7, locale),
  };
  const armamentSlotNames: Record<number, string> = {
    1: t("Scroll"),
    2: t("Instrument"),
    3: t("Flag"),
    4: t("Emblem"),
  };
  const armamentSlots = Array.isArray(loadouts?.armaments?.slots) ? loadouts.armaments.slots : [];
  const equipmentSlots = Array.isArray(loadouts?.equipment?.slots) ? loadouts.equipment.slots : [];

  return (
    <section aria-labelledby="loadouts-title" className="scroll-mt-28">
      <Heading id="loadouts-title" level={2} className="mb-4">
        {t("Build breakdown")}
      </Heading>

      <div className="grid gap-5 xl:grid-cols-2">
        <CombatLabSkills
          primaryCommanderName={primaryCommanderName}
          rangeKey={rangeKey}
          secondaryCommanderName={secondaryCommanderName}
          skills={loadouts?.skills}
        />
        <CombatLabFormationChart points={formationUsage} rangeKey={rangeKey} />
        {armamentSlotOrder.map((slotId) => {
          const matchingSlot = armamentSlots.find((item) => item.slot === slotId);
          const slot = {
            slot: slotId,
            points: Array.isArray(matchingSlot?.points) ? matchingSlot.points : [],
          };
          return (
            <Panel key={slotId} title={t("Armament: {slot}", { slot: armamentSlotNames[slotId] })}>
              <CombatLabArmamentChart rangeKey={rangeKey} slot={slot} />
            </Panel>
          );
        })}
        {equipmentSlotOrder.map((slotId) => {
          const matchingSlot = equipmentSlots.find((item) => item.slot === slotId);
          const slot = {
            slot: slotId,
            points: Array.isArray(matchingSlot?.points) ? matchingSlot.points : [],
          };
          return (
            <Panel
              className={slotId === 7 ? "xl:col-span-2" : undefined}
              key={slotId}
              title={t("Equipment: {slot}", { slot: equipmentSlotNames[slotId] })}
            >
              <CombatLabEquipmentChart
                accessoryPairings={
                  slotId === 7 ? loadouts?.equipment?.accessoryPairings : undefined
                }
                rangeKey={rangeKey}
                slot={slot}
              />
            </Panel>
          );
        })}
      </div>
    </section>
  );
}

function Panel({
  children,
  className = "",
  title,
}: {
  children: React.ReactNode;
  className?: string;
  title: string;
}) {
  return (
    <article
      className={`min-w-0 rounded-md border border-zinc-950/10 bg-white p-5 dark:border-white/10 dark:bg-zinc-900 sm:p-6 ${className}`}
    >
      <Subheading level={3} className="!text-lg/7">
        {title}
      </Subheading>
      <div className="mt-3">{children}</div>
    </article>
  );
}
