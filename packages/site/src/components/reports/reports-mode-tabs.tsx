"use client";

import { cn } from "cn";
import { use } from "react";
import { ReportsFilterContext, type ReportsFilterMode } from "@/providers/reports-filter-context";

const tabs: { value: ReportsFilterMode | undefined; label: string }[] = [
  { value: undefined, label: "All" },
  { value: "openfield", label: "Open Field" },
  { value: "rally", label: "Rally" },
  { value: "garrison", label: "Garrison" },
];

export function ReportsModeTabs() {
  const context = use(ReportsFilterContext);

  if (!context) {
    throw new Error("ReportsModeTabs must be used within a ReportsFilterProvider");
  }

  const { mode, setMode } = context;

  return (
    <div role="tablist" aria-label="Battle type" className="flex gap-1">
      {tabs.map((tab) => {
        const selected = tab.value === mode;
        return (
          <button
            key={tab.label}
            type="button"
            role="tab"
            aria-selected={selected}
            onClick={() => setMode(tab.value)}
            className={cn(
              "rounded-lg px-3 py-1.5 text-sm/6 font-medium",
              selected
                ? "bg-zinc-950/5 text-zinc-950 dark:bg-white/10 dark:text-white"
                : "text-zinc-500 hover:bg-zinc-950/5 hover:text-zinc-950 dark:text-zinc-400 dark:hover:bg-white/5 dark:hover:text-white"
            )}
          >
            {tab.label}
          </button>
        );
      })}
    </div>
  );
}
