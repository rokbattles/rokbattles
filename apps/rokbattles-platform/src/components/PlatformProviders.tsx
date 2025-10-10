"use client";

import type React from "react";
import { GovernorProvider } from "@/components/context/GovernorContext";
import { ReportsFilterProvider } from "@/components/context/ReportsFilterContext";

export default function PlatformProviders({ children }: { children: React.ReactNode }) {
  return (
    <GovernorProvider>
      <ReportsFilterProvider>{children}</ReportsFilterProvider>
    </GovernorProvider>
  );
}
