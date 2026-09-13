import type { Cell, SheetData } from "write-excel-file/browser";
import { createTerritorySpreadsheet } from "@/lib/territory/export";
import { realToGamePoint } from "@/lib/territory/presentation";
import { orderedRouteSites } from "./routes";
import { legacyBuilding } from "./territory";
import type { MapData, Plan, Site } from "./types";

function siteCoordinates(site: Site): string {
  const { x, y } = realToGamePoint(site);
  const name = site.label ?? (site.kind === "barbarian-keep" ? "Keep" : "Camp");
  return `${name}: (${x}, ${y})`;
}

/** Match the existing planner's alliance columns; Baulur columns follow route order. */
export function createPlannerSpreadsheet(plan: Plan, data: MapData) {
  if (plan.mode === "territory")
    return createTerritorySpreadsheet(
      {
        version: 2,
        mapSlug: plan.map,
        activeAllianceId: plan.alliances[0]?.id ?? "",
        alliances: plan.alliances,
        buildings: plan.buildings.map(legacyBuilding),
        drawings: [],
      },
      {
        mainFortress: "Center fortress",
        subFortress: "Alliance fortress",
        horse: "Alliance horse",
        flag: "Flag",
      }
    );

  const sites = new Map(data.sites.map((site) => [site.id, site]));
  const alliances = new Map(plan.alliances.map((alliance) => [alliance.id, alliance.name]));
  const routeColumns = plan.routes.map((route) => {
    const column: Cell[] = [{ type: String, value: route.name, fontWeight: "bold" }];
    if (route.commander) column.push({ type: String, value: route.commander });
    column.push(null);
    orderedRouteSites(route).forEach((id, index) => {
      const site = sites.get(id);
      if (site)
        column.push({
          type: String,
          value: `${index + 1}. ${index === 0 ? "Start · " : ""}${siteCoordinates(site)}`,
        });
    });
    if (route.passes.length) {
      column.push(null, { type: String, value: "Passes", fontWeight: "bold" });
      for (const pass of route.passes) {
        const site = sites.get(pass.siteId);
        if (site)
          column.push({
            type: String,
            value: `${alliances.get(pass.allianceId) ?? ""} · ${siteCoordinates(site)}`,
          });
      }
    }
    return column;
  });
  const rows: SheetData = Array.from(
    { length: Math.max(0, ...routeColumns.map((column) => column.length)) },
    (_, row) =>
      routeColumns.flatMap((column, index) =>
        index === 0 ? [column[row] ?? null] : [null, column[row] ?? null]
      )
  );
  const columns = routeColumns.flatMap((_, index) =>
    index === 0 ? [{ width: 40 }] : [{ width: 4 }, { width: 40 }]
  );
  return { data: rows, columns };
}

export async function exportPlannerSpreadsheet(plan: Plan, data: MapData) {
  const { default: writeExcelFile } = await import("write-excel-file/browser");
  const sheet = createPlannerSpreadsheet(plan, data);
  await writeExcelFile(sheet.data, { columns: sheet.columns }).toFile(
    `${plan.mode}-plan-${plan.map}.xlsx`
  );
}
