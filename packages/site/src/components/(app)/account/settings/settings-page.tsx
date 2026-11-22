import type { ReactNode } from "react";

export function SettingsPage({ children }: { children: ReactNode }) {
  return <div className="space-y-8 pt-4">{children}</div>;
}
