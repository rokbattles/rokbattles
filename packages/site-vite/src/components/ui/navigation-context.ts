import { createContext } from "react";

export const NavigationCloseContext = createContext<(() => void) | null>(null);
