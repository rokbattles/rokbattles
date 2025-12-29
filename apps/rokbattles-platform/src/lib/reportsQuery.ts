import type { ReportsFilterType } from "@/components/context/ReportsFilterContext";

export type ReportsQueryParams = {
  cursor?: string;
  playerId?: number;
  type?: ReportsFilterType;
  rallyOnly: boolean;
  primaryCommanderId?: number;
  secondaryCommanderId?: number;
};

export function buildReportsQueryParams({
  cursor,
  playerId,
  type,
  rallyOnly,
  primaryCommanderId,
  secondaryCommanderId,
}: ReportsQueryParams) {
  const params = new URLSearchParams();

  if (cursor) params.set("cursor", cursor);
  if (typeof playerId === "number" && Number.isFinite(playerId)) {
    params.set("playerId", String(playerId));
  }
  if (type) params.set("type", type);
  if (rallyOnly) params.set("rallyOnly", "1");
  if (typeof primaryCommanderId === "number" && Number.isFinite(primaryCommanderId)) {
    params.set("primaryCommanderId", String(primaryCommanderId));
  }
  if (typeof secondaryCommanderId === "number" && Number.isFinite(secondaryCommanderId)) {
    params.set("secondaryCommanderId", String(secondaryCommanderId));
  }

  const query = params.toString();
  return query ? `?${query}` : "";
}
