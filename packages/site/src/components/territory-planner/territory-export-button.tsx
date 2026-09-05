"use client";

import { ArrowDownTrayIcon } from "@heroicons/react/20/solid";
import { useExtracted } from "next-intl";
import { useState } from "react";
import { useTerritoryPlannerLabels } from "@/components/territory-planner/use-territory-planner-labels";
import { Button } from "@/components/ui/button";
import type { PlannerDocument } from "@/lib/territory/types";

export function TerritoryExportButton({ document }: { document: PlannerDocument }) {
  const t = useExtracted();
  const { toolLabel } = useTerritoryPlannerLabels();
  const [error, setError] = useState("");
  const [isExporting, setIsExporting] = useState(false);

  async function exportPlan() {
    setIsExporting(true);
    setError("");
    try {
      const { exportTerritoryPlan } = await import("@/lib/territory/export");
      await exportTerritoryPlan(document, {
        mainFortress: toolLabel("mainFortress"),
        subFortress: toolLabel("subFortress"),
        horse: toolLabel("horse"),
        flag: toolLabel("flag"),
      });
    } catch {
      setError(t("The plan could not be exported. Please try again."));
    } finally {
      setIsExporting(false);
    }
  }

  return (
    <>
      <Button className="rounded-md" disabled={isExporting} onClick={exportPlan}>
        <ArrowDownTrayIcon /> {t("Export")}
      </Button>
      {error ? <p role="alert">{error}</p> : null}
    </>
  );
}
