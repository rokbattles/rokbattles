import { isTauri } from "@tauri-apps/api/core";
import type { ReactNode } from "react";
import { Navigate, useLocation } from "react-router";

export function TauriRouteGuard({ children }: { children: ReactNode }) {
  const { pathname, search, hash } = useLocation();

  if (isTauri() && pathname !== "/app" && !pathname.startsWith("/app/")) {
    return <Navigate to={{ pathname: "/app", search, hash }} replace />;
  }

  return children;
}
