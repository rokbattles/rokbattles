import { useExtracted } from "next-intl";
import { Subheading } from "@/components/ui/heading";
import type { BaulurLootDocument } from "@/lib/loot-explorer/api";
import { LootTable } from "./loot-table";

export function BaulurRolls({
  pool,
  locale,
}: {
  pool: BaulurLootDocument["rollPool"];
  locale?: string;
}) {
  const t = useExtracted();

  return (
    <section className="space-y-8">
      <div>
        <Subheading>{t("Over 1% damage")}</Subheading>
        <div className="text-sm/6 text-zinc-500 dark:text-zinc-400">
          {t("This has been seen {count, plural, one {# time} other {# times}}.", {
            count: pool.matchedResults,
          })}
        </div>
      </div>
      {pool.unmatchedResults > 0 ? (
        <p className="rounded-lg bg-amber-50 px-4 py-3 text-sm/6 text-amber-900 dark:bg-amber-500/10 dark:text-amber-200">
          {t(
            "Rates exclude {count, plural, one {# reward result} other {# reward results}} that could not be assigned to the five slots.",
            { count: pool.unmatchedResults }
          )}
        </p>
      ) : null}
      {pool.matchedResults === 0 ? (
        <p className="text-sm/6 text-zinc-500 dark:text-zinc-400">
          {t("No matching reward results have been observed yet.")}
        </p>
      ) : null}
      <div className="space-y-8">
        {pool.slots.map((slot) => (
          <section key={slot.slot} className="space-y-3">
            <Subheading level={3}>{t("Slot {slot}", { slot: slot.slot.toString() })}</Subheading>
            <LootTable
              loot={slot.loot}
              locale={locale}
              noItem={
                slot.slot > 2 || slot.noItemResults > 0
                  ? { dropRate: slot.noItemRate, results: slot.noItemResults }
                  : undefined
              }
            />
          </section>
        ))}
      </div>
    </section>
  );
}
