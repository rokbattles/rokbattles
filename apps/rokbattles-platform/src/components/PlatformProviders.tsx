"use client";

import type React from "react";
import { ReportsFilterProvider } from "@/components/context/ReportsFilterContext";

export default function PlatformProviders({ children }: { children: React.ReactNode }) {
  return <ReportsFilterProvider>{children}</ReportsFilterProvider>;
}
