"use client";

import { useExtracted } from "next-intl";
import { mapDisplayTitle } from "@/lib/territory/presentation";
import type { PlannerTool, ResourceKind, TerritoryMapIndexRow } from "@/lib/territory/types";

export function useTerritoryPlannerLabels() {
  const t = useExtracted();
  const resourceLabels: Record<ResourceKind, string> = {
    food: t("Food"),
    wood: t("Wood"),
    stone: t("Stone"),
    coin: t("Gold"),
    crystal: t("Crystal"),
  };
  const toolLabels: Record<PlannerTool, string> = {
    select: t("Select"),
    draw: t("Draw"),
    flag: t("Flag"),
    mainFortress: t("Center Fortress"),
    subFortress: t("Fortress"),
    horse: t("Horse"),
  };
  const mapLabels: Record<string, string> = {
    "preparation-season-01": t("Home 1"),
    "preparation-season-02": t("Home 2"),
    "preparation-season-03": t("Home 3"),
    "preparation-season-04": t("Home 4"),
    "s1-endless-war": t("The Endless War"),
    "s2-distant-journey": t("A Distant Journey"),
    "s3-light-and-darkness": t("Light and Darkness"),
    "s4-heroic-anthem": t("Heroic Anthem"),
    "s9-king-of-the-nile": t("King of the Nile"),
    "s10-siege-of-orleans": t("Siege of Orleans"),
    "s11-warriors-unbound": t("Warriors Unbound"),
    "s12-storm-of-stratagems": t("Storm of Stratagems"),
    "s14-tides-of-war": t("Tides of War"),
    "s15-alliance-invictus": t("Alliance Invictus"),
    "s16-keener-blades": t("Keener Blades"),
    "s19-king-of-all-britain": t("King of All Britain"),
    "s20-song-of-troy": t("Song of Troy"),
  };
  return {
    mapLabel: (map: Pick<TerritoryMapIndexRow, "slug" | "title">) =>
      mapLabels[map.slug] ?? mapDisplayTitle(map.title),
    resourceLabel: (kind: ResourceKind) => resourceLabels[kind],
    toolLabel: (kind: PlannerTool) => toolLabels[kind],
  };
}
