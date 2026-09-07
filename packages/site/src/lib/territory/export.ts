import type { Cell, SheetData } from "write-excel-file/browser";
import { realToGamePoint } from "@/lib/territory/presentation";
import type { BuildingKind, PlannerDocument } from "@/lib/territory/types";

export function createTerritorySpreadsheet(
  document: PlannerDocument,
  labels: Record<BuildingKind, string>
) {
  const allianceColumns = document.alliances.map((alliance) => {
    const buildings = document.buildings.filter((building) => building.allianceId === alliance.id);
    const column: Cell[] = [{ type: String, value: alliance.name, fontWeight: "bold" }, null];

    for (const kind of ["mainFortress", "subFortress", "horse", "flag"] as const) {
      const placed = buildings.filter((building) => building.kind === kind);
      if (kind === "flag" && placed.length > 0 && column.length > 2) column.push(null);
      placed.forEach((building, index) => {
        const label =
          kind === "flag" || kind === "subFortress" ? `${labels[kind]} ${index + 1}` : labels[kind];
        const { x, y } = realToGamePoint(building);
        column.push({ type: String, value: `${label}: (${x}, ${y})` });
      });
    }

    return column;
  });

  const data: SheetData = Array.from(
    { length: Math.max(0, ...allianceColumns.map((column) => column.length)) },
    (_, row) =>
      allianceColumns.flatMap((column, index) =>
        index === 0 ? [column[row] ?? null] : [null, column[row] ?? null]
      )
  );
  const columns = document.alliances.flatMap((_, index) =>
    index === 0 ? [{ width: 40 }] : [{ width: 4 }, { width: 40 }]
  );

  return { data, columns };
}

export async function exportTerritoryPlan(
  document: PlannerDocument,
  labels: Record<BuildingKind, string>
) {
  const { default: writeExcelFile } = await import("write-excel-file/browser");
  const { data, columns } = createTerritorySpreadsheet(document, labels);
  await writeExcelFile(data, { columns }).toFile(`territory-plan-${document.mapSlug}.xlsx`);
}
