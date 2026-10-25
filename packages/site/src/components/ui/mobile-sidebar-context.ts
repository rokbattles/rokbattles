"use client";

import { createContext } from "react";

export const MobileSidebarCloseContext = createContext<(() => void) | null>(null);
