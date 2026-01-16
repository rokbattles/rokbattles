"use client";

import { ThemeProvider } from "next-themes";
import { NuqsAdapter } from "nuqs/adapters/next/app";
import type React from "react";
import type { ClaimedGovernor } from "@/lib/types/current-user";
import { GovernorProvider } from "@/providers/governor-context";
import { ReportsFilterProvider } from "@/providers/reports-filter-context";

interface PlatformProvidersProps {
  children: React.ReactNode;
  initialGovernors?: ClaimedGovernor[];
  initialActiveGovernorId?: number;
}

export default function PlatformProviders({
  children,
  initialGovernors,
  initialActiveGovernorId,
}: PlatformProvidersProps) {
  return (
    <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
      <NuqsAdapter>
        <GovernorProvider
          initialActiveGovernorId={initialActiveGovernorId}
          initialGovernors={initialGovernors}
        >
          <ReportsFilterProvider>{children}</ReportsFilterProvider>
        </GovernorProvider>
      </NuqsAdapter>
    </ThemeProvider>
  );
}
