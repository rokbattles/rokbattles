import type {
  ReportsFilterSide,
  ReportsFilterSubtype,
  ReportsFilterType,
  ReportsGarrisonBuildingType,
} from "@/providers/reports-filter-context";

export type ReportsBattleKind = "openfield" | "rally" | "garrison";

export type ReportsQueryParams = {
  after?: string;
  before?: string;
  playerId?: number;
  receiverId?: number;
  type?: ReportsFilterType;
  subtype?: ReportsFilterSubtype;
  senderPrimaryCommanderId?: number;
  senderSecondaryCommanderId?: number;
  opponentPrimaryCommanderId?: number;
  opponentSecondaryCommanderId?: number;
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
  type,
  subtype,
  senderPrimaryCommanderId,
  senderSecondaryCommanderId,
  opponentPrimaryCommanderId,
  opponentSecondaryCommanderId,
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
