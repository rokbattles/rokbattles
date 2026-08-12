"use client";

import { useExtracted, useLocale } from "next-intl";
import { useEffect, useState } from "react";
import { Subheading } from "@/components/ui/heading";
import { GameTranslate } from "@/components/v1/game-translate";
import { fetchKaharTreasureLoot, type KaharTreasureLootDocument } from "@/lib/loot-explorer/api";
import { LootExplorerLayout } from "./loot-explorer-layout";
import { LootExplorerStatus } from "./loot-explorer-status";
import { LootExplorerSummary } from "./loot-explorer-summary";
import { LootTable } from "./loot-table";

export function KaharTreasureExplorer() {
  const t = useExtracted();
  const locale = useLocale();
  const [item, setItem] = useState<KaharTreasureLootDocument | null>(null);
  const [status, setStatus] = useState<"loading" | "ready" | "error">("loading");

  useEffect(() => {
    let ignore = false;

    setStatus("loading");
    fetchKaharTreasureLoot()
      .then((response) => {
        if (ignore) {
          return;
        }
        setItem(response);
        setStatus("ready");
      })
      .catch(() => {
        if (!ignore) {
          setStatus("error");
        }
      });

    return () => {
      ignore = true;
    };
  }, []);

  if (status === "loading") {
    return (
      <LootExplorerStatus active="kahars-treasure" message={t("Loading loot explorer data...")} />
    );
  }

  if (status === "error") {
    return (
      <LootExplorerStatus
        active="kahars-treasure"
        message={t("Failed to load loot explorer data.")}
      />
    );
  }

  return (
    <LootExplorerLayout active="kahars-treasure">
      <LootExplorerSummary
        generatedAt={item?.refreshedAt}
        items={[
          { label: t("Results"), value: item?.totals.results ?? 0 },
          { label: t("AP used"), value: item?.totals.apUsed ?? 0 },
        ]}
      />
      <section className="space-y-3">
        <div>
          <Subheading>
            <GameTranslate value="LC_KINGDOMWAR_BARBARIAN_TITLE" />
          </Subheading>
        </div>
        <LootTable loot={item?.loot ?? []} locale={locale} />
      </section>
    </LootExplorerLayout>
  );
}
