"use client";

import type React from "react";
import { GovernorProvider } from "@/components/context/GovernorContext";
import { ReportsFilterProvider } from "@/components/context/ReportsFilterContext";
import type { ClaimedGovernor } from "@/lib/types/current-user";

type PlatformProvidersProps = {
  children: React.ReactNode;
  initialGovernors?: ClaimedGovernor[];
  initialActiveGovernorId?: number;
};

export default function PlatformProviders({
  children,
  initialGovernors,
  initialActiveGovernorId,
}: PlatformProvidersProps) {
  return (
    <GovernorProvider
      initialGovernors={initialGovernors}
      initialActiveGovernorId={initialActiveGovernorId}
    >
      <ReportsFilterProvider>{children}</ReportsFilterProvider>
    </GovernorProvider>
  );
}
