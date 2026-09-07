import { TerritoryPlanner } from "@/components/territory-planner/territory-planner";
import type { TerritoryMapListResponse } from "@/lib/territory/types";

export default async function TerritoryPlannerPage() {
  const apiUrl = process.env.API_URL || "http://localhost:8001";
  const response = await fetch(`${apiUrl.replace(/\/$/, "")}/v1/global/territory-planner/list`, {
    cache: "no-store",
  });
  if (!response.ok) {
    throw new Error(`Failed to load territory maps (${response.status})`);
  }
  const catalog = (await response.json()) as TerritoryMapListResponse;
  return <TerritoryPlanner maps={catalog.maps} />;
}
