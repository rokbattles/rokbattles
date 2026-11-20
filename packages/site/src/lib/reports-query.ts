import { commanderIdsWithTalent } from "@/hooks/use-commander-name";
import type {
  ReportsFilterSide,
  ReportsGarrisonBuildingType,
  ReportsTroopType,
} from "@/providers/reports-filter-context";

export type ReportsBattleKind = "openfield" | "rally" | "garrison";

export type ReportsQueryParams = {
  after?: string;
  before?: string;
  playerId?: number;
  receiverId?: number;
  troopType?: ReportsTroopType;
  tradeMin?: number;
  tradeMax?: number;
  commanderPrimaryId?: number;
  commanderSecondaryId?: number;
  rallySide: ReportsFilterSide;
  garrisonSide: ReportsFilterSide;
  garrisonBuildingType?: ReportsGarrisonBuildingType;
  kind?: ReportsBattleKind;
};

export function buildReportsQueryParams({
  after,
  before,
  playerId,
  receiverId,
  troopType,
  tradeMin,
  tradeMax,
  commanderPrimaryId,
  commanderSecondaryId,
  rallySide,
  garrisonSide,
  garrisonBuildingType,
  kind,
}: ReportsQueryParams) {
  const params = new URLSearchParams();

  if (before) {
    params.set("before", before);
  } else if (after) {
    params.set("after", after);
  }
  if (typeof playerId === "number" && Number.isFinite(playerId)) {
    params.set("pid", String(playerId));
  }
  if (typeof receiverId === "number" && Number.isFinite(receiverId)) {
    params.set("receiver", String(receiverId));
  }
  if (troopType) params.set("cps", commanderIdsWithTalent(troopType).join(","));
  if (typeof tradeMin === "number" && Number.isFinite(tradeMin)) {
    params.set("tmin", String(tradeMin));
  }
  if (typeof tradeMax === "number" && Number.isFinite(tradeMax)) {
    params.set("tmax", String(tradeMax));
  }
  if (typeof commanderPrimaryId === "number" && Number.isFinite(commanderPrimaryId)) {
    params.set("cpc", String(commanderPrimaryId));
  }
  if (typeof commanderSecondaryId === "number" && Number.isFinite(commanderSecondaryId)) {
    params.set("csc", String(commanderSecondaryId));
  }
  // The rally and garrison pages are the same as filtering to either side.
  const effectiveRallySide = kind === "rally" ? "both" : rallySide;
  const effectiveGarrisonSide = kind === "garrison" ? "both" : garrisonSide;
  if (effectiveRallySide !== "none") {
    params.set("rs", effectiveRallySide);
  }
  if (effectiveGarrisonSide !== "none") {
    params.set("gs", effectiveGarrisonSide);
    if (garrisonBuildingType) {
      params.set("gb", garrisonBuildingType);
    }
  }

  if (kind === "openfield") {
    params.set("of", "1");
  }

  const query = params.toString();
  return query ? `?${query}` : "";
}
