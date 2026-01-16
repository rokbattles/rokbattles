import type {
  ReportsFilterSide,
  ReportsFilterType,
  ReportsGarrisonBuildingType,
} from "@/providers/reports-filter-context";

export type ReportsQueryParams = {
  cursor?: string;
  playerId?: number;
  type?: ReportsFilterType;
  senderPrimaryCommanderId?: number;
  senderSecondaryCommanderId?: number;
  opponentPrimaryCommanderId?: number;
  opponentSecondaryCommanderId?: number;
  rallySide: ReportsFilterSide;
  garrisonSide: ReportsFilterSide;
  garrisonBuildingType?: ReportsGarrisonBuildingType;
};

export function buildReportsQueryParams({
  cursor,
  playerId,
  type,
  senderPrimaryCommanderId,
  senderSecondaryCommanderId,
  opponentPrimaryCommanderId,
  opponentSecondaryCommanderId,
  rallySide,
  garrisonSide,
  garrisonBuildingType,
}: ReportsQueryParams) {
  const params = new URLSearchParams();

  if (cursor) params.set("cursor", cursor);
  if (typeof playerId === "number" && Number.isFinite(playerId)) {
    params.set("pid", String(playerId));
  }
  if (type) params.set("type", type);
  if (
    typeof senderPrimaryCommanderId === "number" &&
    Number.isFinite(senderPrimaryCommanderId)
  ) {
    params.set("spc", String(senderPrimaryCommanderId));
  }
  if (
    typeof senderSecondaryCommanderId === "number" &&
    Number.isFinite(senderSecondaryCommanderId)
  ) {
    params.set("ssc", String(senderSecondaryCommanderId));
  }
  if (
    typeof opponentPrimaryCommanderId === "number" &&
    Number.isFinite(opponentPrimaryCommanderId)
  ) {
    params.set("opc", String(opponentPrimaryCommanderId));
  }
  if (
    typeof opponentSecondaryCommanderId === "number" &&
    Number.isFinite(opponentSecondaryCommanderId)
  ) {
    params.set("osc", String(opponentSecondaryCommanderId));
  }
  if (rallySide !== "none") {
    params.set("rs", rallySide);
  }
  if (garrisonSide !== "none") {
    params.set("gs", garrisonSide);
    if (garrisonBuildingType) {
      params.set("gb", garrisonBuildingType);
    }
  }

  const query = params.toString();
  return query ? `?${query}` : "";
}
