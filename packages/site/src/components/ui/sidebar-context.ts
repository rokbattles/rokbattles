"use client";

import { createContext } from "react";

export const SidebarContext = createContext<{
  collapsed: boolean;
  toggle: () => void;
} | null>(null);
