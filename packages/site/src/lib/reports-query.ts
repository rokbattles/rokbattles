import type {
  ReportsFilterSide,
  ReportsFilterSubtype,
  ReportsFilterType,
  ReportsGarrisonBuildingType,
} from "@/providers/reports-filter-context";

export type ReportsQueryParams = {
  after?: string;
  before?: string;
  playerId?: number;
  type?: ReportsFilterType;
  subtype?: ReportsFilterSubtype;
  senderPrimaryCommanderId?: number;
  senderSecondaryCommanderId?: number;
  opponentPrimaryCommanderId?: number;
  opponentSecondaryCommanderId?: number;
  rallySide: ReportsFilterSide;
  garrisonSide: ReportsFilterSide;
  garrisonBuildingType?: ReportsGarrisonBuildingType;
};

export function buildReportsQueryParams({
  after,
  before,
  playerId,
  type,
  subtype,
  senderPrimaryCommanderId,
  senderSecondaryCommanderId,
  opponentPrimaryCommanderId,
  opponentSecondaryCommanderId,
  rallySide,
  garrisonSide,
  garrisonBuildingType,
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
  if (type) params.set("type", type);
  if (subtype && (type === "kvk" || type === "ark")) params.set("subtype", subtype);
  if (typeof senderPrimaryCommanderId === "number" && Number.isFinite(senderPrimaryCommanderId)) {
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
