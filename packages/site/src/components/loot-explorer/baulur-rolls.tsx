import { Subheading } from "@/components/ui/heading";
import type { BaulurLootDocument } from "@/lib/loot-explorer/api";
import { LootTable } from "./loot-table";

const numberFormatter = new Intl.NumberFormat("en-US");

export function BaulurRolls({ pool }: { pool: BaulurLootDocument["rollPool"] }) {
  return (
    <section className="space-y-8">
      <div>
        <Subheading>Over 1% damage</Subheading>
        <div className="text-sm/6 text-zinc-400">
          {`This has been seen ${
            Math.abs(pool.matchedResults) === 1
              ? `${numberFormatter.format(pool.matchedResults)} time`
              : `${numberFormatter.format(pool.matchedResults)} times`
          }.`}
        </div>
      </div>
      {pool.unmatchedResults > 0 ? (
        <p className="rounded-lg px-4 py-3 text-sm/6 bg-amber-500/10 text-amber-200">
          {`Rates exclude ${
            Math.abs(pool.unmatchedResults) === 1
              ? `${numberFormatter.format(pool.unmatchedResults)} reward result`
              : `${numberFormatter.format(pool.unmatchedResults)} reward results`
          } that could not be assigned to the five slots.`}
        </p>
      ) : null}
      {pool.matchedResults === 0 ? (
        <p className="text-sm/6 text-zinc-400">
          No matching reward results have been observed yet.
        </p>
      ) : null}
      <div className="space-y-8">
        {pool.slots.map((slot) => (
          <section key={slot.slot} className="space-y-3">
            <Subheading level={3}>{`Slot ${slot.slot.toString()}`}</Subheading>
            <LootTable
              loot={slot.loot}
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
